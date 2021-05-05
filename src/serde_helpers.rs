//! Collection of helper functions for serializing to and deserializing from BSON using Serde

use std::{convert::TryFrom, result::Result};

use serde::{ser, Serializer};

pub use bson_datetime_as_iso_string::{
    deserialize as deserialize_bson_datetime_from_iso_string,
    serialize as serialize_bson_datetime_as_iso_string,
};
pub use chrono_datetime_as_bson_datetime::{
    deserialize as deserialize_chrono_datetime_from_bson_datetime,
    serialize as serialize_chrono_datetime_as_bson_datetime,
};
pub use hex_string_as_object_id::{
    deserialize as deserialize_hex_string_from_object_id,
    serialize as serialize_hex_string_as_object_id,
};
pub use iso_string_as_bson_datetime::{
    deserialize as deserialize_iso_string_from_bson_datetime,
    serialize as serialize_iso_string_as_bson_datetime,
};
pub use timestamp_as_u32::{
    deserialize as deserialize_timestamp_from_u32,
    serialize as serialize_timestamp_as_u32,
};
pub use u32_as_f64::{deserialize as deserialize_u32_from_f64, serialize as serialize_u32_as_f64};
pub use u32_as_timestamp::{
    deserialize as deserialize_u32_from_timestamp,
    serialize as serialize_u32_as_timestamp,
};
pub use u64_as_f64::{deserialize as deserialize_u64_from_f64, serialize as serialize_u64_as_f64};
pub use uuid_as_binary::{
    deserialize as deserialize_uuid_from_binary,
    serialize as serialize_uuid_as_binary,
};

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

/// Contains functions to serialize a u32 as an f64 (BSON double) and deserialize a
/// u32 from an f64 (BSON double).
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use bson::serde_helpers::u32_as_f64;
/// #[derive(Serialize, Deserialize)]
/// struct FileInfo {
///     #[serde(with = "u32_as_f64")]
///     pub size_bytes: u32,
/// }
/// ```
pub mod u32_as_f64 {
    use serde::{de, Deserialize, Deserializer, Serializer};

    /// Deserializes a u32 from an f64 (BSON double). Errors if an exact conversion is not possible.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let f = f64::deserialize(deserializer)?;
        if (f - f as u32 as f64).abs() <= f64::EPSILON {
            Ok(f as u32)
        } else {
            Err(de::Error::custom(format!(
                "cannot convert f64 (BSON double) {} to u32",
                f
            )))
        }
    }

    /// Serializes a u32 as an f64 (BSON double).
    pub fn serialize<S: Serializer>(val: &u32, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(*val as f64)
    }
}

/// Contains functions to serialize a u64 as an f64 (BSON double) and deserialize a
/// u64 from an f64 (BSON double).
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use bson::serde_helpers::u64_as_f64;
/// #[derive(Serialize, Deserialize)]
/// struct FileInfo {
///     #[serde(with = "u64_as_f64")]
///     pub size_bytes: u64,
/// }
/// ```
pub mod u64_as_f64 {
    use serde::{de, ser, Deserialize, Deserializer, Serializer};

    /// Deserializes a u64 from an f64 (BSON double). Errors if an exact conversion is not possible.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let f = f64::deserialize(deserializer)?;
        if (f - f as u64 as f64).abs() <= f64::EPSILON {
            Ok(f as u64)
        } else {
            Err(de::Error::custom(format!(
                "cannot convert f64 (BSON double) {} to u64",
                f
            )))
        }
    }

    /// Serializes a u64 as an f64 (BSON double). Errors if an exact conversion is not possible.
    pub fn serialize<S: Serializer>(val: &u64, serializer: S) -> Result<S::Ok, S::Error> {
        if val < &u64::MAX && *val == *val as f64 as u64 {
            serializer.serialize_f64(*val as f64)
        } else {
            Err(ser::Error::custom(format!(
                "cannot convert u64 {} to f64 (BSON double)",
                val
            )))
        }
    }
}

/// Contains functions to serialize a chrono::DateTime as a bson::DateTime and deserialize a
/// chrono::DateTime from a bson::DateTime.
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use bson::serde_helpers::chrono_datetime_as_bson_datetime;
/// #[derive(Serialize, Deserialize)]
/// struct Event {
///     #[serde(with = "chrono_datetime_as_bson_datetime")]
///     pub date: chrono::DateTime<chrono::Utc>,
/// }
/// ```
pub mod chrono_datetime_as_bson_datetime {
    use crate::DateTime;
    use chrono::Utc;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::result::Result;

