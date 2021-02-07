use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::forward_to_deserialize_any;
use serde::Deserialize;

use std::convert::TryInto;
use std::fmt::Debug;
use std::num::TryFromIntError;

use crate::raw::{elem::Element, ArrayIter, Doc, DocBuf, DocIter, RawError};
use crate::spec::ElementType;

use object_id::RawObjectIdDeserializer;

pub mod binary;
pub mod datetime;
pub mod js;
pub mod object_id;
pub mod regex;

#[derive(Debug)]
pub enum Error {
    Eof,
    TrailingData(Vec<u8>),
    EncodingError,
    MalformedDocument,
    UnexpectedType,
    Unimplemented,
    IntConversion(TryFromIntError),
    Internal(String),
    NotFound,
    TmPErroR,
}

impl From<TryFromIntError> for Error {
    fn from(err: TryFromIntError) -> Error {
        Error::IntConversion(err)
    }
}

impl std::error::Error for Error {}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl de::Error for Error {
    fn custom<T: std::fmt::Display>(err: T) -> Error {
        Error::Internal(format!("{}", err))
    }
}

impl From<RawError> for Error {
    fn from(val: RawError) -> Error {
        match val {
            RawError::Utf8EncodingError(_) => Error::EncodingError,
            RawError::UnexpectedType => Error::UnexpectedType,
            RawError::MalformedValue(_) => Error::MalformedDocument,
        }
    }
}

pub struct BsonDeserializer<'de> {
    bson: Element<'de>,
}

impl<'de> BsonDeserializer<'de> {
    #[deprecated(since = "0.2.0", note = "use from_doc(&docref) instead")]
    pub fn from_docref(doc: &'de Doc) -> Self {
        BsonDeserializer::from_rawbson(Element::new(ElementType::EmbeddedDocument, doc.as_bytes()))
    }

    pub fn from_doc(doc: &'de Doc) -> Self {
        BsonDeserializer::from_rawbson(Element::new(ElementType::EmbeddedDocument, doc.as_bytes()))
    }

    pub fn from_rawbson(bson: Element<'de>) -> Self {
        BsonDeserializer { bson }
    }
}

#[deprecated(since = "0.2.0", note = "use from_doc(&docbuf) instead")]
pub fn from_docbuf<'de, T>(rawdoc_buf: &'de DocBuf) -> Result<T, Error>
where
    T: Deserialize<'de>,
{
    from_doc(rawdoc_buf)
}

pub fn from_doc<'de, T>(rawdoc: &'de Doc) -> Result<T, Error>
where
    T: Deserialize<'de>,
{
    let mut de = BsonDeserializer::from_doc(rawdoc);
    T::deserialize(&mut de)
}

#[deprecated(since = "0.2.0", note = "use from_doc(&docref) instead")]
pub fn from_docref<'de, T>(rawdoc: &'de Doc) -> Result<T, Error>
where
    T: Deserialize<'de>,
{
    from_doc(rawdoc)
}

