use std::vec;

use serde::de::{self, Deserialize, Deserializer, Visitor, EnumVisitor, MapVisitor, SeqVisitor,
                VariantVisitor};

use bson::Bson;
use oid::ObjectId;
use ordered::{OrderedDocument, OrderedDocumentIntoIterator, OrderedDocumentVisitor};
use super::error::{DecoderError, DecoderResult};

pub struct BsonVisitor;


impl Deserialize for ObjectId {
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer
    {
        deserializer.deserialize_map(BsonVisitor)
            .and_then(|bson| if let Bson::ObjectId(oid) = bson {
                Ok(oid)
            } else {
                Err(de::Error::invalid_value(&format!("expected objectId extended document, \
                                                       found {}",
                                                      bson)))
            })
    }
}

impl Deserialize for OrderedDocument {
    /// Deserialize this value given this `Deserializer`.
    fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
        where D: Deserializer
    {
        deserializer.deserialize_map(BsonVisitor)
            .and_then(|bson| if let Bson::Document(doc) = bson {
                Ok(doc)
            } else {
                Err(de::Error::invalid_value(&format!("expected document, found extended JSON \
                                                       data type: {}",
                                                      bson)))
            })
    }
}

impl Deserialize for Bson {
    #[inline]
    fn deserialize<D>(deserializer: &mut D) -> Result<Bson, D::Error>
        where D: Deserializer
    {
        deserializer.deserialize(BsonVisitor)
    }
}

impl Visitor for BsonVisitor {
    type Value = Bson;

    #[inline]
    fn visit_bool<E>(&mut self, value: bool) -> Result<Bson, E> {
        Ok(Bson::Boolean(value))
    }

    #[inline]
    fn visit_i8<E>(&mut self, value: i8) -> Result<Bson, E> {
        Ok(Bson::I32(value as i32))
    }


    #[inline]
    fn visit_i16<E>(&mut self, value: i16) -> Result<Bson, E> {
        Ok(Bson::I32(value as i32))
    }


    #[inline]
    fn visit_i32<E>(&mut self, value: i32) -> Result<Bson, E> {
        Ok(Bson::I32(value))
    }

    #[inline]
    fn visit_i64<E>(&mut self, value: i64) -> Result<Bson, E> {
        Ok(Bson::I64(value))
    }

    #[inline]
    fn visit_u64<E>(&mut self, value: u64) -> Result<Bson, E> {
        Ok(Bson::I64(value as i64))
    }

    #[inline]
    fn visit_f64<E>(&mut self, value: f64) -> Result<Bson, E> {
        Ok(Bson::FloatingPoint(value))
    }

    #[inline]
    fn visit_str<E>(&mut self, value: &str) -> Result<Bson, E>
        where E: de::Error
    {
        self.visit_string(String::from(value))
    }

    #[inline]
    fn visit_string<E>(&mut self, value: String) -> Result<Bson, E> {
        Ok(Bson::String(value))
    }

    #[inline]
    fn visit_none<E>(&mut self) -> Result<Bson, E> {
        Ok(Bson::Null)
    }

    #[inline]
    fn visit_some<D>(&mut self, deserializer: &mut D) -> Result<Bson, D::Error>
        where D: Deserializer
    {
        de::Deserialize::deserialize(deserializer)
    }

    #[inline]
    fn visit_unit<E>(&mut self) -> Result<Bson, E> {
        Ok(Bson::Null)
    }

    #[inline]
    fn visit_seq<V>(&mut self, visitor: V) -> Result<Bson, V::Error>
        where V: SeqVisitor
    {
        let values = try!(de::impls::VecVisitor::new().visit_seq(visitor));
        Ok(Bson::Array(values))
    }

