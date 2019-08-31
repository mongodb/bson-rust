use std::fmt;
use std::vec;

use serde::de::{
    self, Deserialize, DeserializeSeed, Deserializer, EnumAccess, Error, MapAccess, SeqAccess, Unexpected,
    VariantAccess, Visitor,
};

use super::error::{DecoderError, DecoderResult};
use crate::bson::{Bson, TimeStamp, UtcDateTime};
#[cfg(feature = "decimal128")]
use crate::decimal128::Decimal128;
use crate::oid::ObjectId;
use crate::ordered::{OrderedDocument, OrderedDocumentIntoIterator, OrderedDocumentVisitor};
use crate::spec::BinarySubtype;

pub struct BsonVisitor;

impl<'de> Deserialize<'de> for ObjectId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        deserializer.deserialize_map(BsonVisitor).and_then(|bson| {
                                                     if let Bson::ObjectId(oid) = bson {
                                                         Ok(oid)
                                                     } else {
                                                         let err =
                                                             format!("expected objectId extended document, found {}",
                                                                     bson);
                                                         Err(de::Error::invalid_type(Unexpected::Map, &&err[..]))
                                                     }
                                                 })
    }
}

impl<'de> Deserialize<'de> for OrderedDocument {
    /// Deserialize this value given this `Deserializer`.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        deserializer.deserialize_map(BsonVisitor).and_then(|bson| {
            if let Bson::Document(doc) = bson {
                Ok(doc)
            } else {
                let err = format!("expected document, found extended JSON data type: {}", bson);
                Err(de::Error::invalid_type(Unexpected::Map, &&err[..]))
            }
        })
    }
}

impl<'de> Deserialize<'de> for Bson {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Bson, D::Error>
        where D: Deserializer<'de>
    {
        deserializer.deserialize_any(BsonVisitor)
    }
}

impl<'de> Visitor<'de> for BsonVisitor {
    type Value = Bson;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("expecting a Bson")
    }

    #[inline]
    fn visit_bool<E>(self, value: bool) -> Result<Bson, E>
        where E: Error
    {
        Ok(Bson::Boolean(value))
    }

    #[inline]
    fn visit_i8<E>(self, value: i8) -> Result<Bson, E>
        where E: Error
    {
        Ok(Bson::I32(value as i32))
    }

    #[inline]
    fn visit_u8<E>(self, value: u8) -> Result<Bson, E>
        where E: Error
    {
        Err(Error::invalid_type(Unexpected::Unsigned(value as u64), &"a signed integer"))
    }

    #[inline]
    fn visit_i16<E>(self, value: i16) -> Result<Bson, E>
        where E: Error
    {
        Ok(Bson::I32(value as i32))
    }

    #[inline]
    fn visit_u16<E>(self, value: u16) -> Result<Bson, E>
        where E: Error
    {
        Err(Error::invalid_type(Unexpected::Unsigned(value as u64), &"a signed integer"))
    }

    #[inline]
    fn visit_i32<E>(self, value: i32) -> Result<Bson, E>
        where E: Error
    {
        Ok(Bson::I32(value))
    }

    #[inline]
    fn visit_u32<E>(self, value: u32) -> Result<Bson, E>
        where E: Error
    {
        Err(Error::invalid_type(Unexpected::Unsigned(value as u64), &"a signed integer"))
    }

    #[inline]
    fn visit_i64<E>(self, value: i64) -> Result<Bson, E>
        where E: Error
    {
        Ok(Bson::I64(value))
    }

    #[inline]
    fn visit_u64<E>(self, value: u64) -> Result<Bson, E>
        where E: Error
    {
        Err(Error::invalid_type(Unexpected::Unsigned(value), &"a signed integer"))
    }

    #[inline]
    fn visit_f64<E>(self, value: f64) -> Result<Bson, E> {
        Ok(Bson::FloatingPoint(value))
    }

    #[inline]
    fn visit_str<E>(self, value: &str) -> Result<Bson, E>
        where E: de::Error
    {
        self.visit_string(String::from(value))
    }

    #[inline]
    fn visit_string<E>(self, value: String) -> Result<Bson, E> {
        Ok(Bson::String(value))
    }

    #[inline]
    fn visit_none<E>(self) -> Result<Bson, E> {
        Ok(Bson::Null)
    }

    #[inline]
    fn visit_some<D>(self, deserializer: D) -> Result<Bson, D::Error>
        where D: Deserializer<'de>
    {
        deserializer.deserialize_any(self)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Bson, E> {
        Ok(Bson::Null)
    }

    #[inline]
    fn visit_seq<V>(self, mut visitor: V) -> Result<Bson, V::Error>
        where V: SeqAccess<'de>
    {
        let mut values = Vec::new();

        while let Some(elem) = visitor.next_element()? {
            values.push(elem);
        }

        Ok(Bson::Array(values))
    }

    #[inline]
    fn visit_map<V>(self, visitor: V) -> Result<Bson, V::Error>
        where V: MapAccess<'de>
    {
        let values = OrderedDocumentVisitor::new().visit_map(visitor)?;
        Ok(Bson::from_extended_document(values.into()))
    }

    #[inline]
    fn visit_bytes<E>(self, v: &[u8]) -> Result<Bson, E>
        where E: Error
    {
        Ok(Bson::Binary(BinarySubtype::Generic, v.to_vec()))
    }
}

