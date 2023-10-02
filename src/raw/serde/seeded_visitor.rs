use std::{borrow::Cow, convert::TryFrom, fmt::Formatter, ops::Range};

use serde::{
    de::{DeserializeSeed, Error as SerdeError, MapAccess, SeqAccess, Visitor},
    Deserializer,
};
use serde_bytes::ByteBuf;

use crate::{
    de::MIN_BSON_DOCUMENT_SIZE,
    extjson::models::{
        BorrowedBinaryBody,
        BorrowedDbPointerBody,
        BorrowedRegexBody,
        TimestampBody,
    },
    oid::ObjectId,
    raw::{RAW_ARRAY_NEWTYPE, RAW_DOCUMENT_NEWTYPE},
    spec::{BinarySubtype, ElementType},
    RawDocumentBuf,
};

use super::CowStr;

/// A copy-on-write byte buffer containing raw BSON bytes. The inner value starts as `None` and
/// transitions to either `Cow::Borrowed` or `Cow::Owned` depending upon the data visited.
pub(crate) struct CowByteBuffer<'de>(pub(crate) Option<Cow<'de, [u8]>>);

impl<'de> CowByteBuffer<'de> {
    /// Creates an new empty buffer.
    pub(crate) fn new() -> Self {
        Self(None)
    }

    /// The length of the buffer.
    fn len(&self) -> usize {
        match &self.0 {
            Some(buffer) => buffer.len(),
            None => 0,
        }
    }

    /// Gets a mutable reference to the inner buffer, allocating a `Vec<u8>` and transitioning the
    /// buffer's state as necessary.
    fn get_owned_buffer(&mut self) -> &mut Vec<u8> {
        self.0
            .get_or_insert_with(|| Cow::Owned(Vec::new()))
            .to_mut()
    }

    /// Appends a single byte to the buffer.
    fn push_byte(&mut self, byte: u8) {
        let buffer = self.get_owned_buffer();
        buffer.push(byte);
    }

    /// Appends a slice of bytes to the buffer.
    fn append_bytes(&mut self, bytes: &[u8]) {
        let buffer = self.get_owned_buffer();
        buffer.extend_from_slice(bytes);
    }

    /// Appends a slice of borrowed bytes to the buffer. If the buffer is currently `None`, it will
    /// store a reference to the borrowed bytes; otherwise, it will copy the bytes to the
    /// existing buffer.
    fn append_borrowed_bytes(&mut self, bytes: &'de [u8]) {
        match &mut self.0 {
            Some(buffer) => buffer.to_mut().extend_from_slice(bytes),
            None => self.0 = Some(Cow::Borrowed(bytes)),
        }
    }

    /// Copies a slice of bytes into the given range. This method will panic if the range is out of
    /// bounds.
    fn copy_from_slice(&mut self, range: Range<usize>, slice: &[u8]) {
        let buffer = self.get_owned_buffer();
        buffer[range].copy_from_slice(slice);
    }

    /// Removes the bytes in the given range from the buffer. This method will panic if the range is
    /// out of bounds.
    fn drain(&mut self, range: Range<usize>) {
        let buffer = self.get_owned_buffer();
        buffer.drain(range);
    }
}

pub(crate) struct SeededVisitor<'a, 'de> {
    buffer: &'a mut CowByteBuffer<'de>,
}

impl<'a, 'de> DeserializeSeed<'de> for SeededVisitor<'a, 'de> {
    type Value = ElementType;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }
}

impl<'a, 'de> DeserializeSeed<'de> for &mut SeededVisitor<'a, 'de> {
    type Value = ElementType;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(SeededVisitor {
            buffer: self.buffer,
        })
    }
}

