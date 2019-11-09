// ObjectId handling

use serde::de::{DeserializeSeed, Deserializer, MapAccess, Visitor};
use serde::forward_to_deserialize_any;

use super::Error;
use crate::raw::RawBson;
use crate::spec::ElementType;

pub static FIELD: &str = "$__bson_object_id";
pub static FIELDS: &[&str] = &[FIELD];
pub static NAME: &str = "$__bson_ObjectId";

pub struct ObjectIdDeserializer<'de> {
    bson: RawBson<'de>,
    visited: bool,
}

impl<'de> ObjectIdDeserializer<'de> {
    pub fn new(bson: RawBson<'de>) -> ObjectIdDeserializer<'de> {
        ObjectIdDeserializer { bson, visited: false }
    }
}

impl<'de> Deserializer<'de> for ObjectIdDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::ObjectId => self.deserialize_struct(NAME, FIELDS, visitor),
            _ => Err(Error::MalformedDocument),
        }
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        match self.bson.element_type() {
            ElementType::ObjectId => visitor.visit_bytes(self.bson.as_bytes()),
            _ => Err(Error::MalformedDocument),
        }
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_map(self)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if name == NAME && fields == FIELDS {
            visitor.visit_map(self)
        } else {
            Err(Error::MalformedDocument)
        }
    }

    forward_to_deserialize_any!(
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        byte_buf option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    );
}
impl<'de> MapAccess<'de> for ObjectIdDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<<K as DeserializeSeed<'de>>::Value>, Self::Error>
    where
        K: DeserializeSeed<'de>,
    {
        println!("object id next key");
        if self.visited {
            Ok(None)
        } else {
            self.visited = true;
            seed.deserialize(ObjectIdKeyDeserializer).map(Some)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<<V as DeserializeSeed<'de>>::Value, Self::Error>
    where
        V: DeserializeSeed<'de>,
    {
        println!("object id next value");

        seed.deserialize(ObjectIdValueDeserializer::new(self.bson))
    }
}

struct ObjectIdKeyDeserializer;

impl<'de> Deserializer<'de> for ObjectIdKeyDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(FIELD)
    }

    forward_to_deserialize_any!(
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    );
}

struct ObjectIdValueDeserializer<'de>(RawBson<'de>);

impl<'de> ObjectIdValueDeserializer<'de> {
    fn new(bson: RawBson<'de>) -> ObjectIdValueDeserializer<'de> {
        ObjectIdValueDeserializer(bson)
    }
}

impl<'de> Deserializer<'de> for ObjectIdValueDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        match self.0.element_type() {
            ElementType::ObjectId => visitor.visit_borrowed_bytes(self.0.as_bytes()),
            _ => Err(Error::MalformedDocument),
        }
    }

    forward_to_deserialize_any!(
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    );
}
