use std::convert::TryFrom;

use serde::de::{DeserializeSeed, Deserializer, MapAccess, Visitor};
use serde::forward_to_deserialize_any;

use super::Error;

pub static NAME: &str = "$__bson_UtcDateTime";
pub static FIELD: &str = "$__bson_utcdatetime";
pub static FIELDS: &'static [&'static str] = &[FIELD];

struct UtcDateTimeKeyDeserializer {
    key: &'static str,
}

impl UtcDateTimeKeyDeserializer {
    fn new(key: &'static str) -> UtcDateTimeKeyDeserializer {
        UtcDateTimeKeyDeserializer { key }
    }
}

impl<'de> Deserializer<'de> for UtcDateTimeKeyDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.key)
    }

    forward_to_deserialize_any!(
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    );
}

pub struct UtcDateTimeDeserializer {
    data: i64,
    visited: bool,
}

impl UtcDateTimeDeserializer {
    pub fn new(data: i64) -> UtcDateTimeDeserializer {
        UtcDateTimeDeserializer { data, visited: false }
    }
}

impl<'de> Deserializer<'de> for UtcDateTimeDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_struct(NAME, FIELDS, visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.data)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_u64(TryFrom::try_from(self.data).map_err(|err| Error::MalformedDocument)?)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_map(self)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &str,
        _fields: &[&str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if name == NAME {
            visitor.visit_map(self)
        } else {
            Err(Error::MalformedDocument)
        }
    }

    forward_to_deserialize_any!(
        bool u8 u16 u32 i8 i16 i32 f32 f64 char bytes byte_buf
        option unit newtype_struct str string tuple
        ignored_any seq unit_struct tuple_struct enum identifier
    );
}

impl<'de> MapAccess<'de> for UtcDateTimeDeserializer {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.visited {
            false => seed.deserialize(UtcDateTimeKeyDeserializer::new(FIELD)).map(Some),
            true => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        match self.visited {
            false => {
                self.visited = true;
                seed.deserialize(UtcDateTimeFieldDeserializer::new(self.data))
            }
            true => Err(Error::MalformedDocument),
        }
    }
}

struct UtcDateTimeFieldDeserializer {
    data: i64,
}

impl<'de> UtcDateTimeFieldDeserializer {
    fn new(data: i64) -> UtcDateTimeFieldDeserializer {
        UtcDateTimeFieldDeserializer { data }
    }
}

impl<'de> Deserializer<'de> for UtcDateTimeFieldDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_i64(visitor)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_i64(self.data)
    }

    forward_to_deserialize_any!(
        bool u8 u16 u32 u64 i8 i16 i32 f32 f64 char seq
        bytes byte_buf str string map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    );
}