pub fn from_bytes<'de, T: 'de>(data: &'de [u8]) -> Result<T, Error>
where
    T: Deserialize<'de>,
{
    let raw_document = Doc::new(data)?;
    from_doc(raw_document)
}

impl<'a, 'de: 'a> Deserializer<'de> for &'a mut BsonDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::Double => self.deserialize_f64(visitor),
            ElementType::String => self.deserialize_str(visitor),
            ElementType::EmbeddedDocument => self.deserialize_map(visitor),
            ElementType::Array => self.deserialize_seq(visitor),
            ElementType::Binary => self.deserialize_bytes(visitor),
            ElementType::Undefined => self.deserialize_unit(visitor),
            ElementType::ObjectId => {
                self.deserialize_struct(object_id::NAME, object_id::FIELDS, visitor)
            }
            ElementType::Boolean => self.deserialize_bool(visitor),
            ElementType::DateTime => {
                self.deserialize_struct(datetime::NAME, datetime::FIELDS, visitor)
            }
            ElementType::Null => self.deserialize_unit(visitor),
            ElementType::DbPointer => Err(Error::Unimplemented), // deserialize (&str, ObjectId), or struct
            ElementType::RegularExpression => {
                self.deserialize_struct(regex::NAME, regex::FIELDS, visitor)
            }
            ElementType::JavaScriptCode => self.deserialize_str(visitor),
            ElementType::Symbol => self.deserialize_str(visitor),
            ElementType::JavaScriptCodeWithScope => {
                self.deserialize_struct(js::WITH_SCOPE_NAME, js::WITH_SCOPE_FIELDS, visitor)
            } // deserialize (&'str, Map) or struct
            ElementType::Int32 => self.deserialize_i32(visitor),
            ElementType::Timestamp => self.deserialize_u64(visitor),
            ElementType::Int64 => self.deserialize_i64(visitor),
            ElementType::MinKey => self.deserialize_unit(visitor),
            ElementType::MaxKey => self.deserialize_unit(visitor),
            ElementType::Decimal128 => self.deserialize_i128(visitor),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_bool(self.bson.as_bool()?)
    }

    #[cfg(feature = "u2i")]
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Int32 => self.bson.as_i32()?.try_into()?,
            ElementType::Int64 => self.bson.as_i64()?.try_into()?,
            _ => return Err(Error::UnexpectedType),
        };
        visitor.visit_u8(val)
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Int32 => self.bson.as_i32()?.try_into()?,
            ElementType::Int64 => self.bson.as_i64()?.try_into()?,
            _ => return Err(Error::UnexpectedType),
        };
        visitor.visit_i8(val)
    }

    #[cfg(feature = "u2i")]
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Int32 => self.bson.as_i32()?.try_into()?,
            ElementType::Int64 => self.bson.as_i64()?.try_into()?,
            _ => return Err(Error::UnexpectedType),
        };
        visitor.visit_u16(val)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Int32 => self.bson.as_i32()?.try_into()?,
            ElementType::Int64 => self.bson.as_i64()?.try_into()?,
            _ => return Err(Error::UnexpectedType),
        };
        visitor.visit_i16(val)
    }

    #[cfg(feature = "u2i")]
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Int32 => self.bson.as_i32()?.try_into()?,
            ElementType::Int64 => self.bson.as_i64()?.try_into()?,
            _ => return Err(Error::UnexpectedType),
        };
        visitor.visit_u32(val)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Int32 => self.bson.as_i32()?,
            ElementType::Int64 => self.bson.as_i64()?.try_into()?,
            _ => return Err(Error::UnexpectedType),
        };
        visitor.visit_i32(val)
    }

    #[cfg(feature = "u2i")]
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Int32 => self.bson.as_i32()?.try_into()?,
            ElementType::Int64 => self.bson.as_i64()?.try_into()?,
            ElementType::Timestamp => self.bson.as_timestamp()?,
            ElementType::DateTime => self
                .bson
                .as_utc_date_time()?
                .timestamp_millis()
                .try_into()?,
            _ => return Err(Error::UnexpectedType),
        };
        visitor.visit_u64(val)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Int32 => self.bson.as_i32()?.into(),
            ElementType::Int64 => self.bson.as_i64()?,
            ElementType::DateTime => self.bson.as_datetime()?.timestamp_millis(),
            _ => return Err(Error::UnexpectedType),
        };
        visitor.visit_i64(val)
    }

    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Int32 => self.bson.as_i32()?.into(),
            ElementType::Int64 => self.bson.as_i64()?.into(),
            _ => return Err(Error::UnexpectedType),
        };
        visitor.visit_i128(val)
    }

    #[cfg(feature = "u2i")]
    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Int32 => self.bson.as_i32()?.try_into()?,
            ElementType::Int64 => self.bson.as_i64()?.try_into()?,
            ElementType::Timestamp => self.bson.as_timestamp()?.into(),
            _ => return Err(Error::UnexpectedType),
        };
        visitor.visit_u128(val)
    }

    #[cfg(not(feature = "u2i"))]
    fn deserialize_u8<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(Error::MalformedDocument)
    }

    #[cfg(not(feature = "u2i"))]
    fn deserialize_u16<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(Error::MalformedDocument)
    }

    #[cfg(not(feature = "u2i"))]
    fn deserialize_u32<V: Visitor<'de>>(self, _visitor: V) -> Result<V::Value, Self::Error> {
        Err(Error::MalformedDocument)
    }

    #[cfg(not(feature = "u2i"))]
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Timestamp => self.bson.as_timestamp()?.time() as u64, // TODO: Proper Timestamp handling
            ElementType::Int64 => self.bson.as_i64()?.try_into()?,
            _ => return Err(Error::UnexpectedType),
        };
        visitor.visit_u64(val)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_f64(self.bson.as_f64()?)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_f64(self.bson.as_f64()?)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let s = self.bson.as_str()?;
        let mut chars = s.chars();
        let char = match chars.next() {
            Some(char) => char,
            None => return Err(Error::UnexpectedType),
        };
        if chars.next().is_none() {
            visitor.visit_char(char)
        } else {
            // Got multiple characters.
            Err(Error::UnexpectedType)
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::String => visitor.visit_borrowed_str(self.bson.as_str()?),
            ElementType::JavaScriptCode => visitor.visit_borrowed_str(self.bson.as_javascript()?),
            ElementType::Symbol => visitor.visit_borrowed_str(self.bson.as_symbol()?),

            _ => Err(Error::MalformedDocument),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::String => visitor.visit_str(self.bson.as_str()?),
            ElementType::JavaScriptCode => visitor.visit_str(self.bson.as_javascript()?),
            ElementType::Symbol => visitor.visit_str(self.bson.as_symbol()?),
            ElementType::ObjectId => visitor.visit_str(&self.bson.as_object_id()?.to_hex()),
            _ => Err(Error::Unimplemented),
        }
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::String => {
                let raw_data = self.bson.as_bytes();
                let len = i32::from_le_bytes(raw_data[0..4].try_into().expect("i32 needs 4 bytes"));
                assert_eq!(raw_data.len(), len as usize + 4);
                visitor.visit_borrowed_bytes(&raw_data[4..])
            }
            ElementType::Binary => {
                let binary = self.bson.as_binary().expect("was not binary");
                let deserializer = binary::BinaryDeserializer::new(binary);
                deserializer.deserialize_bytes(visitor)
            }
            ElementType::Symbol => {
                let raw_data = self.bson.as_bytes();
                let len = i32::from_le_bytes(raw_data[0..4].try_into().expect("i32 needs 4 bytes"));
                assert_eq!(raw_data.len(), len as usize + 4);
                visitor.visit_borrowed_bytes(&raw_data[4..])
            }
            ElementType::ObjectId => visitor.visit_borrowed_bytes(self.bson.as_bytes()),
            _ => Err(Error::MalformedDocument),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::String => {
                let raw_data = self.bson.as_bytes();
                let len = i32::from_le_bytes(raw_data[0..4].try_into().expect("i32 needs 4 bytes"));
                assert_eq!(raw_data.len(), len as usize + 4);
                visitor.visit_bytes(&raw_data[4..])
            }
            ElementType::Binary => {
                let binary = self.bson.as_binary()?;
                let deserializer = binary::BinaryDeserializer::new(binary);
                deserializer.deserialize_byte_buf(visitor)
            }
            ElementType::Symbol => {
                let raw_data = self.bson.as_bytes();
                let len = i32::from_le_bytes(raw_data[0..4].try_into().expect("i32 needs 4 bytes"));
                assert_eq!(raw_data.len(), len as usize + 4);
                visitor.visit_bytes(&raw_data[4..])
            }
            ElementType::ObjectId => visitor.visit_bytes(self.bson.as_bytes()),
            _ => Err(Error::MalformedDocument),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::Null => visitor.visit_unit(),
            _ => Err(Error::MalformedDocument),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(
        self,
        _name: &str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        _name: &str,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::Array => {
                let arr = self.bson.as_array()?;
                let sequencer = BsonArraySequencer::new(arr.into_iter());
                visitor.visit_seq(sequencer)
            }
            ElementType::ObjectId => self.deserialize_byte_buf(visitor),
            _ => Err::<V::Value, Self::Error>(Error::Unimplemented),
        }
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::EmbeddedDocument => {
                let doc = self.bson.as_document()?;
                let mapper = BsonDocumentMap::new(doc.into_iter());
                visitor.visit_map(mapper)
            }
            ElementType::ObjectId => {
                let mapper = RawObjectIdDeserializer::new(self.bson);
                visitor.visit_map(mapper)
            }
            et => {
                println!("Map of {:?}", et);
                Err(Error::TmPErroR)
            }
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(
        self,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::Array => self.deserialize_seq(visitor),
            ElementType::JavaScriptCodeWithScope => {
                js::JavaScriptWithScopeDeserializer::new(self.bson.as_javascript_with_scope()?)
                    .deserialize_tuple(len, visitor)
            }
            ElementType::RegularExpression => {
                regex::RegexDeserializer::new(self.bson.as_regex()?).deserialize_tuple(len, visitor)
            }

            _ => Err(Error::TmPErroR),
        }
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if name == object_id::NAME {
            object_id::RawObjectIdDeserializer::new(self.bson)
                .deserialize_struct(name, fields, visitor)
        } else if name == binary::NAME {
            self.bson
                .as_binary()
                .map_err(Error::from)
                .map(binary::BinaryDeserializer::new)
                .and_then(|de| de.deserialize_struct(name, fields, visitor))
        } else if name == datetime::NAME {
            self.bson
                .as_datetime()
                .map_err(Error::from)
                .map(|dt| dt.timestamp_millis())
                .map(datetime::DateTimeDeserializer::new)
                .and_then(|de| de.deserialize_struct(name, fields, visitor))
        } else if name == js::WITH_SCOPE_NAME {
            self.bson
                .as_javascript_with_scope()
                .map_err(Error::from)
                .map(js::JavaScriptWithScopeDeserializer::new)
                .and_then(|de| de.deserialize_struct(name, fields, visitor))
        } else if name == regex::NAME {
            self.bson
                .as_regex()
                .map_err(Error::from)
                .map(regex::RegexDeserializer::new)
                .and_then(|de| de.deserialize_struct(name, fields, visitor))
        } else {
            self.deserialize_map(visitor)
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &str,
        _fields: &[&str],
        _visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(Error::Unimplemented)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_unit()
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        self.deserialize_str(visitor)
    }
}

