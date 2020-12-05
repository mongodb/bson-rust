//! Collection of helper functions for serializing to and deserializing from BSON using Serde

use std::{convert::TryFrom, result::Result, str::FromStr};

use chrono::{DateTime, Utc};
use serde::{de, ser, Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

use crate::{oid::ObjectId, spec::BinarySubtype, Binary, Bson, Timestamp};

/// Attempts to serialize a u32 as an i32. Errors if an exact conversion is not possible.
pub fn serialize_u32_as_i32<S: Serializer>(val: &u32, serializer: S) -> Result<S::Ok, S::Error> {
    match i32::try_from(*val) {
        Ok(val) => serializer.serialize_i32(val),
        Err(_) => Err(ser::Error::custom(format!("cannot convert {} to i32", val))),
    }
}

/// Serializes a u32 as an i64.
pub fn serialize_u32_as_i64<S: Serializer>(val: &u32, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_i64(*val as i64)
}

/// Attempts to serialize a u64 as an i32. Errors if an exact conversion is not possible.
pub fn serialize_u64_as_i32<S: Serializer>(val: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    match i32::try_from(*val) {
        Ok(val) => serializer.serialize_i32(val),
        Err(_) => Err(ser::Error::custom(format!("cannot convert {} to i32", val))),
    }
}

/// Attempts to serialize a u64 as an i64. Errors if an exact conversion is not possible.
pub fn serialize_u64_as_i64<S: Serializer>(val: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    match i64::try_from(*val) {
        Ok(val) => serializer.serialize_i64(val),
        Err(_) => Err(ser::Error::custom(format!("cannot convert {} to i64", val))),
    }
}

/// Deserializes a chrono::DateTime<Utc> from the extended JSON representation of DateTime.
pub fn deserialize_chrono_datetime_from_ext_json<'de, D>(
    deserializer: D,
) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    let bson = Bson::deserialize(deserializer)?;
    match bson {
        Bson::DateTime(date) => Ok(date),
        _ => Err(de::Error::custom(
            "cannot convert extended JSON to DateTime",
        )),
    }
}

/// Deserializes a DateTime from the extended JSON representation.
pub fn deserialize_bson_datetime_from_ext_json<'de, D>(
    deserializer: D,
) -> Result<crate::DateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let bson = Bson::deserialize(deserializer)?;
    match bson {
        Bson::DateTime(date) => Ok(crate::DateTime(date)),
        _ => Err(de::Error::custom(
            "cannot convert extended JSON to DateTime",
        )),
    }
}

/// Deserializes an ISO string from a DateTime.
pub fn deserialize_iso_string_from_datetime<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let date = crate::DateTime::deserialize(deserializer)?;
    Ok(date.to_string())
}

/// Serializes an ISO string as a DateTime.
pub fn serialize_iso_string_as_datetime<S: Serializer>(
    val: &str,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let date = DateTime::from_str(val)
        .map_err(|_| ser::Error::custom(format!("cannot convert {} to DateTime", val)))?;
    Bson::DateTime(date).serialize(serializer)
}

/// Deserializes an ObjectId from the extended JSON representation.
pub fn deserialize_object_id_from_ext_json<'de, D>(deserializer: D) -> Result<ObjectId, D::Error>
where
    D: Deserializer<'de>,
{
    let bson = Bson::deserialize(deserializer)?;
    match bson {
        Bson::ObjectId(oid) => Ok(oid),
        _ => Err(de::Error::custom(
            "cannot convert extended JSON to ObjectId",
        )),
    }
}

/// Serializes a hex string as a ObjectId.
pub fn serialize_hex_string_as_object_id<S: Serializer>(
    val: &str,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match ObjectId::with_string(val) {
        Ok(oid) => oid.serialize(serializer),
        Err(_) => Err(ser::Error::custom(format!(
            "cannot convert {} to ObjectId",
            val
        ))),
    }
}

/// Serializes a Uuid as a Binary.
pub fn serialize_uuid_as_binary<S: Serializer>(
    val: &Uuid,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let binary = Binary {
        subtype: BinarySubtype::Uuid,
        bytes: val.as_bytes().to_vec(),
    };
    binary.serialize(serializer)
}

/// Deserializes a Uuid from a Binary.
pub fn deserialize_uuid_from_binary<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
where
    D: Deserializer<'de>,
{
    let binary = Binary::deserialize(deserializer)?;
    if binary.subtype == BinarySubtype::Uuid {
        if binary.bytes.len() == 16 {
            let mut bytes = [0u8; 16];
            for i in 0..16 {
                bytes[i] = binary.bytes[i];
            }
            Ok(Uuid::from_bytes(bytes))
        } else {
            Err(de::Error::custom(
                "cannot convert Binary to Uuid: incorrect bytes length",
            ))
        }
    } else {
        Err(de::Error::custom(
            "cannot convert Binary to Uuid: incorrect binary subtype",
        ))
    }
}

/// Serializes a u64 as a Timestamp.
pub fn serialize_u64_as_timestamp<S: Serializer>(
    val: &u64,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    let time = (*val >> 32) as u32;
    let increment = *val as u32;
    let timestamp = Bson::Timestamp(Timestamp { time, increment });
    timestamp.serialize(serializer)
}

/// Deserializes a u64 from a bson::Timestamp.
pub fn deserialize_u64_from_timestamp<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let timestamp = Timestamp::deserialize(deserializer)?;
    let time = (timestamp.time as u64) << 32;
    Ok(time + timestamp.increment as u64)
}
