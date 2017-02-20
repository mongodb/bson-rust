use std::vec;
use std::fmt;

use serde::de::{self, Deserialize, Deserializer, Visitor, MapVisitor, SeqVisitor, VariantVisitor,
                DeserializeSeed, EnumVisitor};
use serde::de::Unexpected;

use bson::Bson;
use oid::ObjectId;
use ordered::{OrderedDocument, OrderedDocumentIntoIterator, OrderedDocumentVisitor};
use super::error::{DecoderError, DecoderResult};

pub struct BsonVisitor;


impl Deserialize for ObjectId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer
    {
        deserializer.deserialize_map(BsonVisitor)
            .and_then(|bson| {
                if let Bson::ObjectId(oid) = bson {
                    Ok(oid)
                } else {
                    let err = format!("expected objectId extended document, found {}", bson);
                    Err(de::Error::invalid_type(Unexpected::Map, &&err[..]))
                }
            })
    }
}

impl Deserialize for OrderedDocument {
    /// Deserialize this value given this `Deserializer`.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where D: Deserializer
    {
        deserializer.deserialize_map(BsonVisitor)
            .and_then(|bson| if let Bson::Document(doc) = bson {
                Ok(doc)
            } else {
                let err = format!("expected document, found extended JSON data type: {}", bson);
                Err(de::Error::invalid_type(Unexpected::Map, &&err[..]))
            })
    }
}

impl Deserialize for Bson {
    #[inline]
    fn deserialize<D>(deserializer: D) -> Result<Bson, D::Error>
        where D: Deserializer
    {
        deserializer.deserialize(BsonVisitor)
    }
}

impl Visitor for BsonVisitor {
    type Value = Bson;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "expecting a Bson")
    }

    #[inline]
    fn visit_bool<E>(self, value: bool) -> Result<Bson, E> {
        Ok(Bson::Boolean(value))
    }

    #[inline]
    fn visit_i8<E>(self, value: i8) -> Result<Bson, E> {
        Ok(Bson::I32(value as i32))
    }


    #[inline]
    fn visit_i16<E>(self, value: i16) -> Result<Bson, E> {
        Ok(Bson::I32(value as i32))
    }


    #[inline]
    fn visit_i32<E>(self, value: i32) -> Result<Bson, E> {
        Ok(Bson::I32(value))
    }

    #[inline]
    fn visit_i64<E>(self, value: i64) -> Result<Bson, E> {
        Ok(Bson::I64(value))
    }

    #[inline]
    fn visit_u64<E>(self, value: u64) -> Result<Bson, E> {
        Ok(Bson::I64(value as i64))
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
        where D: Deserializer
    {
        deserializer.deserialize(self)
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Bson, E> {
        Ok(Bson::Null)
    }

    #[inline]
    fn visit_seq<V>(self, visitor: V) -> Result<Bson, V::Error>
        where V: SeqVisitor
    {
        let values = try!(de::impls::VecVisitor::new().visit_seq(visitor));
        Ok(Bson::Array(values))
    }

    #[inline]
    fn visit_map<V>(self, visitor: V) -> Result<Bson, V::Error>
        where V: MapVisitor
    {
        let values = try!(OrderedDocumentVisitor::new().visit_map(visitor));
        Ok(Bson::from_extended_document(values.into()))
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
            where V: ::serde::de::Visitor
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
            where V: ::serde::de::Visitor
        {
            self.deserialize(visitor)
        }
    };
}

impl Deserializer for Decoder {
    type Error = DecoderError;

    #[inline]
    fn deserialize<V>(mut self, visitor: V) -> DecoderResult<V::Value>
        where V: Visitor
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
                visitor.visit_seq(SeqDecoder {
                    iter: v.into_iter(),
                    len: len,
                })
            }
            Bson::Document(v) => {
                let len = v.len();
                visitor.visit_map(MapDecoder {
                    iter: v.into_iter(),
                    value: None,
                    len: len,
                })
            }
            Bson::Boolean(v) => visitor.visit_bool(v),
            Bson::Null => visitor.visit_unit(),
            Bson::I32(v) => visitor.visit_i32(v),
            Bson::I64(v) => visitor.visit_i64(v),
            _ => {
                let doc = value.to_extended_document();
                let len = doc.len();
                visitor.visit_map(MapDecoder {
                    iter: doc.into_iter(),
                    value: None,
                    len: len,
                })
            }
        }
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> DecoderResult<V::Value>
        where V: Visitor
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
        where V: Visitor
    {
        let value = match self.value.take() {
            Some(Bson::Document(value)) => value,
            Some(Bson::String(variant)) => {
                return visitor.visit_enum(EnumDecoder {
                    val: Bson::String(variant),
                    decoder: VariantDecoder { val: None },
                });
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
            Some(_) => {
                Err(DecoderError::InvalidType("expected a single key:value pair".to_owned()))
            }
            None => {
                visitor.visit_enum(EnumDecoder {
                    val: Bson::String(variant),
                    decoder: VariantDecoder { val: Some(value) },
                })
            }
        }
    }

    #[inline]
    fn deserialize_newtype_struct<V>(self,
                                     _name: &'static str,
                                     visitor: V)
                                     -> DecoderResult<V::Value>
        where V: Visitor
    {
        visitor.visit_newtype_struct(self)
    }

    forward_to_deserialize!{
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
        deserialize_seq_fixed_size(len: usize);
        deserialize_bytes();
        deserialize_map();
        deserialize_unit_struct(name: &'static str);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_struct_field();
        deserialize_tuple(len: usize);
        deserialize_ignored_any();
        deserialize_byte_buf();
    }
}

struct EnumDecoder {
    val: Bson,
    decoder: VariantDecoder,
}

impl EnumVisitor for EnumDecoder {
    type Error = DecoderError;
    type Variant = VariantDecoder;
    fn visit_variant_seed<V>(self, seed: V) -> DecoderResult<(V::Value, Self::Variant)>
        where V: DeserializeSeed
    {
        let dec = Decoder::new(self.val);
        let value = seed.deserialize(dec)?;
        Ok((value, self.decoder))
    }
}

struct VariantDecoder {
    val: Option<Bson>,
}

impl VariantVisitor for VariantDecoder {
    type Error = DecoderError;

    fn visit_unit(mut self) -> DecoderResult<()> {
        match self.val.take() {
            None => Ok(()),
            Some(val) => {
                try!(Deserialize::deserialize(Decoder::new(val)));
                Ok(())
            }
        }
    }

    fn visit_newtype_seed<T>(mut self, seed: T) -> DecoderResult<T::Value>
        where T: DeserializeSeed
    {
        let dec = Decoder::new(try!(self.val
            .take()
            .ok_or(DecoderError::EndOfStream)));
        seed.deserialize(dec)
    }

    fn visit_tuple<V>(mut self, _len: usize, visitor: V) -> DecoderResult<V::Value>
        where V: Visitor
    {
        if let Bson::Array(fields) =
            try!(self.val
                .take()
                .ok_or(DecoderError::EndOfStream)) {

            let de = SeqDecoder {
                len: fields.len(),
                iter: fields.into_iter(),
            };
            de.deserialize(visitor)
        } else {
            return Err(DecoderError::InvalidType("expected a tuple".to_owned()));
        }
    }

    fn visit_struct<V>(mut self,
                       _fields: &'static [&'static str],
                       visitor: V)
                       -> DecoderResult<V::Value>
        where V: Visitor
    {
        if let Bson::Document(fields) =
            try!(self.val
                .take()
                .ok_or(DecoderError::EndOfStream)) {
            let de = MapDecoder {
                len: fields.len(),
                iter: fields.into_iter(),
                value: None,
            };
            de.deserialize(visitor)
        } else {
            return Err(DecoderError::InvalidType("expected a struct".to_owned()));
        }
    }
}