/// A visitor that builds up a raw BSON value in a single buffer. This visitor will only produce
/// valid BSON if the value being deserialized is a byte buffer, slice of bytes, map, or sequence.
/// Implementations using this visitor should check the `ElementType` returned from `deserialize` to
/// validate that a valid BSON type was visited.
impl<'a, 'de> SeededVisitor<'a, 'de> {
    pub(crate) fn new(buffer: &'a mut CowByteBuffer<'de>) -> Self {
        Self { buffer }
    }

    /// Appends a cstring to the buffer. Returns an error if the given string contains a null byte.
    fn append_cstring(&mut self, key: &str) -> Result<(), String> {
        let key_bytes = key.as_bytes();
        if key_bytes.contains(&0) {
            return Err(format!("key contains interior null byte: {}", key));
        }

        self.buffer.append_bytes(key_bytes);
        self.buffer.push_byte(0);

        Ok(())
    }

    /// Appends a string and its length to the buffer.
    fn append_string(&mut self, s: &str) {
        let bytes = s.as_bytes();

        // Add 1 to account for null byte.
        self.append_length_bytes((bytes.len() + 1) as i32);

        self.buffer.append_bytes(bytes);
        self.buffer.push_byte(0);
    }

    /// Converts the given length into little-endian bytes and appends the bytes to the buffer.
    fn append_length_bytes(&mut self, length: i32) {
        self.buffer.append_bytes(&length.to_le_bytes());
    }

    /// Appends an owned byte buffer to the buffer. If the buffer is currently empty (i.e. the byte
    /// buffer was the top-level value provided to the deserializer), the buffer will be updated to
    /// contain an owned copy of the bytes provided. Otherwise (i.e. when the value is embedded),
    /// the bytes and their corresponding length and subtype bytes will be appended to the
    /// buffer.
    fn append_owned_binary(&mut self, bytes: Vec<u8>, subtype: u8) {
        match &mut self.buffer.0 {
            Some(_) => self.append_embedded_binary(&bytes, subtype),
            None => self.buffer.0 = Some(Cow::Owned(bytes)),
        }
    }

    /// Appends a slice of bytes to the buffer. If the buffer is currently empty (i.e. the byte
    /// buffer was the top-level value provided to the deserializer), the buffer will be updated to
    /// contain a reference to the slice of bytes. Otherwise (i.e. when the value is embedded),
    /// the bytes and their corresponding length and subtype will be appended to the buffer.
    fn append_borrowed_binary(&mut self, bytes: &'de [u8], subtype: u8) {
        match &self.buffer.0 {
            Some(_) => self.append_embedded_binary(bytes, subtype),
            None => self.buffer.0 = Some(Cow::Borrowed(bytes)),
        }
    }

    /// Appends the given bytes and their corresponding length and subtype to the buffer.
    fn append_embedded_binary(&mut self, bytes: &[u8], subtype: impl Into<u8>) {
        self.append_length_bytes(bytes.len() as i32);
        self.buffer.push_byte(subtype.into());
        self.buffer.append_bytes(bytes);
    }

    /// Appends 1 byte to the buffer as a placeholder for an element type. This byte should be
    /// overwritten by a call to append_element after the element has been written to the buffer.
    fn pad_element_type(&mut self) -> usize {
        let index = self.buffer.len();
        self.buffer.push_byte(0);
        index
    }

    /// Writes the given element_type at the given index, which should be obtained from
    /// pad_element_type.
    fn write_element_type(&mut self, element_type: ElementType, index: usize) {
        self.buffer
            .copy_from_slice(index..index + 1, &[element_type as u8]);
    }

    /// Appends 4 bytes to the buffer as a placeholder for the length of a document. These bytes
    /// should be overwritten by a call to finish_document after the data in the document has been
    /// written.
    fn pad_document_length(&mut self) -> usize {
        let index = self.buffer.len();
        self.buffer.append_bytes(&[0; 4]);
        index
    }