/// Serde Decoder
pub struct Decoder {
    value: Option<Bson>,
}

impl Decoder {
    pub fn new(value: Bson) -> Decoder {
        Decoder { value: Some(value) }
    }
}

macro_rules! forward_to_deserialize {
    ($(
        $name:ident ( $( $arg:ident : $ty:ty ),* );
    )*) => {
        $(
            forward_to_deserialize!{
                func: $name ( $( $arg: $ty ),* );
            }
        )*
    };

    (func: deserialize_enum ( $( $arg:ident : $ty:ty ),* );) => {
        fn deserialize_enum<V>(
            self,
            $(_: $ty,)*
            _visitor: V,
        ) -> ::std::result::Result<V::Value, Self::Error>
            where V: ::serde::de::Visitor<'de>
        {
            Err(::serde::de::Error::custom("unexpected Enum"))
        }
    };

    (func: $name:ident ( $( $arg:ident : $ty:ty ),* );) => {
        #[inline]
        fn $name<V>(
            self,
            $(_: $ty,)*
            visitor: V,
        ) -> ::std::result::Result<V::Value, Self::Error>
            where V: ::serde::de::Visitor<'de>
        {
            self.deserialize_any(visitor)
        }
    };
}

impl<'de> Deserializer<'de> for Decoder {
    type Error = DecoderError;

    #[inline]
    fn deserialize_any<V>(mut self, visitor: V) -> DecoderResult<V::Value>
        where V: Visitor<'de>
    {
        let value = match self.value.take() {
            Some(value) => value,
            None => return Err(DecoderError::EndOfStream),
        };

        match value {
            Bson::FloatingPoint(v) => visitor.visit_f64(v),
            Bson::String(v) => visitor.visit_string(v),
            Bson::Array(v) => {
                let len = v.len();
                visitor.visit_seq(SeqDecoder { iter: v.into_iter(),
                                               len: len })
            }
            Bson::Document(v) => {
                let len = v.len();
                visitor.visit_map(MapDecoder { iter: v.into_iter(),
                                               value: None,
                                               len: len })
            }
            Bson::Boolean(v) => visitor.visit_bool(v),
            Bson::Null => visitor.visit_unit(),
            Bson::I32(v) => visitor.visit_i32(v),
            Bson::I64(v) => visitor.visit_i64(v),
            Bson::Binary(_, v) => visitor.visit_bytes(&v),
            _ => {
                let doc = value.to_extended_document();
                let len = doc.len();
                visitor.visit_map(MapDecoder { iter: doc.into_iter(),
                                               value: None,
                                               len: len })
            }
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> DecoderResult<V::Value>
        where V: Visitor<'de>
    {
        match self.value {
            Some(Bson::Null) => visitor.visit_none(),
            Some(_) => visitor.visit_some(self),
            None => Err(DecoderError::EndOfStream),
        }
    }

    #[inline]
    fn deserialize_enum<V>(mut self,
                           _name: &str,
                           _variants: &'static [&'static str],
                           visitor: V)
                           -> DecoderResult<V::Value>
        where V: Visitor<'de>
    {
        let value = match self.value.take() {
            Some(Bson::Document(value)) => value,
            Some(Bson::String(variant)) => {
                return visitor.visit_enum(EnumDecoder { val: Bson::String(variant),
                                                        decoder: VariantDecoder { val: None } });
            }
            Some(_) => {
                return Err(DecoderError::InvalidType("expected an enum".to_owned()));
            }
            None => {
                return Err(DecoderError::EndOfStream);
            }
        };

        let mut iter = value.into_iter();

        let (variant, value) = match iter.next() {
            Some(v) => v,
            None => return Err(DecoderError::SyntaxError("expected a variant name".to_owned())),
        };

        // enums are encoded in json as maps with a single key:value pair
        match iter.next() {
            Some(_) => Err(DecoderError::InvalidType("expected a single key:value pair".to_owned())),
            None => visitor.visit_enum(EnumDecoder { val: Bson::String(variant),
                                                     decoder: VariantDecoder { val: Some(value) } }),
        }
    }

    #[inline]
    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> DecoderResult<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_newtype_struct(self)
    }