struct BsonArraySequencer<'de> {
    arr_iter: ArrayIter<'de>,
}

impl<'de> BsonArraySequencer<'de> {
    fn new(arr_iter: ArrayIter<'de>) -> Self {
        BsonArraySequencer { arr_iter }
    }
}

impl<'de> SeqAccess<'de> for BsonArraySequencer<'de> {
    type Error = Error;

    fn next_element_seed<E>(&mut self, seed: E) -> Result<Option<E::Value>, Self::Error>
    where
        E: DeserializeSeed<'de>,
    {
        match self.arr_iter.next() {
            Some(Ok(bson)) => {
                let mut deserializer = BsonDeserializer::from_rawbson(bson);
                seed.deserialize(&mut deserializer).map(Some)
            }
            Some(Err(err)) => Err(err.into()),
            None => Ok(None),
        }
    }
}

struct BsonDocumentMap<'de> {
    doc_iter: DocIter<'de>,
    next: Option<Element<'de>>,
}

impl<'de> BsonDocumentMap<'de> {
    fn new(doc_iter: DocIter<'de>) -> Self {
        BsonDocumentMap {
            doc_iter,
            next: None,
        }
    }
}

impl<'de> MapAccess<'de> for BsonDocumentMap<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.doc_iter.next() {
            Some(Ok((key, value))) => {
                self.next = Some(value);
                let deserializer = StrDeserializer::new(key);
                Ok(Some(seed.deserialize(deserializer)?))
            }
            Some(Err(err)) => Err(err.into()),
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        let bson = self.next.take().ok_or(Error::Eof)?;
        let mut deserializer = BsonDeserializer::from_rawbson(bson);
        seed.deserialize(&mut deserializer)
    }
}

