//! Collection of helper functions for serializing to and deserializing from BSON using Serde

use std::{convert::TryFrom, result::Result};

use serde::{ser, Serialize, Serializer};

use crate::oid::ObjectId;

#[doc(inline)]
pub use bson_datetime_as_rfc3339_string::{
    deserialize as deserialize_bson_datetime_from_rfc3339_string,
    serialize as serialize_bson_datetime_as_rfc3339_string,
};
#[cfg(feature = "chrono-0_4")]
#[doc(inline)]
pub use chrono_datetime_as_bson_datetime::{
    deserialize as deserialize_chrono_datetime_from_bson_datetime,
    serialize as serialize_chrono_datetime_as_bson_datetime,
};
#[doc(inline)]
pub use hex_string_as_object_id::{
    deserialize as deserialize_hex_string_from_object_id,
    serialize as serialize_hex_string_as_object_id,
};
#[doc(inline)]
pub use i64_as_bson_datetime::{
    deserialize as deserialize_i64_from_bson_datetime,
    serialize as serialize_i64_as_bson_datetime,
};
#[doc(inline)]
pub use rfc3339_string_as_bson_datetime::{
    deserialize as deserialize_rfc3339_string_from_bson_datetime,
    serialize as serialize_rfc3339_string_as_bson_datetime,
};
#[cfg(feature = "time-0_3")]
#[doc(inline)]
pub use time_0_3_offsetdatetime_as_bson_datetime::{
    deserialize as deserialize_time_0_3_offsetdatetime_from_bson_datetime,
    serialize as serialize_time_0_3_offsetdatetime_as_bson_datetime,
};
#[doc(inline)]
pub use timestamp_as_u32::{
    deserialize as deserialize_timestamp_from_u32,
    serialize as serialize_timestamp_as_u32,
};
#[doc(inline)]
pub use u32_as_f64::{deserialize as deserialize_u32_from_f64, serialize as serialize_u32_as_f64};
#[doc(inline)]
pub use u32_as_timestamp::{
    deserialize as deserialize_u32_from_timestamp,
    serialize as serialize_u32_as_timestamp,
};
#[doc(inline)]
pub use u64_as_f64::{deserialize as deserialize_u64_from_f64, serialize as serialize_u64_as_f64};