    /// Writes the length of a document at the given index, which should be obtained from
    /// pad_document_length, and appends the final null byte of the document. Returns an error if
    /// the size does not fit into an i32.
    fn finish_document(&mut self, length_index: usize) -> Result<(), String> {
        self.buffer.push_byte(0);

        let length_bytes = match i32::try_from(self.buffer.len() - length_index) {
            Ok(length) => length.to_le_bytes(),
            Err(_) => return Err("value exceeds maximum length".to_string()),
        };

        self.buffer
            .copy_from_slice(length_index..length_index + 4, &length_bytes);

        Ok(())
    }

    /// Iterates the given `MapAccess` and appends its keys and values to the buffer. The given map
    /// must have had `next_key` called exactly once, and the value returned from that call must be
    /// provided as `first_key`. `next_value` must not have been called on the map.
    pub(crate) fn iterate_map<A>(mut self, first_key: CowStr, mut map: A) -> Result<(), A::Error>
    where
        A: MapAccess<'de>,
    {
        let length_index = self.pad_document_length();

        let mut current_key = first_key;
        loop {
            let element_type_index = self.pad_element_type();
            self.append_cstring(current_key.0.as_ref())
                .map_err(SerdeError::custom)?;
            let element_type = map.next_value_seed(&mut self)?;
            self.write_element_type(element_type, element_type_index);

            match map.next_key::<CowStr>()? {
                Some(next_key) => current_key = next_key,
                None => break,
            }
        }

        self.finish_document(length_index)
            .map_err(SerdeError::custom)?;
        Ok(())
    }
}

