//! Collection of helper functions for serializing to and deserializing from BSON using Serde

use std::{convert::TryFrom, result::Result};

use serde::{de, ser, Deserialize, Deserializer, Serialize, Serializer};

use crate::{oid::ObjectId, Bson};

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

pub mod chrono_datetime_to_bson_datetime {
    use crate::{Bson, DateTime};
    use chrono::Utc;
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
    use std::result::Result;

    /// Deserializes a chrono::DateTime<Utc> from the extended JSON representation of DateTime.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<chrono::DateTime<Utc>, D::Error>
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

    /// Serializes a chrono::DateTime as a bson::DateTime.
    pub fn serialize<S: Serializer>(
        val: &chrono::DateTime<Utc>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let datetime = DateTime::from(val.to_owned());
        datetime.serialize(serializer)
    }
}

pub mod iso_string_to_bson_datetime {
    use crate::{Bson, DateTime};
    use serde::{ser, Deserialize, Deserializer, Serialize, Serializer};
    use std::{result::Result, str::FromStr};

    /// Deserializes an ISO string from a DateTime.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let date = DateTime::deserialize(deserializer)?;
        Ok(date.to_string())
    }

    /// Serializes an ISO string as a DateTime.
    pub fn serialize<S: Serializer>(val: &str, serializer: S) -> Result<S::Ok, S::Error> {
        let date = chrono::DateTime::from_str(val).map_err(|_| {
            ser::Error::custom(format!("cannot convert {} to chrono::DateTime", val))
        })?;
        Bson::DateTime(date).serialize(serializer)
    }
}

pub mod bson_datetime_to_iso_string {
    use crate::DateTime;
    use serde::{de, Deserialize, Deserializer, Serializer};
    use std::{result::Result, str::FromStr};

    /// Deserializes a bson::DateTime from an ISO string.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let iso = String::deserialize(deserializer)?;
        let date = chrono::DateTime::from_str(&iso).map_err(|_| {
            de::Error::custom(format!("cannot convert {} to chrono::DateTime", iso))
        })?;
        Ok(DateTime::from(date))
    }

    /// Serializes a bson::DateTime as an ISO string.
    pub fn serialize<S: Serializer>(val: &DateTime, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&val.to_string())
    }
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

/// Serializes a hex string as an ObjectId.
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

pub mod uuid_to_binary {
    use crate::{spec::BinarySubtype, Binary};
    use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
    use std::result::Result;
    use uuid::Uuid;

    /// Serializes a Uuid as a Binary.
    pub fn serialize<S: Serializer>(val: &Uuid, serializer: S) -> Result<S::Ok, S::Error> {
        let binary = Binary {
            subtype: BinarySubtype::Uuid,
            bytes: val.as_bytes().to_vec(),
        };
        binary.serialize(serializer)
    }

    /// Deserializes a Uuid from a Binary.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
    where
        D: Deserializer<'de>,
    {
        let binary = Binary::deserialize(deserializer)?;
        if binary.subtype == BinarySubtype::Uuid {
            if binary.bytes.len() == 16 {
                let mut bytes = [0u8; 16];
                bytes.copy_from_slice(&binary.bytes);
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
}

pub mod u64_to_timestamp {
    use crate::{Bson, Timestamp};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::result::Result;

    /// Serializes a u64 as a Timestamp.
    pub fn serialize<S: Serializer>(val: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        let time = (*val >> 32) as u32;
        let increment = *val as u32;
        let timestamp = Bson::Timestamp(Timestamp { time, increment });
        timestamp.serialize(serializer)
    }

    /// Deserializes a u64 from a bson::Timestamp.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp = Timestamp::deserialize(deserializer)?;
        let time = (timestamp.time as u64) << 32;
        Ok(time + timestamp.increment as u64)
    }
}

pub mod timestamp_to_u64 {
    use crate::Timestamp;
    use serde::{Deserialize, Deserializer, Serializer};
    use std::result::Result;

    /// Serializes a bson::Timestamp as a u64.
    pub fn serialize<S: Serializer>(val: &Timestamp, serializer: S) -> Result<S::Ok, S::Error> {
        let time = (val.time as u64) << 32;
        serializer.serialize_u64(time + val.increment as u64)
    }

    /// Deserializes a bson::Timestamp from a u64.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Timestamp, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp = u64::deserialize(deserializer)?;
        let time = (timestamp >> 32) as u32;
        let increment = timestamp as u32;
        Ok(Timestamp { time, increment })
    }
}