    #[inline]
    fn visit_map<V>(&mut self, visitor: V) -> Result<Bson, V::Error>
        where V: MapVisitor
    {
        let values = try!(OrderedDocumentVisitor::new().visit_map(visitor));
        Ok(Bson::from_extended_document(values.into()))
    }
}

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
            &mut self,
            $(_: $ty,)*
            _visitor: V,
        ) -> ::std::result::Result<V::Value, Self::Error>
            where V: ::serde::de::EnumVisitor
        {
            Err(::serde::de::Error::invalid_type(::serde::de::Type::Enum))
        }
    };

    (func: $name:ident ( $( $arg:ident : $ty:ty ),* );) => {
        #[inline]
        fn $name<V>(
            &mut self,
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
    fn deserialize<V>(&mut self, mut visitor: V) -> DecoderResult<V::Value>
        where V: Visitor
    {
        let value = match self.value.take() {
            Some(value) => value,
            None => {
                return Err(de::Error::end_of_stream());
            }
        };

        match value {
            Bson::FloatingPoint(v) => visitor.visit_f64(v),
            Bson::String(v) => visitor.visit_string(v),
            Bson::Array(v) => {
                let len = v.len();
                visitor.visit_seq(SeqDecoder {
                    de: self,
                    iter: v.into_iter(),
                    len: len,
                })
            }
            Bson::Document(v) => {
                let len = v.len();
                visitor.visit_map(MapDecoder {
                    de: self,
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
                    de: self,
                    iter: doc.into_iter(),
                    value: None,
                    len: len,
                })
            }
        }
    }

    #[inline]
    fn deserialize_option<V>(&mut self, mut visitor: V) -> DecoderResult<V::Value>
        where V: Visitor
    {
        match self.value {
            Some(Bson::Null) => visitor.visit_none(),
            Some(_) => visitor.visit_some(self),
            None => Err(de::Error::end_of_stream()),
        }
    }

    #[inline]
    fn deserialize_enum<V>(&mut self,
                           _name: &str,
                           _variants: &'static [&'static str],
                           mut visitor: V)
                           -> DecoderResult<V::Value>
        where V: EnumVisitor
    {
        let value = match self.value.take() {
            Some(Bson::Document(value)) => value,
            Some(_) => {
                return Err(de::Error::invalid_value("expected an enum"));
            }
            None => {
                return Err(de::Error::end_of_stream());
            }
        };

        let mut iter = value.into_iter();

        let (variant, value) = match iter.next() {
            Some(v) => v,
            None => return Err(de::Error::invalid_value("expected a variant name")),
        };

        // enums are encoded in json as maps with a single key:value pair
        match iter.next() {
            Some(_) => Err(de::Error::invalid_value("expected map")),
            None => {
                visitor.visit(VariantDecoder {
                    de: self,
                    val: Some(value),
                    variant: Some(Bson::String(variant)),
                })
            }
        }
    }

    #[inline]
    fn deserialize_newtype_struct<V>(&mut self,
                                     _name: &'static str,
                                     mut visitor: V)
                                     -> DecoderResult<V::Value>
        where V: Visitor
    {
        visitor.visit_newtype_struct(self)
    }

    forward_to_deserialize!{
        deserialize_bool();
        deserialize_usize();
        deserialize_u8();
        deserialize_u16();
        deserialize_u32();
        deserialize_u64();
        deserialize_isize();
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
    }
}

struct VariantDecoder<'a> {
    de: &'a mut Decoder,
    val: Option<Bson>,
    variant: Option<Bson>,
}

impl<'a> VariantVisitor for VariantDecoder<'a> {
    type Error = DecoderError;

    fn visit_variant<V>(&mut self) -> DecoderResult<V>
        where V: Deserialize
    {
        Deserialize::deserialize(&mut Decoder::new(try!(self.variant
            .take()
            .ok_or(DecoderError::EndOfStream))))
    }

    fn visit_unit(&mut self) -> DecoderResult<()> {
        Deserialize::deserialize(&mut Decoder::new(try!(self.val
            .take()
            .ok_or(DecoderError::EndOfStream))))
    }

    fn visit_newtype<T>(&mut self) -> DecoderResult<T>
        where T: Deserialize
    {
        Deserialize::deserialize(&mut Decoder::new(try!(self.val
            .take()
            .ok_or(DecoderError::EndOfStream))))
    }

    fn visit_tuple<V>(&mut self, _len: usize, visitor: V) -> DecoderResult<V::Value>
        where V: Visitor
    {
        if let Bson::Array(fields) = try!(self.val
            .take()
            .ok_or(DecoderError::EndOfStream)) {
            Deserializer::deserialize(&mut SeqDecoder {
                                          de: self.de,
                                          len: fields.len(),
                                          iter: fields.into_iter(),
                                      },
                                      visitor)
        } else {
            Err(de::Error::invalid_value("expected a tuple"))
        }
    }

    fn visit_struct<V>(&mut self,
                       _fields: &'static [&'static str],
                       visitor: V)
                       -> DecoderResult<V::Value>
        where V: Visitor
    {
        if let Bson::Document(fields) = try!(self.val
            .take()
            .ok_or(DecoderError::EndOfStream)) {
            Deserializer::deserialize(&mut MapDecoder {
                                          de: self.de,
                                          len: fields.len(),
                                          iter: fields.into_iter(),
                                          value: None,
                                      },
                                      visitor)
        } else {
            Err(de::Error::invalid_value("expected a struct"))
        }
    }
}