    forward_to_deserialize! {
        deserialize_bool();
        deserialize_u8();
        deserialize_u16();
        deserialize_u32();
        deserialize_u64();
        deserialize_i8();
        deserialize_i16();
        deserialize_i32();
        deserialize_i64();
        deserialize_f32();
        deserialize_f64();
        deserialize_char();
        deserialize_str();
        deserialize_string();
        deserialize_unit();
        deserialize_seq();
        deserialize_bytes();
        deserialize_map();
        deserialize_unit_struct(name: &'static str);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_tuple(len: usize);
        deserialize_identifier();
        deserialize_ignored_any();
        deserialize_byte_buf();
    }
}

struct EnumDecoder {
    val: Bson,
    decoder: VariantDecoder,
}

impl<'de> EnumAccess<'de> for EnumDecoder {
    type Error = DecoderError;
    type Variant = VariantDecoder;
    fn variant_seed<V>(self, seed: V) -> DecoderResult<(V::Value, Self::Variant)>
        where V: DeserializeSeed<'de>
    {
        let dec = Decoder::new(self.val);
        let value = seed.deserialize(dec)?;
        Ok((value, self.decoder))
    }
}

struct VariantDecoder {
    val: Option<Bson>,
}

impl<'de> VariantAccess<'de> for VariantDecoder {
    type Error = DecoderError;

    fn unit_variant(mut self) -> DecoderResult<()> {
        match self.val.take() {
            None => Ok(()),
            Some(val) => Bson::deserialize(Decoder::new(val)).map(|_| ()),
        }
    }

    fn newtype_variant_seed<T>(mut self, seed: T) -> DecoderResult<T::Value>
        where T: DeserializeSeed<'de>
    {
        let dec = Decoder::new(self.val.take().ok_or(DecoderError::EndOfStream)?);
        seed.deserialize(dec)
    }

    fn tuple_variant<V>(mut self, _len: usize, visitor: V) -> DecoderResult<V::Value>
        where V: Visitor<'de>
    {
        if let Bson::Array(fields) = self.val.take().ok_or(DecoderError::EndOfStream)? {
            let de = SeqDecoder { len: fields.len(),
                                  iter: fields.into_iter() };
            de.deserialize_any(visitor)
        } else {
            return Err(DecoderError::InvalidType("expected a tuple".to_owned()));
        }
    }

    fn struct_variant<V>(mut self, _fields: &'static [&'static str], visitor: V) -> DecoderResult<V::Value>
        where V: Visitor<'de>
    {
        if let Bson::Document(fields) = self.val.take().ok_or(DecoderError::EndOfStream)? {
            let de = MapDecoder { len: fields.len(),
                                  iter: fields.into_iter(),
                                  value: None };
            de.deserialize_any(visitor)
        } else {
            return Err(DecoderError::InvalidType("expected a struct".to_owned()));
        }
    }
}

struct SeqDecoder {
    iter: vec::IntoIter<Bson>,
    len: usize,
}

impl<'de> Deserializer<'de> for SeqDecoder {
    type Error = DecoderError;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> DecoderResult<V::Value>
        where V: Visitor<'de>
    {
        if self.len == 0 {
            visitor.visit_unit()
        } else {
            visitor.visit_seq(self)
        }
    }