#[cfg(feature = "uuid-1")]
#[doc(inline)]
pub use uuid_1_as_binary::{
    deserialize as deserialize_uuid_1_from_binary,
    serialize as serialize_uuid_1_as_binary,
};
#[cfg(feature = "uuid-1")]
#[doc(inline)]
pub use uuid_1_as_c_sharp_legacy_binary::{
    deserialize as deserialize_uuid_1_from_c_sharp_legacy_binary,
    serialize as serialize_uuid_1_as_c_sharp_legacy_binary,
};
#[cfg(feature = "uuid-1")]
#[doc(inline)]
pub use uuid_1_as_java_legacy_binary::{
    deserialize as deserialize_uuid_1_from_java_legacy_binary,
    serialize as serialize_uuid_1_as_java_legacy_binary,
};
#[cfg(feature = "uuid-1")]
#[doc(inline)]
pub use uuid_1_as_python_legacy_binary::{
    deserialize as deserialize_uuid_1_from_python_legacy_binary,
    serialize as serialize_uuid_1_as_python_legacy_binary,
};
#[cfg(feature = "uuid-0_8")]
#[doc(inline)]
pub use uuid_as_binary::{
    deserialize as deserialize_uuid_from_binary,
    serialize as serialize_uuid_as_binary,
};
#[cfg(feature = "uuid-0_8")]
#[doc(inline)]
pub use uuid_as_c_sharp_legacy_binary::{
    deserialize as deserialize_uuid_from_c_sharp_legacy_binary,
    serialize as serialize_uuid_as_c_sharp_legacy_binary,
};
#[cfg(feature = "uuid-0_8")]
#[doc(inline)]
pub use uuid_as_java_legacy_binary::{
    deserialize as deserialize_uuid_from_java_legacy_binary,
    serialize as serialize_uuid_as_java_legacy_binary,
};
#[cfg(feature = "uuid-0_8")]
#[doc(inline)]
pub use uuid_as_python_legacy_binary::{
    deserialize as deserialize_uuid_from_python_legacy_binary,
    serialize as serialize_uuid_as_python_legacy_binary,
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

/// Serializes an [`ObjectId`] as a hex string.
pub fn serialize_object_id_as_hex_string<S: Serializer>(
    val: &ObjectId,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    val.to_hex().serialize(serializer)
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

/// Contains functions to serialize a [`time::OffsetDateTime`] as a [`crate::DateTime`] and
/// deserialize a [`time::OffsetDateTime`] from a [`crate::DateTime`].
///
/// ```rust
/// # #[cfg(feature = "time-0_3")]
/// # {
/// # use serde::{Serialize, Deserialize};
/// # use bson::serde_helpers::time_0_3_offsetdatetime_as_bson_datetime;
/// #[derive(Serialize, Deserialize)]
/// struct Event {
///     #[serde(with = "time_0_3_offsetdatetime_as_bson_datetime")]
///     pub date: time::OffsetDateTime,
/// }
/// # }
/// ```
#[cfg(feature = "time-0_3")]
#[cfg_attr(docsrs, doc(cfg(feature = "time-0_3")))]
pub mod time_0_3_offsetdatetime_as_bson_datetime {
    use crate::DateTime;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::result::Result;

    /// Deserializes a [`time::OffsetDateTime`] from a [`crate::DateTime`].
    #[cfg_attr(docsrs, doc(cfg(feature = "time-0_3")))]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<time::OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let datetime = DateTime::deserialize(deserializer)?;
        Ok(datetime.to_time_0_3())
    }

    /// Serializes a [`time::OffsetDateTime`] as a [`crate::DateTime`].
    #[cfg_attr(docsrs, doc(cfg(feature = "time-0_3")))]
    pub fn serialize<S: Serializer>(
        val: &time::OffsetDateTime,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let datetime = DateTime::from_time_0_3(val.to_owned());
        datetime.serialize(serializer)
    }
}

/// Contains functions to serialize a [`chrono::DateTime`] as a [`crate::DateTime`] and deserialize
/// a [`chrono::DateTime`] from a [`crate::DateTime`].
///
/// ```rust
/// # #[cfg(feature = "chrono-0_4")]
/// # {
/// # use serde::{Serialize, Deserialize};
/// # use bson::serde_helpers::chrono_datetime_as_bson_datetime;
/// #[derive(Serialize, Deserialize)]
/// struct Event {
///     #[serde(with = "chrono_datetime_as_bson_datetime")]
///     pub date: chrono::DateTime<chrono::Utc>,
/// }
/// # }
/// ```
#[cfg(feature = "chrono-0_4")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono-0_4")))]
pub mod chrono_datetime_as_bson_datetime {
    use crate::DateTime;
    use chrono::Utc;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::result::Result;

    /// Deserializes a [`chrono::DateTime`] from a [`crate::DateTime`].
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono-0_4")))]
    pub fn deserialize<'de, D>(deserializer: D) -> Result<chrono::DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let datetime = DateTime::deserialize(deserializer)?;
        Ok(datetime.to_chrono())
    }

    /// Serializes a [`chrono::DateTime`] as a [`crate::DateTime`].
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono-0_4")))]
    pub fn serialize<S: Serializer>(
        val: &chrono::DateTime<Utc>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        let datetime = DateTime::from_chrono(val.to_owned());
        datetime.serialize(serializer)
    }
}

/// Contains functions to serialize an RFC 3339 (ISO 8601) formatted string as a [`crate::DateTime`]
/// and deserialize an RFC 3339 (ISO 8601) formatted string from a [`crate::DateTime`].
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use bson::serde_helpers::rfc3339_string_as_bson_datetime;
/// #[derive(Serialize, Deserialize)]
/// struct Event {
///     #[serde(with = "rfc3339_string_as_bson_datetime")]
///     pub date: String,
/// }
/// ```
pub mod rfc3339_string_as_bson_datetime {
    use crate::{Bson, DateTime};
    use serde::{de, ser, Deserialize, Deserializer, Serialize, Serializer};
    use std::result::Result;

    /// Deserializes an ISO string from a DateTime.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let date = DateTime::deserialize(deserializer)?;
        date.try_to_rfc3339_string()
            .map_err(|e| de::Error::custom(format!("cannot format {} as RFC 3339: {}", date, e)))
    }

    /// Serializes an ISO string as a DateTime.
    pub fn serialize<S: Serializer>(val: &str, serializer: S) -> Result<S::Ok, S::Error> {
        let date = crate::DateTime::parse_rfc3339_str(val)
            .map_err(|_| ser::Error::custom(format!("cannot convert {} to DateTime", val)))?;
        Bson::DateTime(date).serialize(serializer)
    }
}

