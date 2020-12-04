use std::{convert::TryFrom, result::Result, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{de, ser, Deserialize, Deserializer, Serialize, Serializer};
use uuid::{Bytes, Uuid};

use crate::{Binary, Bson, Document, oid::ObjectId, spec::BinarySubtype, Timestamp};

pub fn serialize_u32_as_i32<S: Serializer>(
    val: u32,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match i32::try_from(val) {
        Ok(val) => serializer.serialize_i32(val),
        Err(_) => Err(ser::Error::custom(format!("cannot convert {} to i32", val))),
    }
}

pub fn serialize_u32_as_i64<S: Serializer>(
    val: u32,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.serialize_i64(val as i64)
}

pub fn serialize_u64_as_i32<S: Serializer>(
    val: u64,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match i32::try_from(val) {
        Ok(val) => serializer.serialize_i32(val),
        Err(_) => Err(ser::Error::custom(format!("cannot convert {} to i32", val))),
    }
}

pub fn serialize_u64_as_i64<S: Serializer>(
    val: u64,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match i64::try_from(val) {
        Ok(val) => serializer.serialize_i64(val),
        Err(_) => Err(ser::Error::custom(format!("cannot convert {} to i64", val))),
    }
}

pub fn deserialize_datetime_from_ext_json<'de, D>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let doc = Document::deserialize(deserializer)?;
    match Bson::from_extended_document(doc) {
        Bson::DateTime(date) => Ok(date),
        _ => Err(de::Error::custom("cannot convert extended JSON to DateTime")),
    }
}

pub fn deserialize_iso_string_from_datetime<'de, D>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let str = String::deserialize(deserializer)?;
    DateTime::from_str(&str).map_err(|_| de::Error::custom(format!("cannot convert {} to DateTime", str)))
}

pub fn serialize_iso_string_as_datetime<S: Serializer>(
    val: String,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let date = DateTime::from_str(&val).map_err(|_| ser::Error::custom(format!("cannot convert {} to DateTime", val)))?;
    Bson::DateTime(date).serialize(serializer)
}

pub fn deserialize_object_id_from_ext_json<'de, D>(
    deserializer: D,
) -> Result<ObjectId, D::Error>
where
    D: Deserializer<'de>,
{
    let doc = Document::deserialize(deserializer)?;
    match Bson::from_extended_document(doc.clone()) {
        Bson::ObjectId(oid) => Ok(oid),
        _ => Err(de::Error::custom("cannot convert extended JSON to ObjectId")),
    }
}

pub fn serialize_hex_string_as_object_id<S: Serializer>(
    val: String,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match ObjectId::with_string(&val) {
        Ok(oid) => oid.serialize(serializer),
        Err(_) => Err(ser::Error::custom(format!("cannot convert {} to ObjectId", val)))
    }
}

pub fn serialize_uuid_as_binary<S: Serializer>(
    val: Uuid,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let binary = Binary { subtype: BinarySubtype::Uuid, bytes: val.as_bytes().to_vec() };
    binary.serialize(serializer)
}

pub fn deserialize_uuid_from_binary<'de, D>(
    deserializer: D,
) -> Result<Uuid, D::Error>
where
    D: Deserializer<'de>,
{
    let binary = Binary::deserialize(deserializer)?;
    if binary.subtype == BinarySubtype::Uuid {
        match Bytes::try_from(binary.bytes) {
            Ok(bytes) => Ok(Uuid::from_bytes(bytes)),
            Err(_) => Err(de::Error::custom("cannot convert Binary to Uuid: incorrect bytes length"))
        }
    } else {
        Err(de::Error::custom("cannot convert Binary to Uuid: incorrect binary subtype"))
    }
}

pub fn serialize_u64_as_timestamp<S: Serializer>(
    val: u64,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let time = (val >> 32) as u32;
    let increment = val as u32;
    let timestamp = Bson::Timestamp(Timestamp { time, increment });
    timestamp.serialize(serializer)
}

pub fn deserialize_u64_from_timestamp<'de, D>(
    deserializer: D,
) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let timestamp = Timestamp::deserialize(deserializer)?;
    let time = (timestamp.time as u64) << 32;
    Ok(time + timestamp.increment as u64)
}