    forward_to_deserialize! {
        deserialize_bool();
        deserialize_u8();
        deserialize_u16();
        deserialize_u32();
        deserialize_u64();
        deserialize_i8();
        deserialize_i16();
        deserialize_i32();
        deserialize_i64();
        deserialize_f32();
        deserialize_f64();
        deserialize_char();
        deserialize_str();
        deserialize_string();
        deserialize_unit();
        deserialize_option();
        deserialize_seq();
        deserialize_bytes();
        deserialize_map();
        deserialize_unit_struct(name: &'static str);
        deserialize_newtype_struct(name: &'static str);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_tuple(len: usize);
        deserialize_enum(name: &'static str, variants: &'static [&'static str]);
        deserialize_identifier();
        deserialize_ignored_any();
        deserialize_byte_buf();
    }
}

impl<'de> SeqAccess<'de> for SeqDecoder {
    type Error = DecoderError;

    fn next_element_seed<T>(&mut self, seed: T) -> DecoderResult<Option<T::Value>>
        where T: DeserializeSeed<'de>
    {
        match self.iter.next() {
            None => Ok(None),
            Some(value) => {
                self.len -= 1;
                let de = Decoder::new(value);
                match seed.deserialize(de) {
                    Ok(value) => Ok(Some(value)),
                    Err(err) => Err(err),
                }
            }
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

struct MapDecoder {
    iter: OrderedDocumentIntoIterator,
    value: Option<Bson>,
    len: usize,
}

impl<'de> MapAccess<'de> for MapDecoder {
    type Error = DecoderError;

    fn next_key_seed<K>(&mut self, seed: K) -> DecoderResult<Option<K::Value>>
        where K: DeserializeSeed<'de>
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.len -= 1;
                self.value = Some(value);

                let de = Decoder::new(Bson::String(key));
                match seed.deserialize(de) {
                    Ok(val) => Ok(Some(val)),
                    Err(DecoderError::UnknownField(_)) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> DecoderResult<V::Value>
        where V: DeserializeSeed<'de>
    {
        let value = self.value.take().ok_or(DecoderError::EndOfStream)?;
        let de = Decoder::new(value);
        seed.deserialize(de)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

impl<'de> Deserializer<'de> for MapDecoder {
    type Error = DecoderError;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> DecoderResult<V::Value>
        where V: Visitor<'de>
    {
        visitor.visit_map(self)
    }

    forward_to_deserialize! {
        deserialize_bool();
        deserialize_u8();
        deserialize_u16();
        deserialize_u32();
        deserialize_u64();
        deserialize_i8();
        deserialize_i16();
        deserialize_i32();
        deserialize_i64();
        deserialize_f32();
        deserialize_f64();
        deserialize_char();
        deserialize_str();
        deserialize_string();
        deserialize_unit();
        deserialize_option();
        deserialize_seq();
        deserialize_bytes();
        deserialize_map();
        deserialize_unit_struct(name: &'static str);
        deserialize_newtype_struct(name: &'static str);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_tuple(len: usize);
        deserialize_enum(name: &'static str, variants: &'static [&'static str]);
        deserialize_identifier();
        deserialize_ignored_any();
        deserialize_byte_buf();
    }
}

impl<'de> Deserialize<'de> for TimeStamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        match Bson::deserialize(deserializer)? {
            Bson::TimeStamp(ts) => {
                let ts = ts.to_le();

                Ok(TimeStamp { t: ((ts as u64) >> 32) as u32,
                               i: (ts & 0xFFFF_FFFF) as u32 })
            }
            _ => Err(D::Error::custom("expecting TimeStamp")),
        }
    }
}

#[cfg(feature = "decimal128")]
impl<'de> Deserialize<'de> for Decimal128 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        match Bson::deserialize(deserializer)? {
            Bson::Decimal128(d128) => Ok(d128),
            _ => Err(D::Error::custom("expecting Decimal128")),
        }
    }
}

impl<'de> Deserialize<'de> for UtcDateTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer<'de>
    {
        match Bson::deserialize(deserializer)? {
            Bson::UtcDatetime(dt) => Ok(UtcDateTime(dt)),
            _ => Err(D::Error::custom("expecting UtcDateTime")),
        }
    }
}
