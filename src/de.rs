use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, Visitor};
use serde::forward_to_deserialize_any;
use serde::Deserialize;

use std::convert::TryInto;
use std::fmt::Debug;
use std::num::TryFromIntError;

use crate::raw::{RawBson, RawBsonDoc, RawBsonDocIterator, RawValueAccessError};
use crate::spec::ElementType;

pub mod binary;
pub mod object_id;

#[derive(Debug)]
pub enum Error {
    Eof,
    TrailingData(Vec<u8>),
    EncodingError,
    MalformedDocument,
    Unimplemented,
    IntConversion(TryFromIntError),
    Internal(String),
    NotFound,
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

impl<'a> From<RawValueAccessError<'a>> for Error {
    fn from(val: RawValueAccessError) -> Error {
        match val {
            RawValueAccessError::EncodingError(_) => Error::EncodingError,
            RawValueAccessError::UnexpectedType => Error::MalformedDocument,
            RawValueAccessError::NotPresent => Error::NotFound,
        }
    }
}

pub struct BsonDeserializer<'de> {
    bson: RawBson<'de>,
}

impl<'de> BsonDeserializer<'de> {
    pub fn from_rawdoc(doc: RawBsonDoc<'de>) -> Self {
        BsonDeserializer::from_rawbson(RawBson::new(ElementType::EmbeddedDocument, doc.as_bytes()))
    }

    pub fn from_rawbson(bson: RawBson<'de>) -> Self {
        BsonDeserializer { bson: bson.into() }
    }
}

pub fn from_bytes<'a, T>(data: &'a [u8]) -> Result<T, Error>
where
    T: Deserialize<'a>,
{
    let doc = RawBsonDoc::new(data);
    let mut deserializer = BsonDeserializer::from_rawdoc(doc);
    let t = T::deserialize(&mut deserializer)?;
    // TODO: Implement completion check.
    Ok(t)
}

