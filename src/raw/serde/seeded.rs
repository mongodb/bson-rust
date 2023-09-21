use crate::spec::ElementType;
use serde::{
    de::{DeserializeSeed, Error as SerdeError, MapAccess, SeqAccess, Visitor},
    Deserializer,
};
use std::{convert::TryFrom, fmt::Formatter};

struct SeededVisitor<'a> {
    buffer: &'a mut Vec<u8>,
    embedded: bool,
}

impl<'a> SeededVisitor<'a> {
    fn append_key(&mut self, key: &str) -> Result<(), String> {
        let key_bytes = key.as_bytes();
        if key_bytes.contains(&0) {
            return Err(format!("key includes interior null byte: {}", key));
        }

        self.buffer.extend_from_slice(key_bytes);
        self.buffer.push(0);

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

        let length_index = self.pad_document_length();

        while let Some(key) = map.next_key::<&str>()? {
            let element_type_index = self.pad_element_type();
            self.append_key(key).map_err(SerdeError::custom)?;
            let element_type = map.next_value_seed(&mut self)?;
            self.write_element_type(element_type, element_type_index);
        }

        self.finish_document(length_index)
            .map_err(SerdeError::custom)?;

        Ok(ElementType::EmbeddedDocument)
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
    fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        let bytes = s.as_bytes();

        // Add 1 to account for null byte.
        let length_bytes = match i32::try_from(bytes.len() + 1) {
            Ok(length) => length.to_le_bytes(),
            Err(_) => {
                return Err(SerdeError::custom(format!(
                    "string exceeds maximum length: {}",
                    s
                )))
            }
        };
        self.buffer.extend_from_slice(&length_bytes);

        self.buffer.extend_from_slice(bytes);
        self.buffer.push(0);

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