struct SeqDecoder {
    iter: vec::IntoIter<Bson>,
    len: usize,
}

impl Deserializer for SeqDecoder {
    type Error = DecoderError;

    #[inline]
    fn deserialize<V>(self, visitor: V) -> DecoderResult<V::Value>
        where V: Visitor
    {
        if self.len == 0 {
            visitor.visit_unit()
        } else {
            visitor.visit_seq(self)
        }
    }

    forward_to_deserialize!{
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
        deserialize_seq_fixed_size(len: usize);
        deserialize_bytes();
        deserialize_map();
        deserialize_unit_struct(name: &'static str);
        deserialize_newtype_struct(name: &'static str);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_struct_field();
        deserialize_tuple(len: usize);
        deserialize_enum(name: &'static str, variants: &'static [&'static str]);
        deserialize_ignored_any();
        deserialize_byte_buf();
    }
}

impl SeqVisitor for SeqDecoder {
    type Error = DecoderError;

    fn visit_seed<T>(&mut self, seed: T) -> DecoderResult<Option<T::Value>>
        where T: DeserializeSeed
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

struct MapDecoder {
    iter: OrderedDocumentIntoIterator,
    value: Option<Bson>,
    len: usize,
}

impl MapVisitor for MapDecoder {
    type Error = DecoderError;

    fn visit_key_seed<K>(&mut self, seed: K) -> DecoderResult<Option<K::Value>>
        where K: DeserializeSeed
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

    fn visit_value_seed<V>(&mut self, seed: V) -> DecoderResult<V::Value>
        where V: DeserializeSeed
    {
        let value = try!(self.value
            .take()
            .ok_or(DecoderError::EndOfStream));
        let de = Decoder::new(value);
        seed.deserialize(de)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl Deserializer for MapDecoder {
    type Error = DecoderError;

    #[inline]
    fn deserialize<V>(self, visitor: V) -> DecoderResult<V::Value>
        where V: Visitor
    {
        visitor.visit_map(self)
    }

    forward_to_deserialize!{
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
        deserialize_seq_fixed_size(len: usize);
        deserialize_bytes();
        deserialize_map();
        deserialize_unit_struct(name: &'static str);
        deserialize_newtype_struct(name: &'static str);
        deserialize_tuple_struct(name: &'static str, len: usize);
        deserialize_struct(name: &'static str, fields: &'static [&'static str]);
        deserialize_struct_field();
        deserialize_tuple(len: usize);
        deserialize_enum(name: &'static str, variants: &'static [&'static str]);
        deserialize_ignored_any();
        deserialize_byte_buf();
    }
}
