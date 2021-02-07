use serde::de::{DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};
use serde::forward_to_deserialize_any;

use crate::raw::Doc;
use super::{BsonDeserializer, Error};

pub static NAME: &str = "$__bson_JavaScript";
pub static WITH_SCOPE_NAME: &str = "$__bson_JavaScriptWithScope";
pub static DATA_FIELD: &str = "$__bson_javascript_data";
pub static SCOPE_FIELD: &str = "$__bson_javascript_scope";
pub static FIELDS: &[&str] = &[DATA_FIELD];
pub static WITH_SCOPE_FIELDS: &[&str] = &[DATA_FIELD, SCOPE_FIELD];

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
    scope: &'de Doc,
    visiting: ScopedVisiting,
}

impl<'de> JavaScriptWithScopeDeserializer<'de> {
    pub(super) fn new<D: AsRef<Doc> + ?Sized>(
        data: (&'de str, &'de D),
    ) -> JavaScriptWithScopeDeserializer<'de> {
        JavaScriptWithScopeDeserializer {
            js: data.0,
            scope: data.1.as_ref(),
            visiting: ScopedVisiting::Js,
        }
    }
}

impl<'de> Deserializer<'de> for JavaScriptWithScopeDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Error>
    where
        V: Visitor<'de>,
    {
        self.deserialize_seq(visitor)
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
        if ct != 2 {
            Err(Error::MalformedDocument)
        } else {
            visitor.visit_seq(self)
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
        option unit newtype_struct
        ignored_any unit_struct tuple_struct enum identifier
    );
}

enum ScopedVisiting {
    Js,
    Scope,
    Done,
}

impl<'de> SeqAccess<'de> for JavaScriptWithScopeDeserializer<'de> {
    type Error = Error;

    fn next_element_seed<E>(&mut self, seed: E) -> Result<Option<E::Value>, Error>
    where
        E: DeserializeSeed<'de>,
    {
        match self.visiting {
            ScopedVisiting::Js => {
                self.visiting = ScopedVisiting::Scope;
                seed.deserialize(JavaScriptWithScopeJsDeserializer::new(self.js))
                    .map(Some)
            }
            ScopedVisiting::Scope => {
                self.visiting = ScopedVisiting::Done;
                seed.deserialize(&mut BsonDeserializer::from_doc(&self.scope))
                    .map(Some)
            }
            ScopedVisiting::Done => Ok(None),
        }
    }
}

impl<'de> MapAccess<'de> for JavaScriptWithScopeDeserializer<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Error>
    where
        K: DeserializeSeed<'de>,
    {
        match self.visiting {
            ScopedVisiting::Js => seed
                .deserialize(JavaScriptKeyDeserializer::new(DATA_FIELD))
                .map(Some),
            ScopedVisiting::Scope => seed
                .deserialize(JavaScriptKeyDeserializer::new(SCOPE_FIELD))
                .map(Some),
            ScopedVisiting::Done => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Error>
    where
        V: DeserializeSeed<'de>,
    {
        match self.visiting {
            ScopedVisiting::Js => {
                self.visiting = ScopedVisiting::Scope;
                seed.deserialize(JavaScriptWithScopeJsDeserializer::new(self.js))
            }
            ScopedVisiting::Scope => {
                self.visiting = ScopedVisiting::Done;
                seed.deserialize(&mut BsonDeserializer::from_doc(self.scope))
            }
            ScopedVisiting::Done => Err(Error::MalformedDocument),
        }
    }
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