struct StrDeserializer<'a> {
    value: &'a str,
}

impl<'a> StrDeserializer<'a> {
    fn new(value: &'a str) -> StrDeserializer<'a> {
        StrDeserializer { value }
    }
}

impl<'de> Deserializer<'de> for StrDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_borrowed_str(self.value)
    }

    forward_to_deserialize_any!(
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    );
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::{doc, oid::ObjectId, Bson, DateTime};
    use crate::{spec::BinarySubtype, Binary, JavaScriptCodeWithScope};
    use chrono::Utc;
    use serde::Deserialize;
    
    use crate::raw::{Doc, DocBuf};
    use super::{from_bytes, from_doc};

    mod uuid {
        use std::convert::TryInto;
        use std::fmt;
        
        use serde::de::Visitor;
        use serde::de::{Deserialize, MapAccess};
        use serde::Deserializer;

        use crate::spec::BinarySubtype;
        
        #[derive(Clone, Debug, Eq, PartialEq)]
        pub(super) struct Uuid {
            data: Vec<u8>,
        }

        impl Uuid {
            pub fn new(data: Vec<u8>) -> Uuid {
                Uuid { data }
            }
        }

        impl<'de> Deserialize<'de> for Uuid {
            fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
            where
                D: Deserializer<'de>,
            {
                struct UuidVisitor;

                impl<'de> Visitor<'de> for UuidVisitor {
                    type Value = Uuid;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str("a bson uuid")
                    }

                    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
                    where
                        M: MapAccess<'de>,
                    {
                        let subtype_key = map.next_key::<FieldKey>()?;
                        if subtype_key.map(|dk| dk.key) != Some(super::super::binary::SUBTYPE_FIELD)
                        {
                            return Err(serde::de::Error::custom(
                                "BinarySubtypeKey not found in synthesized struct",
                            ));
                        }

                        let subtype_value: BinarySubtypeFromU8 = map.next_value()?;
                        match subtype_value.subtype {
                            BinarySubtype::Uuid | BinarySubtype::UuidOld => {}
                            _ => {
                                return Err(serde::de::Error::custom(
                                    "Expected binary subtype of Uuid (4) or UuidOld (3)",
                                ))
                            }
                        }

                        let data_key = map.next_key::<FieldKey>()?;

                        if data_key.map(|dk| dk.key) != Some(super::super::binary::DATA_FIELD) {
                            return Err(serde::de::Error::custom(
                                "BinaryDataKey not found in synthesized struct",
                            ));
                        }
                        let data_value: BinaryDataFromBytes = map.next_value()?;
                        Ok(Uuid {
                            data: data_value.data,
                        })
                    }
                }
                static FIELDS: [&str; 2] = [
                    super::super::binary::SUBTYPE_FIELD,
                    super::super::binary::DATA_FIELD,
                ];
                deserializer.deserialize_struct(super::super::binary::NAME, &FIELDS, UuidVisitor)
            }
        }

        struct FieldKey {
            key: &'static str,
        }

        impl FieldKey {
            fn new(key: &'static str) -> FieldKey {
                FieldKey { key }
            }
        }

        impl<'de> Deserialize<'de> for FieldKey {
            fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
            where
                D: Deserializer<'de>,
            {
                struct KeyVisitor;

                impl<'de> Visitor<'de> for KeyVisitor {
                    type Value = FieldKey;

                    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                        formatter.write_str("an identifier")
                    }

                    fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<FieldKey, E> {
                        use super::super::binary::{DATA_FIELD, SUBTYPE_FIELD};
                        if s == SUBTYPE_FIELD {
                            Ok(FieldKey::new(SUBTYPE_FIELD))
                        } else if s == DATA_FIELD {
                            Ok(FieldKey::new(DATA_FIELD))
                        } else {
                            Err(serde::de::Error::custom(format!("unexpected field: {}", s)))
                        }
                    }
                }

                deserializer.deserialize_identifier(KeyVisitor)
            }
        }

        struct BinarySubtypeFromU8 {
            subtype: BinarySubtype,
        }

        impl BinarySubtypeFromU8 {
            fn new(subtype_byte: u8) -> BinarySubtypeFromU8 {
                let subtype = BinarySubtype::from(subtype_byte);
                BinarySubtypeFromU8 { subtype }
            }
        }

        impl<'de> Deserialize<'de> for BinarySubtypeFromU8 {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct SubtypeVisitor;

                impl<'de> Visitor<'de> for SubtypeVisitor {
                    type Value = BinarySubtypeFromU8;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("a u8 representing a binary subtype")
                    }

                    fn visit_u8<E: serde::de::Error>(
                        self,
                        byte: u8,
                    ) -> Result<BinarySubtypeFromU8, E> {
                        Ok(BinarySubtypeFromU8::new(byte))
                    }
                    fn visit_i32<E: serde::de::Error>(
                        self,
                        int: i32,
                    ) -> Result<BinarySubtypeFromU8, E> {
                        Ok(BinarySubtypeFromU8::new(
                            int.try_into().map_err(|_| E::custom("non-byte integer"))?,
                        ))
                    }
                }

                deserializer.deserialize_u8(SubtypeVisitor)
            }
        }

        struct BinaryDataFromBytes {
            data: Vec<u8>,
        }

        impl BinaryDataFromBytes {
            fn new(data: Vec<u8>) -> BinaryDataFromBytes {
                BinaryDataFromBytes { data }
            }
        }

        impl<'de> Deserialize<'de> for BinaryDataFromBytes {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: Deserializer<'de>,
            {
                struct DataVisitor;

                impl<'de> Visitor<'de> for DataVisitor {
                    type Value = BinaryDataFromBytes;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("bytes")
                    }

                    fn visit_bytes<E: serde::de::Error>(
                        self,
                        bytes: &[u8],
                    ) -> Result<BinaryDataFromBytes, E> {
                        Ok(BinaryDataFromBytes::new(bytes.to_vec()))
                    }
                }

                deserializer.deserialize_bytes(DataVisitor)
            }
        }
    }

    #[derive(Debug, Deserialize)]
    struct Person<'a> {
        #[serde(rename = "_id")]
        id: ObjectId,
        first_name: &'a str,
        middle_name: Option<String>,
        last_name: String,
        number: &'a [u8],
        gid: uuid::Uuid,
        has_cookies: bool,
        birth_year: Option<f64>,
    }

    #[test]
    fn deserialize_struct() {
        let mut docbytes = Vec::new();
        let doc = doc! {
            "_id": ObjectId::with_string("abcdefabcdefabcdefabcdef").unwrap(),
            "first_name": "Edward",
            "middle_name": Bson::Null,
            "last_name": "Teach",
            "number": Binary { subtype: BinarySubtype::Generic, bytes: vec![8, 6, 7, 5, 3, 0, 9] },
            "has_cookies": false,
            "gid": Binary { subtype: BinarySubtype::Uuid, bytes: b"12345678901234567890123456789012".to_vec() },
            "birth_year": 15.0,
        };
        doc.to_writer(&mut docbytes)
            .expect("could not encode document");
        let p: Person = from_bytes(&docbytes).expect("could not decode into Person struct");
        assert_eq!(p.first_name, "Edward");
        assert_eq!(p.middle_name, None);
        assert_eq!(p.last_name, "Teach");
        assert_eq!(p.id.to_hex(), "abcdefabcdefabcdefabcdef");
        assert_eq!(p.number, &[8, 6, 7, 5, 3, 0, 9]);
        assert_eq!(p.has_cookies, false);
        assert_eq!(
            p.gid,
            uuid::Uuid::new(b"12345678901234567890123456789012".to_vec())
        );
        assert_eq!(p.birth_year, Some(15.0));
    }

    #[test]
    fn object_id() {
        let object_id = ObjectId::new();
        let doc = doc! {
            "oid": object_id.clone(),
        };
        let mut docbytes = Vec::new();
        doc.to_writer(&mut docbytes)
            .expect("cannot serialize document");
        let as_object: HashMap<String, ObjectId> =
            from_bytes(&docbytes).expect("deserialize object_id");
        assert_eq!(as_object.get("oid").unwrap(), &object_id);
        let as_string: HashMap<String, String> = from_bytes(&docbytes).expect("deserialize string");
        assert_eq!(as_string.get("oid").unwrap(), &object_id.to_hex());
        let as_bytes: HashMap<String, &[u8]> =
            from_bytes(&docbytes).expect("deserialize borrowed bytes");
        assert_eq!(as_bytes.get("oid").unwrap(), &object_id.bytes());
    }

    #[test]
    fn wrong_binary_type_for_uuid() {
        let mut docbytes = Vec::new();
        let doc = &doc! {
            "_id": ObjectId::with_string("abcdefabcdefabcdefabcdef").unwrap(),
            "first_name": "Edward",
            "last_name": "Teach",
            "has cookies": true,
            "number": Binary { subtype: BinarySubtype::BinaryOld, bytes: vec![7, 0, 0, 0, 8, 6, 7, 5, 3, 0, 9] },
            "gid": Binary { subtype: BinarySubtype::Function, bytes: b"12345678901234567890123456789012".to_vec() },
        };
        doc.to_writer(&mut docbytes)
            .expect("could not encode document");

        from_bytes::<Person>(&docbytes).expect_err("Should have failed to decode gid field");
    }

    #[test]
    fn deserialize_map() {
        let mut docbytes = Vec::new();
        let doc = doc! {
            "this": "that",
            "three": "four",
            "keymaster": "gatekeeper",
        };
        doc.to_writer(&mut docbytes)
            .expect("could not encode document");

        let map: HashMap<&str, &str> =
            from_bytes(&docbytes).expect("could not decode into HashMap<&str, &str>");
        assert_eq!(map.len(), 3);
        assert_eq!(*map.get("this").expect("key not found"), "that");
        assert_eq!(*map.get("three").expect("key not found"), "four");
        assert_eq!(*map.get("keymaster").expect("key not found"), "gatekeeper");

        let map: HashMap<String, String> =
            from_bytes(&docbytes).expect("could not decode into HashMap<String, String>");
        assert_eq!(map.len(), 3);
        assert_eq!(map.get("this").expect("key not found"), "that");
        assert_eq!(map.get("three").expect("key not found"), "four");
        assert_eq!(map.get("keymaster").expect("key not found"), "gatekeeper");
    }

    #[test]
    fn deserialize_seq() {
        let mut docbytes = Vec::new();
        let doc = doc! {"array": [1i32, 2i64, 3i32, "abc"]};
        doc.to_writer(&mut docbytes)
            .expect("could not encode document");
        let map: HashMap<String, Vec<Bson>> =
            from_bytes(&docbytes).expect("could not decode into HashMap<String, Vec<Bson>");
        assert_eq!(map.len(), 1);
        let arr = map.get("array").expect("key not found");
        assert_eq!(arr.get(0).expect("no index 0"), &Bson::Int32(1));
        assert_eq!(arr.get(1).expect("no index 1"), &Bson::Int64(2));
        assert_eq!(arr.get(2).expect("no index 2"), &Bson::Int32(3));
        assert_eq!(arr.get(3).expect("no index 3"), &Bson::String("abc".into()));
        assert!(arr.get(4).is_none());
    }

    #[test]
    fn deserialize_js_with_scope() {
        let mut docbytes = Vec::new();
        let doc = doc! {"js_with_scope": JavaScriptCodeWithScope {
            code: String::from("console.log(value);"),
            scope: doc!{"value": "Hello world"},
        }};
        doc.to_writer(&mut docbytes)
            .expect("could not encode document");

        let rawdoc = Doc::new(&docbytes).expect("Invalid document");
        assert!(rawdoc.get_javascript_with_scope("js_with_scope").is_ok());
        let map: HashMap<&str, (&str, HashMap<&str, &str>)> =
            from_doc(rawdoc).expect("could not decode js with scope");
        assert_eq!(
            map.get("js_with_scope").expect("no key js_with_scope").0,
            "console.log(value);"
        );
        assert_eq!(
            map.get("js_with_scope")
                .expect("no key js_with_scope")
                .1
                .get("value")
                .expect("no key value"),
            &"Hello world",
        );
    }

    #[test]
    fn deserialize_regexp() {
        let mut docbytes = Vec::new();
        let doc = doc! {"regex": crate::Regex { pattern: String::from("^_id$"), options: String::from("i") } };
        doc.to_writer(&mut docbytes)
            .expect("could not encode document");
        let rawdoc = Doc::new(&docbytes).expect("Invalid document");
        assert!(rawdoc.get_regex("regex").is_ok());
        let map: HashMap<&str, (&str, &str)> = from_doc(rawdoc).expect("could not decode regex");
        assert_eq!(map.get("regex").expect("no key regex").0, "^_id$");
        assert_eq!(map.get("regex").expect("no key regex").1, "i");
    }

    #[test]
    fn deserialize_utc_datetime_to_struct() {
        #[derive(Deserialize)]
        struct Dateish {
            #[serde(with = "chrono::serde::ts_milliseconds")]
            utc_datetime: chrono::DateTime<Utc>,
        }
        let mut docbytes = Vec::new();
        let doc = doc! {"utc_datetime": Bson::DateTime(Utc::now())};
        doc.to_writer(&mut docbytes)
            .expect("could not encode document");
        let rawdoc = DocBuf::new(docbytes).expect("invalid document");
        assert!(rawdoc.get_datetime("utc_datetime").is_ok());
        let value: Dateish = from_doc(&rawdoc).expect("could not decode utc_datetime");
        let elapsed = Utc::now().signed_duration_since(value.utc_datetime);
        // The previous now was less than half a second ago
        assert!(elapsed.num_milliseconds() >= 0);
        assert!(elapsed.num_milliseconds() < 500);
    }

    #[test]
    fn deserialize_utc_datetime_as_chrono_datetime() {
        let mut docbytes = Vec::new();
        let doc = doc! {"utc_datetime": Utc::now()};
        doc.to_writer(&mut docbytes)
            .expect("could not encode document");
        let rawdoc = DocBuf::new(docbytes).expect("invalid document");
        assert!(rawdoc.get_datetime("utc_datetime").is_ok());
        let map: HashMap<&str, DateTime> =
            from_doc(&rawdoc).expect("could not decode utc_datetime");

        let dt = map.get("utc_datetime").expect("no key utc_datetime");
        println!("{:?}", dt);
        let dt = dt.0;
        let elapsed = Utc::now().signed_duration_since(dt);
        // The previous now was less than half a second ago
        assert!(elapsed.num_milliseconds() >= 0);
        assert!(elapsed.num_milliseconds() < 500);
    }

    #[test]
    fn deserialize_object_id_as_bson() {
        let mut docbytes = Vec::new();
        let doc = doc! { "object_id": ObjectId::with_string("123456123456123456123456").unwrap() };
        doc.to_writer(&mut docbytes)
            .expect("could not encode document");
        let rawdoc = DocBuf::new(docbytes).expect("invalid document");
        assert!(rawdoc.get_object_id("object_id").is_ok());
        let map: HashMap<&str, Bson> = from_doc(&rawdoc).expect("could not decode object_id");
        assert_eq!(
            map.get("object_id").unwrap(),
            &Bson::ObjectId(ObjectId::with_string("123456123456123456123456").unwrap())
        );
    }

    #[test]
    fn deserialize_utc_datetime_as_bson() {
        let mut docbytes = Vec::new();
        let doc = doc! {"utc_datetime": Utc::now()};
        doc.to_writer(&mut docbytes)
            .expect("could not encode document");
        let rawdoc = DocBuf::new(docbytes).expect("invalid document");
        assert!(rawdoc.get_datetime("utc_datetime").is_ok());
        let map: HashMap<&str, Bson> = from_doc(&rawdoc).expect("could not decode utc_datetime");

        let dt = map.get("utc_datetime").expect("no key utc_datetime");
        let dt = dt
            .as_datetime()
            .expect("value was not of type Bson::DateTime");
        let elapsed = Utc::now().signed_duration_since(*dt);
        // The previous now was less than half a second ago
        assert!(elapsed.num_milliseconds() >= 0);
        assert!(elapsed.num_milliseconds() < 500);
    }

    #[test]
    fn deserialize_utc_datetime_as_i64() {
        let mut docbytes = Vec::new();
        let doc = doc! {"utc_datetime": Bson::DateTime(Utc::now())};
        doc.to_writer(&mut docbytes)
            .expect("could not encode document");
        let rawdoc = DocBuf::new(docbytes).expect("invalid document");
        assert!(rawdoc.get_datetime("utc_datetime").is_ok());
        let map: HashMap<&str, i64> =
            from_doc(&rawdoc).expect("could not decode utc_datetime as i64");
        let _time = map.get("utc_datetime").expect("no key utc_datetime");
    }
}
