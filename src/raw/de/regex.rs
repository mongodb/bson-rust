use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::forward_to_deserialize_any;

use super::Error;
use crate::raw::elem::RawBsonRegex;

pub static NAME: &str = "$__bson_Regex";
pub static REGEXP_FIELD: &str = "$__bson_regexp_regexp";
pub static OPTIONS_FIELD: &str = "$__bson_regexp_options";
pub static FIELDS: &[&str] = &[REGEXP_FIELD, OPTIONS_FIELD];

struct RegexKeyDeserializer {
    key: &'static str,
}

impl RegexKeyDeserializer {
    fn new(key: &'static str) -> RegexKeyDeserializer {
        RegexKeyDeserializer { key }
    }
}

impl<'de> Deserializer<'de> for RegexKeyDeserializer {
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

pub(super) struct RegexDeserializer<'de> {
    data: RawBsonRegex<'de>,
    visiting: Visiting,
}

impl<'de> RegexDeserializer<'de> {
    pub(super) fn new(data: RawBsonRegex<'de>) -> RegexDeserializer<'de> {
        RegexDeserializer {
            data,
            visiting: Visiting::Regex,
        }
    }
}

impl<'de> Deserializer<'de> for RegexDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_seq(self)
    }

    fn deserialize_tuple<V>(self, ct: usize, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        if ct == 2 {
            visitor.visit_seq(self)
        } else {
            Err(Error::MalformedDocument)
        }
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
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char bytes byte_buf
        option unit newtype_struct str string
        ignored_any unit_struct tuple_struct enum identifier
    );
}

enum Visiting {
    Regex,
    Options,
    Done,
}

impl<'de> SeqAccess<'de> for RegexDeserializer<'de> {
    type Error = Error;

    fn next_element_seed<E>(&mut self, seed: E) -> Result<Option<E::Value>, Error>
    where
        E: DeserializeSeed<'de>,
    {
        match self.visiting {
            Visiting::Regex => {
                self.visiting = Visiting::Options;
                seed.deserialize(RegexFieldDeserializer::new(self.data.pattern()))
                    .map(Some)
            }
            Visiting::Options => {
                self.visiting = Visiting::Done;
                seed.deserialize(RegexFieldDeserializer::new(self.data.options()))
                    .map(Some)
            }
            Visiting::Done => Ok(None),
        }
    }
}

impl<'de> MapAccess<'de> for RegexDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.visiting {
            Visiting::Regex => seed
                .deserialize(RegexKeyDeserializer::new(REGEXP_FIELD))
                .map(Some),
            Visiting::Options => seed
                .deserialize(RegexKeyDeserializer::new(OPTIONS_FIELD))
                .map(Some),
            Visiting::Done => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        match self.visiting {
            Visiting::Regex => {
                self.visiting = Visiting::Options;
                seed.deserialize(RegexFieldDeserializer::new(self.data.pattern()))
            }
            Visiting::Options => {
                self.visiting = Visiting::Done;
                seed.deserialize(RegexFieldDeserializer::new(self.data.options()))
            }
            Visiting::Done => Err(Error::MalformedDocument),
        }
    }
}

struct RegexFieldDeserializer<'de> {
    data: &'de str,
}

impl<'de> RegexFieldDeserializer<'de> {
    fn new(data: &'de str) -> RegexFieldDeserializer<'de> {
        RegexFieldDeserializer { data }
    }
}

impl<'de> Deserializer<'de> for RegexFieldDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.data)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.data)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.data)
    }

    forward_to_deserialize_any!(
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    );
}