    /// Deserializes a chrono::DateTime from a bson::DateTime.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<chrono::DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let datetime = DateTime::deserialize(deserializer)?;
        Ok(datetime.into())
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

/// Contains functions to serialize an ISO string as a bson::DateTime and deserialize an ISO string
/// from a bson::DateTime.
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use bson::serde_helpers::iso_string_as_bson_datetime;
/// #[derive(Serialize, Deserialize)]
/// struct Event {
///     #[serde(with = "iso_string_as_bson_datetime")]
///     pub date: String,
/// }
/// ```
pub mod iso_string_as_bson_datetime {
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

/// Contains functions to serialize a bson::DateTime as an ISO string and deserialize a
/// bson::DateTime from an ISO string.
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use bson::serde_helpers::bson_datetime_as_iso_string;
/// #[derive(Serialize, Deserialize)]
/// struct Event {
///     #[serde(with = "bson_datetime_as_iso_string")]
///     pub date: bson::DateTime,
/// }
/// ```
pub mod bson_datetime_as_iso_string {
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

/// Contains functions to serialize a hex string as an ObjectId and deserialize a
/// hex string from an ObjectId
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use bson::serde_helpers::hex_string_as_object_id;
/// #[derive(Serialize, Deserialize)]
/// struct Item {
///     #[serde(with = "hex_string_as_object_id")]
///     pub id: String,
/// }
/// ```
pub mod hex_string_as_object_id {
    use crate::oid::ObjectId;
    use serde::{ser, Deserialize, Deserializer, Serialize, Serializer};

    /// Deserializes a hex string from an ObjectId.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let object_id = ObjectId::deserialize(deserializer)?;
        Ok(object_id.to_hex())
    }

    /// Serializes a hex string as an ObjectId.
    pub fn serialize<S: Serializer>(val: &str, serializer: S) -> Result<S::Ok, S::Error> {
        match ObjectId::with_string(val) {
            Ok(oid) => oid.serialize(serializer),
            Err(_) => Err(ser::Error::custom(format!(
                "cannot convert {} to ObjectId",
                val
            ))),
        }
    }
}

/// Contains functions to serialize a Uuid as a bson::Binary and deserialize a Uuid from a
/// bson::Binary.
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use uuid::Uuid;
/// # use bson::serde_helpers::uuid_as_binary;
/// #[derive(Serialize, Deserialize)]
/// struct Item {
///     #[serde(with = "uuid_as_binary")]
///     pub id: Uuid,
/// }
/// ```
pub mod uuid_as_binary {
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

/// Contains functions to serialize a u32 as a bson::Timestamp and deserialize a u32 from a
/// bson::Timestamp. The u32 should represent seconds since the Unix epoch.
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use bson::serde_helpers::u32_as_timestamp;
/// #[derive(Serialize, Deserialize)]
/// struct Event {
///     #[serde(with = "u32_as_timestamp")]
///     pub time: u32,
/// }
/// ```
pub mod u32_as_timestamp {
    use crate::{Bson, Timestamp};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::result::Result;

    /// Serializes a u32 as a bson::Timestamp.
    pub fn serialize<S: Serializer>(val: &u32, serializer: S) -> Result<S::Ok, S::Error> {
        let timestamp = Bson::Timestamp(Timestamp {
            time: *val,
            increment: 0,
        });
        timestamp.serialize(serializer)
    }

    /// Deserializes a u32 from a bson::Timestamp.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let timestamp = Timestamp::deserialize(deserializer)?;
        Ok(timestamp.time)
    }
}

/// Contains functions to serialize a bson::Timestamp as a u32 and deserialize a bson::Timestamp
/// from a u32. The u32 should represent seconds since the Unix epoch. Serialization will return an
/// error if the Timestamp has a non-zero increment.
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use bson::{serde_helpers::timestamp_as_u32, Timestamp};
/// #[derive(Serialize, Deserialize)]
/// struct Item {
///     #[serde(with = "timestamp_as_u32")]
///     pub timestamp: Timestamp,
/// }
/// ```
pub mod timestamp_as_u32 {
    use crate::Timestamp;
    use serde::{ser, Deserialize, Deserializer, Serializer};
    use std::result::Result;

    /// Serializes a bson::Timestamp as a u32. Returns an error if the conversion is lossy (i.e. the
    /// Timestamp has a non-zero increment).
    pub fn serialize<S: Serializer>(val: &Timestamp, serializer: S) -> Result<S::Ok, S::Error> {
        if val.increment != 0 {
            return Err(ser::Error::custom(
                "Cannot convert Timestamp with a non-zero increment to u32",
            ));
        }
        serializer.serialize_u32(val.time)
    }

    /// Deserializes a bson::Timestamp from a u32.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<Timestamp, D::Error>
    where
        D: Deserializer<'de>,
    {
        let time = u32::deserialize(deserializer)?;
        Ok(Timestamp { time, increment: 0 })
    }
}
