use crate::{
    de::MIN_BSON_DOCUMENT_SIZE,
    extjson::models::TimestampBody,
    oid::ObjectId,
    raw::{serde::CowStr, RAW_ARRAY_NEWTYPE, RAW_DOCUMENT_NEWTYPE},
    spec::{BinarySubtype, ElementType},
    RawDocumentBuf,
};
use serde::{
    de::{DeserializeSeed, Error as SerdeError, MapAccess, SeqAccess, Visitor},
    Deserialize,
    Deserializer,
};
use serde_bytes::ByteBuf;
use std::{
    borrow::Cow,
    convert::{TryFrom, TryInto},
    fmt::Formatter,
};

struct SeededVisitor<'a> {
    buffer: &'a mut Vec<u8>,
    embedded: bool,
}

impl<'a> SeededVisitor<'a> {
    fn append_key(&mut self, key: &str) -> Result<(), String> {
        let key_bytes = key.as_bytes();
        if key_bytes.contains(&0) {
            return Err(format!("key contains interior null byte: {}", key));
        }

        self.buffer.extend_from_slice(key_bytes);
        self.buffer.push(0);

        Ok(())
    }

    fn append_string(&mut self, s: &str) -> Result<(), String> {
        let bytes = s.as_bytes();

        // Add 1 to account for null byte.
        self.append_length_bytes(bytes.len() + 1)?;
        self.buffer.extend_from_slice(bytes);
        self.buffer.push(0);

        Ok(())
    }

    fn append_length_bytes(&mut self, length: impl TryInto<i32>) -> Result<(), String> {
        let length_bytes = match length.try_into() {
            Ok(length) => length.to_le_bytes(),
            Err(_) => return Err("element exceeds maximum length".to_string()),
        };

        self.buffer.extend(length_bytes);
        Ok(())
    }

    fn append_binary(&mut self, bytes: &[u8], subtype: u8) -> Result<(), String> {
        self.append_length_bytes(bytes.len())?;
        self.buffer.push(subtype);
        self.buffer.extend(bytes);

        Ok(())
    }

    // Appends 1 byte to the buffer as a placeholder for an element type. This byte should be
    // overwritten by a call to append_element after the element has been written to the buffer.
    fn pad_element_type(&mut self) -> usize {
        let index = self.buffer.len();
        self.buffer.push(0);
        index
    }

    // Writes the given element_type at the given index, which should be obtained from
    // pad_element_type.
    fn write_element_type(&mut self, element_type: ElementType, index: usize) {
        self.buffer[index..index + 1].copy_from_slice(&[element_type as u8]);
    }

    // Appends 4 bytes to the buffer as a placeholder for the length of a document. These bytes
    // should be overridden by a call to finish_document after the data in the document has been
    // written.
    fn pad_document_length(&mut self) -> usize {
        let index = self.buffer.len();
        self.buffer.extend_from_slice(&[0; 4]);
        index
    }

    // Writes the length of a document at the given index, which should be obtained from
    // pad_document_length, and appends the final null byte of the document. Returns an error if the
    // size does not fit into an i32.
    fn finish_document(&mut self, index: usize) -> Result<(), String> {
        self.buffer.push(0);

        let length_bytes = match i32::try_from(self.buffer.len() - index) {
            Ok(length) => length.to_le_bytes(),
            Err(_) => return Err("size of map too large".to_string()),
        };

        self.buffer[index..index + 4].copy_from_slice(&length_bytes);

        Ok(())
    }
}

impl<'a, 'de> DeserializeSeed<'de> for SeededVisitor<'a> {
    type Value = ElementType;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_map(self)
    }
}

impl<'a, 'de> DeserializeSeed<'de> for &mut SeededVisitor<'a> {
    type Value = ElementType;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(SeededVisitor {
            buffer: self.buffer,
            embedded: self.embedded,
        })
    }
}