/// Contains functions to serialize a [`crate::DateTime`] as an RFC 3339 (ISO 8601) formatted string
/// and deserialize a [`crate::DateTime`] from an RFC 3339 (ISO 8601) formatted string.
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use bson::serde_helpers::bson_datetime_as_rfc3339_string;
/// #[derive(Serialize, Deserialize)]
/// struct Event {
///     #[serde(with = "bson_datetime_as_rfc3339_string")]
///     pub date: bson::DateTime,
/// }
/// ```
pub mod bson_datetime_as_rfc3339_string {
    use crate::DateTime;
    use serde::{de, ser, Deserialize, Deserializer, Serializer};
    use std::result::Result;

    /// Deserializes a [`crate::DateTime`] from an RFC 3339 formatted string.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let iso = String::deserialize(deserializer)?;
        let date = crate::DateTime::parse_rfc3339_str(&iso).map_err(|_| {
            de::Error::custom(format!("cannot parse RFC 3339 datetime from \"{}\"", iso))
        })?;
        Ok(date)
    }

    /// Serializes a [`crate::DateTime`] as an RFC 3339 (ISO 8601) formatted string.
    pub fn serialize<S: Serializer>(val: &DateTime, serializer: S) -> Result<S::Ok, S::Error> {
        let formatted = val
            .try_to_rfc3339_string()
            .map_err(|e| ser::Error::custom(format!("cannot format {} as RFC 3339: {}", val, e)))?;
        serializer.serialize_str(&formatted)
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
        match ObjectId::parse_str(val) {
            Ok(oid) => oid.serialize(serializer),
            Err(_) => Err(ser::Error::custom(format!(
                "cannot convert {} to ObjectId",
                val
            ))),
        }
    }
}

/// Contains functions to `serialize` a `i64` integer as [`DateTime`](crate::DateTime) and
/// `deserialize` a `i64` integer from [`DateTime`](crate::DateTime).
///
/// ### The i64 should represent seconds `(DateTime::timestamp_millis(..))`.
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use bson::serde_helpers::i64_as_bson_datetime;
/// #[derive(Serialize, Deserialize)]
/// struct Item {
///     #[serde(with = "i64_as_bson_datetime")]
///     pub now: i64,
/// }
/// ```
pub mod i64_as_bson_datetime {
    use crate::DateTime;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    /// Deserializes a i64 integer from a DateTime.
    pub fn deserialize<'de, D>(deserializer: D) -> Result<i64, D::Error>
    where
        D: Deserializer<'de>,
    {
        let date: DateTime = DateTime::deserialize(deserializer)?;
        Ok(date.timestamp_millis())
    }

    /// Serializes a i64 integer as a DateTime.
    pub fn serialize<S: Serializer>(val: &i64, serializer: S) -> Result<S::Ok, S::Error> {
        let date_time = DateTime::from_millis(*val);
        date_time.serialize(serializer)
    }
}

#[allow(unused_macros)]
macro_rules! as_binary_mod {
    ($feat:meta, $uu:path) => {
        use serde::{Deserialize, Deserializer, Serialize, Serializer};
        use std::result::Result;
        use $uu;

        /// Serializes a Uuid as a Binary.
        #[cfg_attr(docsrs, doc($feat))]
        pub fn serialize<S: Serializer>(val: &Uuid, serializer: S) -> Result<S::Ok, S::Error> {
            crate::uuid::Uuid::from(*val).serialize(serializer)
        }

        /// Deserializes a Uuid from a Binary.
        #[cfg_attr(docsrs, doc($feat))]
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
        where
            D: Deserializer<'de>,
        {
            let bson_uuid = crate::uuid::Uuid::deserialize(deserializer)?;
            Ok(bson_uuid.into())
        }
    };
}

