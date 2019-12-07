use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::forward_to_deserialize_any;

use super::Error;
use crate::raw::RawBsonRegexp;

pub static NAME: &str = "$__bson_Regexp";
pub static REGEXP_FIELD: &str = "$__bson_regexp_regexp";
pub static OPTIONS_FIELD: &str = "$__bson_regexp_options";
pub static FIELDS: &'static [&'static str] = &[REGEXP_FIELD, OPTIONS_FIELD];

struct RegexpKeyDeserializer {
    key: &'static str,
}

impl RegexpKeyDeserializer {
    fn new(key: &'static str) -> RegexpKeyDeserializer {
        RegexpKeyDeserializer { key }
    }
}

impl<'de> Deserializer<'de> for RegexpKeyDeserializer {
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

pub(super) struct RegexpDeserializer<'de> {
    data: RawBsonRegexp<'de>,
    visiting: Visiting,
}

impl<'de> RegexpDeserializer<'de> {
    pub(super) fn new(data: RawBsonRegexp<'de>) -> RegexpDeserializer<'de> {
        RegexpDeserializer {
            data,
            visiting: Visiting::Regexp,
        }
    }
}

impl<'de> Deserializer<'de> for RegexpDeserializer<'de> {
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
    Regexp,
    Options,
    Done,
}

impl<'de> SeqAccess<'de> for RegexpDeserializer<'de> {
    type Error = Error;

    fn next_element_seed<E>(&mut self, seed: E) -> Result<Option<E::Value>, Error>
        where
            E: DeserializeSeed<'de>,
    {
        match self.visiting {
            Visiting::Regexp => {
                self.visiting = Visiting::Options;
                seed.deserialize(RegexpFieldDeserializer::new(self.data.pattern()))
                    .map(Some)
            }
            Visiting::Options => {
                self.visiting = Visiting::Done;
                seed.deserialize(RegexpFieldDeserializer::new(self.data.options()))
                    .map(Some)
            }
            Visiting::Done => Ok(None),
        }
    }
}

impl<'de> MapAccess<'de> for RegexpDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
        where
            K: DeserializeSeed<'de>,
    {
        match self.visiting {
            Visiting::Regexp => seed.deserialize(RegexpKeyDeserializer::new(REGEXP_FIELD)).map(Some),
            Visiting::Options => seed.deserialize(RegexpKeyDeserializer::new(OPTIONS_FIELD)).map(Some),
            Visiting::Done => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
        where
            V: DeserializeSeed<'de>,
    {
        match self.visiting {
            Visiting::Regexp => {
                self.visiting = Visiting::Options;
                seed.deserialize(RegexpFieldDeserializer::new(self.data.pattern()))
            }
            Visiting::Options => {
                self.visiting = Visiting::Done;
                seed.deserialize(RegexpFieldDeserializer::new(self.data.options()))
            }
            Visiting::Done => Err(Error::MalformedDocument),
        }
    }
}

struct RegexpFieldDeserializer<'de> {
    data: &'de str,
}

impl<'de> RegexpFieldDeserializer<'de> {
    fn new(data: &'de str) -> RegexpFieldDeserializer<'de> {
        RegexpFieldDeserializer { data }
    }
}

impl<'de> Deserializer<'de> for RegexpFieldDeserializer<'de> {
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