impl<'a, 'de> Visitor<'de> for SeededVisitor<'a, 'de> {
    type Value = ElementType;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("a raw BSON value")
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_map<A>(mut self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let first_key = match map.next_key::<CowStr>()? {
            Some(key) => key,
            None => {
                self.buffer
                    .append_bytes(&MIN_BSON_DOCUMENT_SIZE.to_le_bytes());
                self.buffer.push_byte(0);
                return Ok(ElementType::EmbeddedDocument);
            }
        };

        match first_key.0.as_ref() {
            "$oid" => {
                let oid: ObjectId = map.next_value()?;
                self.buffer.append_bytes(&oid.bytes());
                Ok(ElementType::ObjectId)
            }
            "$symbol" => {
                let s: &str = map.next_value()?;
                self.append_string(s);
                Ok(ElementType::Symbol)
            }
            "$numberDecimalBytes" => {
                let bytes: ByteBuf = map.next_value()?;
                self.buffer.append_bytes(&bytes.into_vec());
                Ok(ElementType::Decimal128)
            }
            "$regularExpression" => {
                let regex: BorrowedRegexBody = map.next_value()?;
                let pattern = regex.pattern.as_ref();
                let options = regex.options.as_ref();

                self.append_cstring(pattern).map_err(SerdeError::custom)?;
                self.append_cstring(options).map_err(SerdeError::custom)?;

                Ok(ElementType::RegularExpression)
            }
            "$undefined" => {
                let _: bool = map.next_value()?;
                Ok(ElementType::Undefined)
            }
            "$binary" => {
                let binary: BorrowedBinaryBody = map.next_value()?;
                match binary.bytes {
                    Cow::Borrowed(borrowed_bytes) => {
                        self.append_borrowed_binary(borrowed_bytes, binary.subtype);
                    }
                    Cow::Owned(owned_bytes) => {
                        self.append_owned_binary(owned_bytes, binary.subtype);
                    }
                }

                Ok(ElementType::Binary)
            }
            "$date" => {
                let date: i64 = map.next_value()?;
                self.buffer.append_bytes(&date.to_le_bytes());
                Ok(ElementType::DateTime)
            }
            "$timestamp" => {
                let timestamp: TimestampBody = map.next_value()?;
                self.buffer.append_bytes(&timestamp.i.to_le_bytes());
                self.buffer.append_bytes(&timestamp.t.to_le_bytes());
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
                        self.append_string(code.0.as_ref());

                        let scope: RawDocumentBuf = map.next_value()?;
                        self.buffer.append_bytes(scope.as_bytes());

                        let length_bytes =
                            ((self.buffer.len() - length_index) as i32).to_le_bytes();
                        self.buffer
                            .copy_from_slice(length_index..length_index + 4, &length_bytes);

                        Ok(ElementType::JavaScriptCodeWithScope)
                    } else {
                        Err(SerdeError::unknown_field(key, &["$scope"]))
                    }
                } else {
                    self.append_string(code.0.as_ref());
                    Ok(ElementType::JavaScriptCode)
                }
            }
            "$dbPointer" => {
                let db_pointer: BorrowedDbPointerBody = map.next_value()?;

                self.append_string(db_pointer.ns.0.as_ref());
                self.buffer.append_bytes(&db_pointer.id.bytes());

                Ok(ElementType::DbPointer)
            }
            RAW_DOCUMENT_NEWTYPE => {
                let document_bytes: &[u8] = map.next_value()?;
                self.buffer.append_borrowed_bytes(document_bytes);
                Ok(ElementType::EmbeddedDocument)
            }
            RAW_ARRAY_NEWTYPE => {
                let array_bytes: &[u8] = map.next_value()?;
                self.buffer.append_borrowed_bytes(array_bytes);
                Ok(ElementType::Array)
            }
            _ => {
                self.iterate_map(first_key, map)?;
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
            self.append_cstring(&key).map_err(SerdeError::custom)?;

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
        self.append_string(s);
        Ok(ElementType::String)
    }

    fn visit_bool<E>(self, b: bool) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.push_byte(b as u8);
        Ok(ElementType::Boolean)
    }

    fn visit_i8<E>(self, n: i8) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.append_bytes(&(n as i32).to_le_bytes());
        Ok(ElementType::Int32)
    }

    fn visit_i16<E>(self, n: i16) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.append_bytes(&(n as i32).to_le_bytes());
        Ok(ElementType::Int32)
    }

    fn visit_i32<E>(self, n: i32) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.append_bytes(&n.to_le_bytes());
        Ok(ElementType::Int32)
    }

    fn visit_i64<E>(self, n: i64) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.append_bytes(&n.to_le_bytes());
        Ok(ElementType::Int64)
    }

    fn visit_u8<E>(self, n: u8) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.append_bytes(&(n as i32).to_le_bytes());
        Ok(ElementType::Int32)
    }

    fn visit_u16<E>(self, n: u16) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.buffer.append_bytes(&(n as i32).to_le_bytes());
        Ok(ElementType::Int32)
    }

    fn visit_u32<E>(self, n: u32) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        match i32::try_from(n) {
            Ok(n) => {
                self.buffer.append_bytes(&n.to_le_bytes());
                Ok(ElementType::Int32)
            }
            Err(_) => {
                self.buffer.append_bytes(&(n as i64).to_le_bytes());
                Ok(ElementType::Int64)
            }
        }
    }

    fn visit_u64<E>(self, n: u64) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        if let Ok(n) = i32::try_from(n) {
            self.buffer.append_bytes(&n.to_le_bytes());
            Ok(ElementType::Int32)
        } else if let Ok(n) = i64::try_from(n) {
            self.buffer.append_bytes(&n.to_le_bytes());
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
        self.buffer.append_bytes(&n.to_le_bytes());
        Ok(ElementType::Double)
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(ElementType::Null)
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(ElementType::Null)
    }

    // visit_byte_buf will forward to this method.
    fn visit_bytes<E>(mut self, bytes: &[u8]) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.append_owned_binary(bytes.to_owned(), BinarySubtype::Generic.into());
        Ok(ElementType::Binary)
    }

    fn visit_borrowed_bytes<E>(mut self, bytes: &'de [u8]) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        self.append_borrowed_binary(bytes, BinarySubtype::Generic.into());
        Ok(ElementType::Binary)
    }
}