/// Contains functions to serialize a [`uuid_0_8::Uuid`] as a [`crate::Binary`] and deserialize a
/// [`uuid_0_8::Uuid`] from a [`crate::Binary`].
///
/// ```rust
/// # #[cfg(feature = "uuid-0_8")]
/// # {
/// use serde::{Serialize, Deserialize};
/// use uuid_0_8::Uuid;
/// use bson::serde_helpers::uuid_as_binary;
///
/// #[derive(Serialize, Deserialize)]
/// struct Item {
///     #[serde(with = "uuid_as_binary")]
///     pub id: Uuid,
/// }
/// # }
/// ```
#[cfg(feature = "uuid-0_8")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid-0_8")))]
pub mod uuid_as_binary {
    as_binary_mod!(cfg(feature = "uuid-0_8"), uuid_0_8::Uuid);
}

/// Contains functions to serialize a [`uuid::Uuid`] as a [`crate::Binary`] and deserialize a
/// [`uuid::Uuid`] from a [`crate::Binary`].
///
/// ```rust
/// # #[cfg(feature = "uuid-1")]
/// # {
/// use serde::{Serialize, Deserialize};
/// use uuid::Uuid;
/// use bson::serde_helpers::uuid_1_as_binary;
///
/// #[derive(Serialize, Deserialize)]
/// struct Item {
///     #[serde(with = "uuid_1_as_binary")]
///     pub id: Uuid,
/// }
/// # }
/// ```
#[cfg(feature = "uuid-1")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid-1")))]
pub mod uuid_1_as_binary {
    as_binary_mod!(cfg(feature = "uuid-1"), uuid::Uuid);
}

#[allow(unused_macros)]
macro_rules! as_legacy_binary_mod {
    ($feat:meta, $uu:path, $rep:path) => {
        use crate::{uuid::UuidRepresentation, Binary};
        use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
        use std::result::Result;
        use $uu;

        /// Serializes a Uuid as a Binary in the legacy UUID format.
        #[cfg_attr(docsrs, doc($feat))]
        pub fn serialize<S: Serializer>(val: &Uuid, serializer: S) -> Result<S::Ok, S::Error> {
            let binary = Binary::from_uuid_with_representation(crate::uuid::Uuid::from(*val), $rep);
            binary.serialize(serializer)
        }

        /// Deserializes a Uuid from a Binary in the legacy UUID format.
        #[cfg_attr(docsrs, doc($feat))]
        pub fn deserialize<'de, D>(deserializer: D) -> Result<Uuid, D::Error>
        where
            D: Deserializer<'de>,
        {
            let binary = Binary::deserialize(deserializer)?;
            let uuid = binary
                .to_uuid_with_representation($rep)
                .map_err(de::Error::custom)?;
            Ok(uuid.into())
        }
    };
}

/// Contains functions to serialize a [`uuid_0_8::Uuid`] to a [`crate::Binary`] in the legacy
/// Java driver UUID format and deserialize [`uuid_0_8::Uuid`] from a [`crate::Binary`] in the
/// legacy Java driver format.
///
/// ```rust
/// #[cfg(feature = "uuid-0_8")]
/// # {
/// use serde::{Serialize, Deserialize};
/// use uuid_0_8::Uuid;
/// use bson::serde_helpers::uuid_as_java_legacy_binary;
///
/// #[derive(Serialize, Deserialize)]
/// struct Item {
///     #[serde(with = "uuid_as_java_legacy_binary")]
///     pub id: Uuid,
/// }
/// # }
/// ```
#[cfg(feature = "uuid-0_8")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid-0_8")))]
pub mod uuid_as_java_legacy_binary {
    as_legacy_binary_mod!(
        cfg(feature = "uuid-0_8"),
        uuid_0_8::Uuid,
        UuidRepresentation::JavaLegacy
    );
}

/// Contains functions to serialize a [`uuid::Uuid`] to a [`crate::Binary`] in the legacy
/// Java driver UUID format and deserialize [`uuid::Uuid`] from a [`crate::Binary`] in the legacy
/// Java driver format.
///
/// ```rust
/// #[cfg(feature = "uuid-1")]
/// # {
/// use serde::{Serialize, Deserialize};
/// use uuid::Uuid;
/// use bson::serde_helpers::uuid_1_as_java_legacy_binary;
///
/// #[derive(Serialize, Deserialize)]
/// struct Item {
///     #[serde(with = "uuid_1_as_java_legacy_binary")]
///     pub id: Uuid,
/// }
/// # }
/// ```
#[cfg(feature = "uuid-1")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid-1")))]
pub mod uuid_1_as_java_legacy_binary {
    as_legacy_binary_mod!(
        cfg(feature = "uuid-1"),
        uuid::Uuid,
        UuidRepresentation::JavaLegacy
    );
}