struct SeqDecoder<'a> {
    de: &'a mut Decoder,
    iter: vec::IntoIter<Bson>,
    len: usize,
}

impl<'a> Deserializer for SeqDecoder<'a> {
    type Error = DecoderError;

    #[inline]
    fn deserialize<V>(&mut self, mut visitor: V) -> DecoderResult<V::Value>
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
        deserialize_usize();
        deserialize_u8();
        deserialize_u16();
        deserialize_u32();
        deserialize_u64();
        deserialize_isize();
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
    }
}

impl<'a> SeqVisitor for SeqDecoder<'a> {
    type Error = DecoderError;

    fn visit<T>(&mut self) -> DecoderResult<Option<T>>
        where T: Deserialize
    {
        match self.iter.next() {
            Some(value) => {
                self.len -= 1;
                self.de.value = Some(value);
                Ok(Some(try!(Deserialize::deserialize(self.de))))
            }
            None => Ok(None),
        }
    }

    fn end(&mut self) -> DecoderResult<()> {
        if self.len == 0 {
            Ok(())
        } else {
            Err(de::Error::invalid_length(self.len))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

struct MapDecoder<'a> {
    de: &'a mut Decoder,
    iter: OrderedDocumentIntoIterator,
    value: Option<Bson>,
    len: usize,
}

impl<'a> MapVisitor for MapDecoder<'a> {
    type Error = DecoderError;

    fn visit_key<T>(&mut self) -> DecoderResult<Option<T>>
        where T: Deserialize
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.len -= 1;
                self.value = Some(value);
                self.de.value = Some(Bson::String(key));
                match Deserialize::deserialize(self.de) {
                    Ok(val) => Ok(Some(val)),
                    Err(DecoderError::UnknownField(_)) => Ok(None),
                    Err(e) => Err(e),
                }
            }
            None => Ok(None),
        }
    }

    fn visit_value<T>(&mut self) -> DecoderResult<T>
        where T: Deserialize
    {
        let value = try!(self.value
            .take()
            .ok_or(DecoderError::EndOfStream));

        self.de.value = Some(value);
        Ok(try!(Deserialize::deserialize(self.de)))
    }

    fn end(&mut self) -> DecoderResult<()> {
        Ok(())
    }

    fn missing_field<V>(&mut self, _field: &'static str) -> DecoderResult<V>
        where V: Deserialize
    {
        // See if the type can deserialize from a unit.
        struct UnitDecoder;

        impl Deserializer for UnitDecoder {
            type Error = DecoderError;

            fn deserialize<V>(&mut self, mut visitor: V) -> DecoderResult<V::Value>
                where V: Visitor
            {
                visitor.visit_unit()
            }

            fn deserialize_option<V>(&mut self, mut visitor: V) -> DecoderResult<V::Value>
                where V: Visitor
            {
                visitor.visit_none()
            }

            forward_to_deserialize!{
                deserialize_bool();
                deserialize_usize();
                deserialize_u8();
                deserialize_u16();
                deserialize_u32();
                deserialize_u64();
                deserialize_isize();
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
                deserialize_newtype_struct(name: &'static str);
                deserialize_seq();
                deserialize_seq_fixed_size(len: usize);
                deserialize_bytes();
                deserialize_map();
                deserialize_unit_struct(name: &'static str);
                deserialize_tuple_struct(name: &'static str, len: usize);
                deserialize_struct(name: &'static str, fields: &'static [&'static str]);
                deserialize_struct_field();
                deserialize_tuple(len: usize);
                deserialize_enum(name: &'static str, variants: &'static [&'static str]);
                deserialize_ignored_any();
            }
        }

        Ok(try!(Deserialize::deserialize(&mut UnitDecoder)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a> Deserializer for MapDecoder<'a> {
    type Error = DecoderError;

    #[inline]
    fn deserialize<V>(&mut self, mut visitor: V) -> DecoderResult<V::Value>
        where V: Visitor
    {
        visitor.visit_map(self)
    }

    forward_to_deserialize!{
        deserialize_bool();
        deserialize_usize();
        deserialize_u8();
        deserialize_u16();
        deserialize_u32();
        deserialize_u64();
        deserialize_isize();
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
    }
}