impl<'a, 'de> Visitor<'de> for SeededVisitor<'a> {
    type Value = ElementType;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        // todo: improve these messages
        if self.embedded {
            formatter.write_str("map")
        } else {
            formatter.write_str("BSON value")
        }
    }

    fn visit_map<A>(mut self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        if !self.embedded {
            self.embedded = true;
        }

        let first_key = match map.next_key::<&str>()? {
            Some(key) => key,
            None => {
                self.buffer.extend(MIN_BSON_DOCUMENT_SIZE.to_le_bytes());
                self.buffer.push(0);
                return Ok(ElementType::EmbeddedDocument);
            }
        };

        match first_key {
            "$oid" => {
                let oid: ObjectId = map.next_value()?;
                self.buffer.extend(oid.bytes());
                Ok(ElementType::ObjectId)
            }
            "$symbol" => {
                let s: CowStr = map.next_value()?;
                self.append_string(s.0.as_ref())
                    .map_err(SerdeError::custom)?;
                Ok(ElementType::Symbol)
            }
            "$numberDecimalBytes" => {
                let bytes: ByteBuf = map.next_value()?;
                self.buffer.extend(&bytes.into_vec());
                Ok(ElementType::Decimal128)
            }
            "$regularExpression" => {
                #[derive(Deserialize)]
                struct Regex<'a> {
                    #[serde(borrow)]
                    pattern: CowStr<'a>,
                    #[serde(borrow)]
                    options: CowStr<'a>,
                }

                let regex: Regex = map.next_value()?;
                let pattern = regex.pattern.0.as_ref();
                let options = regex.options.0.as_ref();

                if pattern.contains('/') || options.contains('/') {
                    return Err(SerdeError::custom(format!(
                        "regular expression cannot contain unescaped forward slashes:\n pattern: \
                         {}\noptions:{}",
                        pattern, options
                    )));
                }

                self.append_key(pattern).map_err(SerdeError::custom)?;
                self.append_key(options).map_err(SerdeError::custom)?;

                Ok(ElementType::RegularExpression)
            }
            "$undefined" => {
                let _: bool = map.next_value()?;
                Ok(ElementType::Undefined)
            }
            "$binary" => {
                #[derive(Deserialize)]
                struct BorrowedBinary<'a> {
                    #[serde(borrow)]
                    bytes: Cow<'a, [u8]>,

                    #[serde(rename = "subType")]
                    subtype: u8,
                }

                let binary: BorrowedBinary = map.next_value()?;
                self.append_binary(binary.bytes.as_ref(), binary.subtype)
                    .map_err(SerdeError::custom)?;

                Ok(ElementType::Binary)
            }
            "$date" => {
                let date: i64 = map.next_value()?;
                self.buffer.extend(date.to_le_bytes());
                Ok(ElementType::DateTime)
            }
            "$timestamp" => {
                let timestamp: TimestampBody = map.next_value()?;
                self.buffer.extend(timestamp.i.to_le_bytes());
                self.buffer.extend(timestamp.t.to_le_bytes());
                Ok(ElementType::Timestamp)
            }
            "$minKey" => {
                let _: i32 = map.next_value()?;
                Ok(ElementType::MinKey)
            }
            "$maxKey" => {
                let _: i32 = map.next_value()?;
                Ok(ElementType::MaxKey)
            }
            "$code" => {
                let code: CowStr = map.next_value()?;
                if let Some(key) = map.next_key::<CowStr>()? {
                    let key = key.0.as_ref();
                    if key == "$scope" {
                        let length_index = self.pad_document_length();
                        self.append_string(code.0.as_ref())
                            .map_err(SerdeError::custom)?;

                        let scope: RawDocumentBuf = map.next_value()?;
                        self.buffer.extend(scope.as_bytes());

                        let length_bytes =
                            ((self.buffer.len() - length_index) as i32).to_le_bytes();
                        self.buffer[length_index..length_index + 4].copy_from_slice(&length_bytes);

                        Ok(ElementType::JavaScriptCodeWithScope)
                    } else {
                        Err(SerdeError::unknown_field(key, &["$scope"]))
                    }
                } else {
                    self.append_string(code.0.as_ref())
                        .map_err(SerdeError::custom)?;
                    Ok(ElementType::JavaScriptCode)
                }
            }
            "$dbPointer" => {
                #[derive(Deserialize)]
                struct BorrowedDbPointer<'a> {
                    #[serde(rename = "$ref")]
                    #[serde(borrow)]
                    ns: CowStr<'a>,

                    #[serde(rename = "$id")]
                    id: ObjectId,
                }

                let db_pointer: BorrowedDbPointer = map.next_value()?;

                self.append_string(db_pointer.ns.0.as_ref())
                    .map_err(SerdeError::custom)?;
                self.buffer.extend(db_pointer.id.bytes());

                Ok(ElementType::DbPointer)
            }
            RAW_DOCUMENT_NEWTYPE => {
                let document_bytes: ByteBuf = map.next_value()?;
                self.buffer.extend(document_bytes.as_ref());
                Ok(ElementType::EmbeddedDocument)
            }
            RAW_ARRAY_NEWTYPE => {
                let array_bytes: ByteBuf = map.next_value()?;
                self.buffer.extend(array_bytes.as_ref());
                Ok(ElementType::Array)
            }
            other => {
                let length_index = self.pad_document_length();

                let mut current_key = other;
                loop {
                    let element_type_index = self.pad_element_type();
                    self.append_key(current_key).map_err(SerdeError::custom)?;
                    let element_type = map.next_value_seed(&mut self)?;
                    self.write_element_type(element_type, element_type_index);

                    match map.next_key::<&str>()? {
                        Some(next_key) => current_key = next_key,
                        None => break,
                    }
                }

                self.finish_document(length_index)
                    .map_err(SerdeError::custom)?;
                Ok(ElementType::EmbeddedDocument)
            }
        }
    }

    fn visit_seq<A>(mut self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let length_index = self.pad_document_length();

        let mut i = 0u32;
        loop {
            let element_type_index = self.pad_element_type();
            let key = i.to_string();
            self.append_key(&key).map_err(SerdeError::custom)?;

            let element_type = match seq.next_element_seed(&mut self)? {
                Some(element_type) => element_type,
                None => {
                    // Remove the additional key and padding for the element that was not present.
                    self.buffer.drain(element_type_index..self.buffer.len());
                    break;
                }
            };

            self.write_element_type(element_type, element_type_index);
            i += 1;
        }

        self.finish_document(length_index)
            .map_err(SerdeError::custom)?;
        Ok(ElementType::Array)
    }

    // visit_string and visit_borrowed_str will forward to this method.
    fn visit_str<E>(mut self, s: &str) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.append_string(s).map_err(SerdeError::custom)?;
        Ok(ElementType::String)
    }

    fn visit_bool<E>(self, b: bool) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.push(b as u8);
        Ok(ElementType::Boolean)
    }

    fn visit_i8<E>(self, n: i8) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.extend_from_slice(&(n as i32).to_le_bytes());
        Ok(ElementType::Int32)
    }

    fn visit_i16<E>(self, n: i16) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.extend_from_slice(&(n as i32).to_le_bytes());
        Ok(ElementType::Int32)
    }

    fn visit_i32<E>(self, n: i32) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.extend_from_slice(&n.to_le_bytes());
        Ok(ElementType::Int32)
    }

    fn visit_i64<E>(self, n: i64) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.extend_from_slice(&n.to_le_bytes());
        Ok(ElementType::Int64)
    }

    fn visit_u8<E>(self, n: u8) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.extend_from_slice(&(n as i32).to_le_bytes());
        Ok(ElementType::Int32)
    }

    fn visit_u16<E>(self, n: u16) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.extend_from_slice(&(n as i32).to_le_bytes());
        Ok(ElementType::Int32)
    }

    fn visit_u32<E>(self, n: u32) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        match i32::try_from(n) {
            Ok(n) => {
                self.buffer.extend_from_slice(&n.to_le_bytes());
                Ok(ElementType::Int32)
            }
            Err(_) => {
                self.buffer.extend_from_slice(&(n as i64).to_le_bytes());
                Ok(ElementType::Int64)
            }
        }
    }

    fn visit_u64<E>(self, n: u64) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        if let Ok(n) = i32::try_from(n) {
            self.buffer.extend_from_slice(&n.to_le_bytes());
            Ok(ElementType::Int32)
        } else if let Ok(n) = i64::try_from(n) {
            self.buffer.extend_from_slice(&n.to_le_bytes());
            Ok(ElementType::Int64)
        } else {
            Err(SerdeError::custom(format!(
                "number is too large for BSON: {}",
                n
            )))
        }
    }

    fn visit_f64<E>(self, n: f64) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.extend_from_slice(&n.to_le_bytes());
        Ok(ElementType::Double)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.push(0);
        Ok(ElementType::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.push(0);
        Ok(ElementType::Null)
    }

    // visit_byte_buf and visit_borrowed_bytes will forward to this method.
    fn visit_bytes<E>(mut self, bytes: &[u8]) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.append_binary(bytes, BinarySubtype::Generic.into())
            .map_err(SerdeError::custom)?;
        Ok(ElementType::Binary)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        raw::error::{Error, ErrorKind, Result},
        Document,
        RawDocumentBuf,
    };

    use super::*;

    fn json_to_doc(json: &str) -> Result<Document> {
        let mut buffer = Vec::new();
        let visitor = SeededVisitor {
            buffer: &mut buffer,
            embedded: false,
        };
        let mut deserializer = serde_json::Deserializer::from_str(json);

        visitor.deserialize(&mut deserializer).map_err(|e| {
            Error::new_without_key(ErrorKind::MalformedValue {
                message: e.to_string(),
            })
        })?;

        let raw_document = RawDocumentBuf::from_bytes(buffer)?;
        raw_document.to_document()
    }

    #[test]
    fn basic_json() {
        assert_eq!(json_to_doc("{}"), Ok(doc! {}));
        assert_eq!(json_to_doc("{\"a\": \"B\"}"), Ok(doc! { "a": "B" }));
        assert!(json_to_doc("{\"a\"}").is_err());
        assert_eq!(
            json_to_doc("{\"a\":{\"b\":\"c\"}}"),
            Ok(doc! { "a": { "b": "c" } })
        );
        assert!(json_to_doc("a").is_err());
    }

    #[test]
    fn numbers() {
        assert_eq!(json_to_doc("{\"a\": 1}"), Ok(doc! { "a": 1 }));
        assert_eq!(
            json_to_doc("{\"a\": {\"1\": 1}}"),
            Ok(doc! {"a": { "1": 1 } })
        );
        assert!(json_to_doc("{1:1}").is_err());
        assert!(json_to_doc(&format!("{{\"a\":{}}}", u64::MAX)).is_err());
    }

    #[test]
    fn arrays() {
        assert_eq!(json_to_doc("{\"a\":[true]}"), Ok(doc! { "a": [true] }));
        assert_eq!(
            json_to_doc(
                "{\"a\":[\"b\", false, 12, -10, 2.1, {\"nested\": \"hi\"}, [\"sub\",
                       \"array\"]]}"
            ),
            Ok(doc! {
                "a": [
                    "b",
                    false,
                    12,
                    -10i64,
                    2.1,
                    { "nested": "hi" },
                    ["sub", "array"],
                ]
            })
        );
    }
}