/// Contains functions to serialize a [`uuid_0_8::Uuid`] to a [`crate::Binary`] in the legacy Python
/// driver UUID format and deserialize [`uuid_0_8::Uuid`] from a [`crate::Binary`] in the legacy
/// Python driver format.
///
/// ```rust
/// # #[cfg(feature = "uuid-0_8")]
/// # {
/// use serde::{Serialize, Deserialize};
/// use uuid_0_8::Uuid;
/// use bson::serde_helpers::uuid_as_python_legacy_binary;
///
/// #[derive(Serialize, Deserialize)]
/// struct Item {
///     #[serde(with = "uuid_as_python_legacy_binary")]
///     pub id: Uuid,
/// }
/// # }
/// ```
#[cfg(feature = "uuid-0_8")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid-0_8")))]
pub mod uuid_as_python_legacy_binary {
    as_legacy_binary_mod!(
        cfg(feature = "uuid-0_8"),
        uuid_0_8::Uuid,
        UuidRepresentation::PythonLegacy
    );
}

/// Contains functions to serialize a [`uuid::Uuid`] to a [`crate::Binary`] in the legacy Python
/// driver UUID format and deserialize [`uuid::Uuid`] from a [`crate::Binary`] in the legacy Python
/// driver format.
///
/// ```rust
/// # #[cfg(feature = "uuid-1")]
/// # {
/// use serde::{Serialize, Deserialize};
/// use uuid::Uuid;
/// use bson::serde_helpers::uuid_1_as_python_legacy_binary;
///
/// #[derive(Serialize, Deserialize)]
/// struct Item {
///     #[serde(with = "uuid_1_as_python_legacy_binary")]
///     pub id: Uuid,
/// }
/// # }
/// ```
#[cfg(feature = "uuid-1")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid-1")))]
pub mod uuid_1_as_python_legacy_binary {
    as_legacy_binary_mod!(
        cfg(feature = "uuid-1"),
        uuid::Uuid,
        UuidRepresentation::PythonLegacy
    );
}

/// Contains functions to serialize a [`uuid_0_8::Uuid`] to a [`crate::Binary`] in the legacy C#
/// driver UUID format and deserialize [`uuid_0_8::Uuid`] from a [`crate::Binary`] in the legacy C#
/// driver format.
///
/// ```rust
/// # #[cfg(feature = "uuid-0_8")]
/// # {
/// use serde::{Serialize, Deserialize};
/// use uuid_0_8::Uuid;
/// use bson::serde_helpers::uuid_as_c_sharp_legacy_binary;
///
/// #[derive(Serialize, Deserialize)]
/// struct Item {
///     #[serde(with = "uuid_as_c_sharp_legacy_binary")]
///     pub id: Uuid,
/// }
/// # }
/// ```
#[cfg(feature = "uuid-0_8")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid-0_8")))]
pub mod uuid_as_c_sharp_legacy_binary {
    as_legacy_binary_mod!(
        cfg(feature = "uuid-0_8"),
        uuid_0_8::Uuid,
        UuidRepresentation::CSharpLegacy
    );
}

/// Contains functions to serialize a [`uuid::Uuid`] to a [`crate::Binary`] in the legacy C# driver
/// UUID format and deserialize [`uuid::Uuid`] from a [`crate::Binary`] in the legacy C# driver
/// format.
///
/// ```rust
/// # #[cfg(feature = "uuid-1")]
/// # {
/// use serde::{Serialize, Deserialize};
/// use uuid::Uuid;
/// use bson::serde_helpers::uuid_1_as_c_sharp_legacy_binary;
///
/// #[derive(Serialize, Deserialize)]
/// struct Item {
///     #[serde(with = "uuid_1_as_c_sharp_legacy_binary")]
///     pub id: Uuid,
/// }
/// # }
/// ```
#[cfg(feature = "uuid-1")]
#[cfg_attr(docsrs, doc(cfg(feature = "uuid-1")))]
pub mod uuid_1_as_c_sharp_legacy_binary {
    as_legacy_binary_mod!(
        cfg(feature = "uuid-1"),
        uuid::Uuid,
        UuidRepresentation::CSharpLegacy
    );
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
