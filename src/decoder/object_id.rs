use std::fmt;

use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
use serde::export::fmt::Error;
use serde::export::Formatter;

use crate::de::object_id;

use crate::oid::ObjectId;
use std::convert::TryInto;

impl<'de> Deserialize<'de> for ObjectId {
    fn deserialize<D>(deserializer: D) -> Result<ObjectId, D::Error>
        where
            D: Deserializer<'de>,
    {
        struct ObjectIdVisitor;

        impl<'de> Visitor<'de> for ObjectIdVisitor {
            type Value = ObjectId;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a bson objectid")
            }

            fn visit_map<V>(self, mut map: V) -> Result<ObjectId, V::Error>
                where
                    V: MapAccess<'de>,
            {
                let value = map.next_key::<ObjectIdKey>()?;
                if value.is_none() {
                    return Err(serde::de::Error::custom(
                        "No ObjectIdKey not found in synthesized struct",
                    ));
                }
                let v: ObjectIdFromBytes = map.next_value()?;
                Ok(v.0)
            }
        }

        deserializer.deserialize_struct(object_id::NAME, object_id::FIELDS, ObjectIdVisitor)
    }
}

struct ObjectIdKey;

impl<'de> Deserialize<'de> for ObjectIdKey {
    fn deserialize<D>(deserializer: D) -> Result<ObjectIdKey, D::Error>
        where
            D: Deserializer<'de>,
    {
        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = ();

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a valid object id field")
            }

            fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<(), E> {
                if s == object_id::FIELD {
                    Ok(())
                } else {
                    Err(serde::de::Error::custom(
                        "field was not $__bson_object_id in synthesized object id struct",
                    ))
                }
            }
        }

        deserializer.deserialize_identifier(FieldVisitor)?;
        Ok(ObjectIdKey)
    }
}

struct ObjectIdFromBytes(ObjectId);

impl<'de> Deserialize<'de> for ObjectIdFromBytes {
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
        where
            D: Deserializer<'de>,
    {
        struct FieldVisitor;

        impl<'de> Visitor<'de> for FieldVisitor {
            type Value = ObjectIdFromBytes;

            fn expecting(&self, formatter: &mut Formatter<'_>) -> Result<(), Error> {
                formatter.write_str("an object id of twelve bytes")
            }

            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                Ok(ObjectIdFromBytes(ObjectId::with_bytes(v.try_into().map_err(serde::de::Error::custom)?)))
            }
        }

        deserializer.deserialize_bytes(FieldVisitor)
    }
}