impl<'a, 'de: 'a> Deserializer<'de> for &'a mut BsonDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        println!("deserialize any");
        match self.bson.element_type() {
            ElementType::FloatingPoint => self.deserialize_f64(visitor),
            ElementType::Utf8String => self.deserialize_str(visitor),
            ElementType::EmbeddedDocument => self.deserialize_map(visitor),
            ElementType::Array => self.deserialize_seq(visitor),
            ElementType::Binary => self.deserialize_bytes(visitor),
            ElementType::Undefined => self.deserialize_unit(visitor),
            ElementType::ObjectId => self.deserialize_struct(object_id::NAME, object_id::FIELDS, visitor),
            ElementType::Boolean => self.deserialize_bool(visitor),
            ElementType::UtcDatetime => Err(Error::Unimplemented),
            ElementType::NullValue => self.deserialize_unit(visitor),
            ElementType::DbPointer => Err(Error::Unimplemented),  // deserialize (&str, ObjectId), (&str, &[u8]), or struct
            ElementType::RegularExpression => Err(Error::Unimplemented), // deserialize (&str, &str) or struct
            ElementType::JavaScriptCode => self.deserialize_str(visitor),
            ElementType::Symbol => self.deserialize_str(visitor),
            ElementType::JavaScriptCodeWithScope => Err(Error::Unimplemented), // deserialize (&'str, Map) or struct
            ElementType::Integer32Bit => self.deserialize_i32(visitor),
            ElementType::TimeStamp => self.deserialize_u64(visitor),
            ElementType::Integer64Bit => self.deserialize_i64(visitor),
            ElementType::MinKey => self.deserialize_unit(visitor),
            ElementType::MaxKey => self.deserialize_unit(visitor),
            #[cfg(feature = "decimal128")]
            ElementType::Decimal128Bit => self.deserialize_i128(visitor),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_bool(self.bson.as_bool()?)
    }

    #[cfg(feature = "u2i")]
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Integer32Bit => {
                self.bson.as_i32()?.try_into()?
            }
            ElementType::Integer64Bit => {
                self.bson.as_i64()?.try_into()?
            }
            _ => return Err(Error::Unimplemented),
        };
        visitor.visit_u8(val)
    }


    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Integer32Bit => {
                self.bson.as_i32()?.try_into()?
            }
            ElementType::Integer64Bit => {
                self.bson.as_i64()?.try_into()?
            }
            _ => return Err(Error::Unimplemented),
        };
        visitor.visit_i8(val)
    }

    #[cfg(feature = "u2i")]
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Integer32Bit => {
                self.bson.as_i32()?.try_into()?
            }
            ElementType::Integer64Bit => {
                self.bson.as_i64()?.try_into()?
            }
            _ => return Err(Error::Unimplemented),
        };
        visitor.visit_u16(val)
    }
    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Integer32Bit => {
                self.bson.as_i32()?.try_into()?
            }
            ElementType::Integer64Bit => {
                self.bson.as_i64()?.try_into()?
            }
            _ => return Err(Error::Unimplemented),
        };
        visitor.visit_i16(val)
    }

    #[cfg(feature = "u2i")]
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Integer32Bit => {
                self.bson.as_i32()?.try_into()?
            }
            ElementType::Integer64Bit => {
                self.bson.as_i64()?.try_into()?
            }
            _ => return Err(Error::Unimplemented),
        };
        visitor.visit_u32(val)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Integer32Bit => {
                self.bson.as_i32()?
            }
            ElementType::Integer64Bit => {
                self.bson.as_i64()?.try_into()?
            }
            _ => return Err(Error::Unimplemented),
        };
        visitor.visit_i32(val)
    }

    #[cfg(feature = "u2i")]
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Integer32Bit => {
                self.bson.as_i32()?.try_into()?
            }
            ElementType::Integer64Bit => {
                self.bson.as_i64()?.try_into()?
            }
            _ => return Err(Error::Unimplemented),
        };
        visitor.visit_u64(val)

    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Integer32Bit => {
                self.bson.as_i32()?.into()
            }
            ElementType::Integer64Bit => {
                self.bson.as_i64()?
            }
            _ => return Err(Error::Unimplemented),
        };
        visitor.visit_i64(val)
    }

    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Integer32Bit => {
                self.bson.as_i32()?.into()
            }
            ElementType::Integer64Bit => {
                self.bson.as_i64()?.into()
            }
            _ => return Err(Error::Unimplemented),
        };
        visitor.visit_i128(val)
    }

    #[cfg(feature = "u2i")]
    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        let val = match self.bson.element_type() {
            ElementType::Integer32Bit => {
                self.bson.as_i32()?.try_into()?
            }
            ElementType::Integer64Bit => {
                self.bson.as_i64()?.try_into()?
            }
            _ => return Err(Error::MalformedDocument),
        };
        visitor.visit_u128(val)
    }

    #[cfg(not(feature = "u2i"))]
    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { Err(Error::MalformedDocument) }

    #[cfg(not(feature = "u2i"))]
    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { Err(Error::MalformedDocument) }

    #[cfg(not(feature = "u2i"))]
    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { Err(Error::MalformedDocument) }

    #[cfg(not(feature = "u2i"))]
    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> { Err(Error::MalformedDocument) }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_f64(self.bson.as_f64()?.into())
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_f64(self.bson.as_f64()?)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        Err(Error::Unimplemented)
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        println!("deserialize str");
        match self.bson.element_type() {
            ElementType::Utf8String => visitor.visit_borrowed_str(self.bson.as_str()?),
            ElementType::JavaScriptCode => {
                visitor.visit_borrowed_str(self.bson.as_javascript()?)
            }
            ElementType::Symbol => visitor.visit_borrowed_str(self.bson.as_symbol()?),

            _ => Err(Error::Unimplemented),
        }
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        println!("deserialize string");
        match self.bson.element_type() {
            ElementType::Utf8String => visitor.visit_str(self.bson.as_str()?),
            ElementType::JavaScriptCode => {
                visitor.visit_str(self.bson.as_javascript()?)
            }
            ElementType::Symbol => visitor.visit_str(self.bson.as_symbol()?),
            _ => Err(Error::Unimplemented),
        }
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        println!("deserializing bytes");
        match self.bson.element_type() {
            ElementType::Utf8String => {
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

            _ => Err(Error::MalformedDocument),
        }
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::Utf8String => {
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
            _ => Err(Error::MalformedDocument),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::NullValue => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::NullValue => visitor.visit_unit(),
            _ => Err(Error::MalformedDocument),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, name: &str, visitor: V) -> Result<V::Value, Self::Error> {
        Err(Error::Unimplemented)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, name: &str, visitor: V) -> Result<V::Value, Self::Error> {
        Err(Error::Unimplemented)
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        Err(Error::Unimplemented)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        println!("deserialize map with type: {:?}", self.bson.element_type());
        match self.bson.element_type() {
            ElementType::EmbeddedDocument => {
                let doc = self.bson.as_document()?;
                let mapper = BsonDocumentMap::new(doc.into_iter());
                Ok(visitor.visit_map(mapper)?)
            }
            _ => Err(Error::MalformedDocument),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error> {
        Err(Error::Unimplemented)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        name: &str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(Error::Unimplemented)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        println!("deserializing struct with name {} and fields {:?}", name, fields);
        if name == object_id::NAME {
            object_id::ObjectIdDeserializer::new(self.bson).deserialize_struct(name, fields, visitor)
        } else if name == binary::NAME {
            self.bson
                .as_binary()
                .map(binary::BinaryDeserializer::new)
                .map(|de| de.deserialize_struct(name, fields, visitor))?
        } else {
            self.deserialize_map(visitor)
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &str,
        fields: &[&str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        Err(Error::Unimplemented)
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        Err(Error::Unimplemented)
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        Err(Error::Unimplemented)
    }
}

struct BsonDocumentMap<'de> {
    doc_iter: RawBsonDocIterator<'de>,
    next: Option<RawBson<'de>>,
}

impl<'de> BsonDocumentMap<'de> {
    fn new(doc_iter: RawBsonDocIterator<'de>) -> Self {
        BsonDocumentMap {
            doc_iter: doc_iter,
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
        println!("next key seed");
        match self.doc_iter.next() {
            Some((key, value)) => {
                self.next = Some(value);
                let deserializer = StrDeserializer::new(key);
                Ok(Some(seed.deserialize(deserializer)?))
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        println!("next value seed");
        let bson = self.next.take().ok_or(Error::MalformedDocument)?;
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
        println!("deserialize identifier");
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

    use crate::oid::ObjectId;
    use crate::spec::BinarySubtype;
    use crate::{doc, encode_document, Bson};

    use serde_derive::Deserialize;

    use super::from_bytes;

    mod uuid {
        use serde::de::Visitor;
        use serde::de::{Deserialize, MapAccess};
        use serde::export::fmt::Error;
        use serde::export::Formatter;
        use serde::Deserializer;

        use crate::spec::BinarySubtype;

        #[derive(Clone, Debug, Eq, PartialEq)]
        pub(super) struct Uuid {
            data: Vec<u8>,
        }

        impl Uuid {
            pub fn new(data: Vec<u8>) -> Uuid {
                return Uuid { data };
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

                    fn expecting(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
                        formatter.write_str("a bson uuid")
                    }

                    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
                    where
                        M: MapAccess<'de>,
                    {
                        let subtype_key = map.next_key::<FieldKey>()?;
                        if subtype_key.map(|dk| dk.key) != Some(super::super::binary::SUBTYPE_FIELD) {
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
                        // Handle old vs new uuid parsing...
                        Ok(Uuid { data: data_value.data })
                    }
                }
                static FIELDS: [&str; 2] = [super::super::binary::SUBTYPE_FIELD, super::super::binary::DATA_FIELD];
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

                    fn expecting(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
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

                    fn expecting(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
                        formatter.write_str("a u8 representing a binary subtype")
                    }

                    fn visit_u8<E: serde::de::Error>(self, byte: u8) -> Result<BinarySubtypeFromU8, E> {
                        Ok(BinarySubtypeFromU8::new(byte))
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

                    fn expecting(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
                        formatter.write_str("bytes")
                    }

                    fn visit_bytes<E: serde::de::Error>(self, bytes: &[u8]) -> Result<BinaryDataFromBytes, E> {
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
        encode_document(
            &mut docbytes,
            &doc! {
                "_id": ObjectId::with_string("abcdefabcdefabcdefabcdef").unwrap(),
                "first_name": "Edward",
                "middle_name": Bson::Null,
                "last_name": "Teach",
                "number": (BinarySubtype::BinaryOld, vec![7, 0, 0, 0, 8, 6, 7, 5, 3, 0, 9]),
                "has_cookies": false,
                "gid": (BinarySubtype::Uuid, b"12345678901234567890123456789012".to_vec()),
                "birth_year": 15.0,
            },
        )
        .expect("could not encode document");
        let p: Person = from_bytes(&docbytes).expect("could not decode into Person struct");
        assert_eq!(p.first_name, "Edward");
        assert_eq!(p.middle_name, None);
        assert_eq!(p.last_name, "Teach");
        assert_eq!(p.id.to_hex(), "abcdefabcdefabcdefabcdef");
        assert_eq!(p.number, &[8, 6, 7, 5, 3, 0, 9]);
        assert_eq!(p.has_cookies, false);
        assert_eq!(p.gid, uuid::Uuid::new(b"12345678901234567890123456789012".to_vec()));
        assert_eq!(p.birth_year, Some(15.0));
    }

    #[test]
    fn wrong_binary_type_for_uuid() {
        let mut docbytes = Vec::new();
        encode_document(
            &mut docbytes,
            &doc! {
                "_id": ObjectId::with_string("abcdefabcdefabcdefabcdef").unwrap(),
                "first_name": "Edward",
                "last_name": "Teach",
                "has cookies": true,
                "number": (BinarySubtype::BinaryOld, vec![7, 0, 0, 0, 8, 6, 7, 5, 3, 0, 9]),
                "gid": (BinarySubtype::Function, b"12345678901234567890123456789012".to_vec()),
            },
        )
        .expect("could not encode document");

        from_bytes::<Person>(&docbytes).expect_err("Should have failed to decode gid field");
    }

    #[test]
    fn deserialize_map() {
        let mut docbytes = Vec::new();
        encode_document(
            &mut docbytes,
            &doc! {
                "this": "that",
                "three": "four",
                "keymaster": "gatekeeper",

            },
        )
        .expect("could not encode document");
        let map: HashMap<&str, &str> = from_bytes(&docbytes).expect("could not decode into HashMap<&str, &str>");
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
}
