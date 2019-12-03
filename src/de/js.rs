use serde::de::{DeserializeSeed, Deserializer, MapAccess, Visitor};
use serde::forward_to_deserialize_any;

use super::Error;
use crate::de::BsonDeserializer;
use crate::raw::RawBsonDoc;

pub static NAME: &str = "$__bson_JavaScript";
pub static WITH_SCOPE_NAME: &str = "$__bson_JavaScriptWithScope";
pub static DATA_FIELD: &str = "$__bson_javascript_data";
pub static SCOPE_FIELD: &str = "$__bson_javascript_scope";
pub static FIELDS: &'static [&'static str] = &[DATA_FIELD];
pub static WITH_SCOPE_FIELDS: &'static [&'static str] = &[DATA_FIELD, SCOPE_FIELD];

struct JavaScriptKeyDeserializer {
    key: &'static str,
}

impl JavaScriptKeyDeserializer {
    fn new(key: &'static str) -> JavaScriptKeyDeserializer {
        JavaScriptKeyDeserializer { key }
    }
}

impl<'de> Deserializer<'de> for JavaScriptKeyDeserializer {
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

pub(super) struct JavaScriptWithScopeDeserializer<'de> {
    js: &'de str,
    scope: RawBsonDoc<'de>,
    visiting: ScopedVisiting,
}

impl<'de> JavaScriptWithScopeDeserializer<'de> {
    pub(super) fn new(data: (&'de str, RawBsonDoc<'de>)) -> JavaScriptWithScopeDeserializer<'de> {
        JavaScriptWithScopeDeserializer {
            js: data.0,
            scope: data.1,
            visiting: ScopedVisiting::New,
        }
    }
}

impl<'de> Deserializer<'de> for JavaScriptWithScopeDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_str(visitor)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.js)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_str(self.js)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
        visitor.visit_map(self)
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        name: &str,
        fields: &[&str],
        visitor: V,
    ) -> Result<V::Value, Self::Error> {
        if name == NAME {
            visitor.visit_map(self)
        } else {
            Err(Error::MalformedDocument)
        }
    }

    forward_to_deserialize_any!(
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char bytes byte_buf seq
        option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    );
}

enum ScopedVisiting {
    New,
    Data,
    Scope,
    Done,
}

impl<'de> MapAccess<'de> for JavaScriptWithScopeDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.visiting {
            ScopedVisiting::New => {
                self.visiting = ScopedVisiting::Data;
                seed.deserialize(JavaScriptKeyDeserializer::new(DATA_FIELD)).map(Some)
            }
            ScopedVisiting::Data => {
                self.visiting = ScopedVisiting::Scope;
                seed.deserialize(JavaScriptKeyDeserializer::new(SCOPE_FIELD)).map(Some)
            }
            ScopedVisiting::Scope => {
                self.visiting = ScopedVisiting::Done;
                Ok(None)
            }
            ScopedVisiting::Done => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        match self.visiting {
            ScopedVisiting::Data => seed.deserialize(JavaScriptWithScopeJsDeserializer::new(self.js)),
            ScopedVisiting::Scope => seed.deserialize(&mut BsonDeserializer::from_rawdoc(self.scope)),
            _ => Err(Error::MalformedDocument),
        }
    }
}

struct JavaScriptWithScopeKeyDeserializer {
    key: &'static str,
}

impl JavaScriptWithScopeKeyDeserializer {
    fn new(key: &'static str) -> JavaScriptWithScopeKeyDeserializer {
        JavaScriptWithScopeKeyDeserializer { key }
    }
}

impl<'de> Deserializer<'de> for JavaScriptWithScopeKeyDeserializer {
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

struct JavaScriptWithScopeJsDeserializer<'de> {
    data: &'de str,
}

impl<'de> JavaScriptWithScopeJsDeserializer<'de> {
    fn new(data: &'de str) -> JavaScriptWithScopeJsDeserializer<'de> {
        JavaScriptWithScopeJsDeserializer { data }
    }
}

impl<'de> Deserializer<'de> for JavaScriptWithScopeJsDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.data)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>
    {
        visitor.visit_borrowed_str(self.data)
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>
    {
        visitor.visit_str(self.data)
    }

    forward_to_deserialize_any!(
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    );
}
