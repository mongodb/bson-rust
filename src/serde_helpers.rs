//! Collection of helper functions for serializing to and deserializing from BSON using Serde

use crate::{oid::ObjectId, Binary, DateTime, Timestamp};
use serde::{de::Visitor, Deserialize, Serialize, Serializer};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    result::Result,
};
use uuid::Uuid;

/// Type converters for serializing and deserializing [`ObjectId`] using [`serde_with::serde_as`].
///
/// ## Available converters
/// - [`object_id::AsHexString`] — serializes an [`ObjectId`] as a hex string.
/// - [`object_id::FromHexString`] — serializes a hex string as an [`ObjectId`].
#[cfg(feature = "serde_with-3")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde_with-3")))]
pub mod object_id {
    use crate::{macros::serde_conv_doc, oid::ObjectId};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};

    serde_conv_doc!(
        /// Serializes an [`ObjectId`] as a hex string and deserializes an [`ObjectId`] from a hex string.
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::{serde_helpers::object_id, oid::ObjectId};
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "object_id::AsHexString")]
        ///     pub id: ObjectId,
        /// }
        /// # }
        /// ```
        pub AsHexString,
        ObjectId,
        |oid: &ObjectId| -> Result<String, String> {
            Ok(oid.to_hex())
        },
        |hex: String| -> Result<ObjectId, String> {
            ObjectId::parse_str(&hex).map_err(|e| format!("Invalid ObjectId string, {}: {}", hex, e))
        }
    );

    serde_conv_doc!(
        /// Serializes a hex string as an [`ObjectId`] and deserializes a hex string from an [`ObjectId`].
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::serde_helpers::object_id;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "object_id::FromHexString")]
        ///     pub id: String,
        /// }
        /// # }
        /// ```
        pub FromHexString,
        String,
        |hex: &String| -> Result<ObjectId, String> {
            ObjectId::parse_str(hex).map_err(|e| format!("Invalid ObjectId string, {}: {}", hex, e))
        },
        |oid: ObjectId| -> Result<String, String> {
            Ok(oid.to_hex())
        }
    );
}

/// Type converters for serializing and deserializing [`DateTime`] using [`serde_with::serde_as`].
///
/// ## Available converters
/// - [`datetime::AsRfc3339String`] — serializes a [`DateTime`] as a RFC 3339 string.
/// - [`datetime::FromRfc3339String`] — serializes a RFC 3339 string as a [`DateTime`].
/// - [`datetime::FromChronoDateTime`] — serializes a [`chrono::DateTime`] as a [`DateTime`].
/// - [`datetime::FromI64`] — serializes a `i64` as a [`DateTime`].
/// - [`datetime::FromTime03OffsetDateTime`] — serializes a [`time::OffsetDateTime`] as a
///   [`DateTime`].
#[cfg(feature = "serde_with-3")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde_with-3")))]
pub mod datetime {
    use crate::{macros::serde_conv_doc, DateTime};
    use chrono::Utc;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};

    serde_conv_doc!(
        /// Serializes a [`DateTime`] as an RFC 3339 (ISO 8601) formatted string and deserializes
        /// a [`DateTime`] from an RFC 3339 (ISO 8601) formatted string.
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::{serde_helpers::datetime, DateTime};
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Event {
        ///     #[serde_as(as = "datetime::AsRfc3339String")]
        ///     pub date: DateTime,
        /// }
        /// # }
        /// ```
        pub AsRfc3339String,
        DateTime,
        |date: &DateTime| -> Result<String, String> {
            date.try_to_rfc3339_string().map_err(|e| {
                format!("Cannot format DateTime {} as String: {}", date, e)
            })
        },
        |string: String| -> Result<DateTime, String> {
            DateTime::parse_rfc3339_str(&string).map_err(|e| format!("Cannot format String {} as DateTime: {}", string, e))
        }
    );

    serde_conv_doc!(
        /// Serializes an RFC 3339 (ISO 8601) formatted string as a [`DateTime`] and deserializes an
        /// RFC 3339 (ISO 8601) formatted string from a [`DateTime`].
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::serde_helpers::datetime;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Event {
        ///     #[serde_as(as = "datetime::FromRfc3339String")]
        ///     pub date: String,
        /// }
        /// # }
        /// ```
        pub FromRfc3339String,
        String,
        |string: &String| -> Result<DateTime, String> {
            DateTime::parse_rfc3339_str(string).map_err(|e| format!("Cannot format String {} as DateTime: {}", string, e))
        },
        |date: DateTime| -> Result<String, String> {
            date.try_to_rfc3339_string().map_err(|e| {
                format!("Cannot format DateTime {} as String: {}", date, e)
            })
        }
    );

    serde_conv_doc!(
        #[cfg(feature = "chrono-0_4")]
        #[cfg_attr(docsrs, doc(cfg(feature = "chrono-0_4")))]
        /// Serializes a [`chrono::DateTime`] as a [`DateTime`] and deserializes a [`chrono::DateTime`]
        /// from a [`DateTime`].
        /// ```rust
        /// # #[cfg(all(feature = "chrono-0_4", feature = "serde_with-3"))]
        /// # {
        /// use bson::serde_helpers::datetime;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Event {
        ///     #[serde_as(as = "datetime::FromChronoDateTime")]
        ///     pub date: chrono::DateTime<chrono::Utc>,
        /// }
        /// # }
        /// ```
        pub FromChronoDateTime,
        chrono::DateTime<Utc>,
        |chrono_date: &chrono::DateTime<Utc>| -> Result<DateTime, String> {
            Ok(DateTime::from_chrono(*chrono_date))
        },
        |bson_date: DateTime| -> Result<chrono::DateTime<Utc>, String> {
            Ok(bson_date.to_chrono())
        }
    );

    serde_conv_doc!(
        /// Serializes a `i64` integer as [`DateTime`] and deserializes a `i64` integer from [`DateTime`].
        ///
        /// The `i64` should represent seconds `(DateTime::timestamp_millis(..))`.
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::serde_helpers::datetime;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "datetime::FromI64")]
        ///     pub now: i64,
        /// }
        /// # }
        /// ```
        pub FromI64,
        i64,
        |value: &i64| -> Result<DateTime, String> {
            Ok(DateTime::from_millis(*value))
        },
        |date: DateTime| -> Result<i64, String> {
            Ok(date.timestamp_millis())
        }
    );

    serde_conv_doc!(
        /// Serializes a [`time::OffsetDateTime`] as a [`DateTime`] and deserializes a
        /// [`time::OffsetDateTime`] from a [`DateTime`].
        /// ```rust
        /// # #[cfg(all(feature = "time-0_3", feature = "serde_with-3"))]
        /// # {
        /// use bson::serde_helpers::datetime;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Event {
        ///     #[serde_as(as = "datetime::FromTime03OffsetDateTime")]
        ///     pub date: time::OffsetDateTime,
        /// }
        /// # }
        /// ```
        pub FromTime03OffsetDateTime,
        time::OffsetDateTime,
        |value: &time::OffsetDateTime| -> Result<DateTime, String> {
            Ok(DateTime::from_time_0_3(*value))
        },
        |date: DateTime| -> Result<time::OffsetDateTime, String> {
            Ok(date.to_time_0_3())
        }
    );
}

/// Type converters for serializing and deserializing `u32` using [`serde_with::serde_as`].
///
/// ## Available converters
/// - [`u32::FromTimestamp`] — serializes a [`Timestamp`] as a `u32`.
/// - [`u32::AsTimestamp`] — serializes a `u32` as a [`Timestamp`].
/// - [`u32::AsF64`] — serializes a `u32` as a `f64`.
/// - [`u32::AsI32`] — serializes a `u32` as a `i32`.
/// - [`u32::AsI64`] — serializes a `u32` as a `i64`.
#[cfg(feature = "serde_with-3")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde_with-3")))]
pub mod u32 {
    use crate::{macros::serde_conv_doc, Timestamp};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};

    serde_conv_doc!(
        /// Serializes a [`Timestamp`] as a `u32` and deserializes a [`Timestamp`] from a `u32`.
        ///
        /// The `u32` should represent seconds since the Unix epoch.
        ///
        /// Serialization errors if the Timestamp has a non-zero increment.
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::{serde_helpers::u32, Timestamp};
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "u32::FromTimestamp")]
        ///     pub timestamp: Timestamp,
        /// }
        /// # }
        /// ```
        pub FromTimestamp,
        Timestamp,
        |timestamp: &Timestamp| -> Result<u32, String> {
            if timestamp.increment != 0 {
                return Err(format!("Cannot convert Timestamp with a non-zero increment to u32: {:?}", timestamp));
            }
            Ok(timestamp.time)
        },
        |value: u32| -> Result<Timestamp, String> {
            Ok(Timestamp { time: value, increment: 0 })
        }
    );

    serde_conv_doc!(
        /// Serializes a `u32` as a [`Timestamp`] and deserializes a `u32` from a [`Timestamp`].
        ///
        /// The `u32` should represent seconds since the Unix epoch.
        ///
        /// Deserialization errors if the Timestamp has a non-zero increment.
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::serde_helpers::u32;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Event {
        ///     #[serde_as(as = "u32::AsTimestamp")]
        ///     pub time: u32,
        /// }
        /// # }
        /// ```
        pub AsTimestamp,
        u32,
        |value: &u32| -> Result<Timestamp, String> {
            Ok(Timestamp { time: *value, increment: 0 })
        },
        |timestamp: Timestamp| -> Result<u32, String> {
            if timestamp.increment != 0 {
                return Err(format!("Cannot convert Timestamp with a non-zero increment to u32: {:?}", timestamp));
            }
            Ok(timestamp.time)
        }
    );

    serde_conv_doc!(
        /// Serializes a `u32` as an `f64` and deserializes a `u32` from an `f64`.
        ///
        /// Errors if an exact conversion is not possible.
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::serde_helpers::u32;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct FileInfo {
        ///     #[serde_as(as = "u32::AsF64")]
        ///     pub size_bytes: u32,
        /// }
        /// # }
        /// ```
        pub AsF64,
        u32,
        |value: &u32| -> Result<f64, String> {
            Ok(*value as f64)
        },
        |value: f64| -> Result<u32, String> {
            if (value - value as u32 as f64).abs() <= f64::EPSILON {
                Ok(value as u32)
            } else {
                Err(format!("Cannot convert f64 {} to u32", value))
            }
        }
    );

    serde_conv_doc!(
        /// Serializes a `u32` as an `i32` and deserializes a `u32` from an `i32`.
        ///
        /// Errors if an exact conversion is not possible.
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::{serde_helpers::u32};
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "u32::AsI32")]
        ///     pub value: u32,
        /// }
        /// # }
        /// ```
        pub AsI32,
        u32,
        |value: &u32| -> Result<i32, String> {
            i32::try_from(*value).map_err(|e| format!("Cannot convert u32 {} to i32: {}", value, e))
        },
        |value: i32| -> Result<u32, String> {
            u32::try_from(value).map_err(|e| format!("Cannot convert i32 {} to u32: {}", value, e))
        }
    );

    serde_conv_doc!(
        /// Serializes a `u32` as an `i64` and deserializes a `u32` from an `i64`.
        ///
        /// Errors if an exact conversion is not possible.
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::{serde_helpers::u32};
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "u32::AsI64")]
        ///     pub value: u32,
        /// }
        /// # }
        /// ```
        pub AsI64,
        u32,
        |value: &u32| -> Result<i64, String> {
            Ok(*value as i64)
        },
        |value: i64| -> Result<u32, String> {
            u32::try_from(value).map_err(|e| format!("Cannot convert i64 {} to u32: {}", value, e))
        }
    );
}

/// Type converters for serializing and deserializing `u64` using [`serde_with::serde_as`].
///
/// ## Available converters
/// - [`u64::AsF64`] — serializes a `u64` as a `f64`.
/// - [`u64::AsI32`] — serializes a `u64` as a `i32`.
/// - [`u64::AsI64`] — serializes a `u64` as a `i64`.
#[cfg(feature = "serde_with-3")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde_with-3")))]
pub mod u64 {
    use crate::macros::serde_conv_doc;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};

    serde_conv_doc!(
        /// Serializes a `u64` as an `f64` and deserializes a `u64` from an `f64`.
        ///
        /// Deserialization errors if an exact conversion is not possible.
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::serde_helpers::u64;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct FileInfo {
        ///     #[serde_as(as = "u64::AsF64")]
        ///     pub size_bytes: u64,
        /// }
        /// # }
        /// ```
        pub AsF64,
        u64,
        |value: &u64| -> Result<f64, String> {
            if value < &u64::MAX && *value == *value as f64 as u64 {
                Ok(*value as f64)
            } else {
                Err(format!("Cannot convert u64 {} to f64", value))
            }
        },
        |value: f64| -> Result<u64, String> {
            if (value - value as u64 as f64).abs() <= f64::EPSILON {
               Ok(value as u64)
            } else {
                Err(format!("Cannot convert f64 {} to u64", value))
            }
        }
    );

    serde_conv_doc!(
        /// Serializes a `u64` as an `i32` and deserializes a `u64` from an `i32`.
        ///
        /// Errors if an exact conversion is not possible.
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::{serde_helpers::u64};
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "u64::AsI32")]
        ///     pub value: u64,
        /// }
        /// # }
        /// ```
        pub AsI32,
        u64,
        |value: &u64| -> Result<i32, String> {
            i32::try_from(*value).map_err(|e| format!("Cannot convert u64 {} to i32: {}", value, e))
        },
        |value: i32| -> Result<u64, String> {
            u64::try_from(value).map_err(|e| format!("Cannot convert i32 {} to u64: {}", value, e))
        }
    );

    serde_conv_doc!(
        /// Serializes a `u64` as an `i64` and deserializes a `u64` from an `i64`.
        ///
        /// Errors if an exact conversion is not possible.
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::{serde_helpers::u64};
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "u64::AsI64")]
        ///     pub value: u64,
        /// }
        /// # }
        /// ```
        pub AsI64,
        u64,
        |value: &u64| -> Result<i64, String> {
            i64::try_from(*value).map_err(|e| format!("Cannot convert u64 {} to i64: {}", value, e))
        },
        |value: i64| -> Result<u64, String> {
            u64::try_from(value).map_err(|e| format!("Cannot convert i64 {} to u64: {}", value, e))
        }
    );
}

/// Type converters for serializing and deserializing [`Uuid`] using [`serde_with::serde_as`].
///
/// ## Available converters
/// - [`uuid_1::AsBinary`] — serializes a [`Uuid`] as a [`Binary`].
/// - [`uuid_1::AsCSharpLegacyBinary`] — serializes a [`Uuid`] as a [`Binary`] in the legacy C#
///   driver UUID format.
/// - [`uuid_1::AsJavaLegacyBinary`] — serializes a [`Uuid`] as a [`Binary`] in the legacy Java
///   driver UUID format.
/// - [`uuid_1::AsPythonLegacyBinary`] — serializes a [`Uuid`] as a [`Binary`] in the legacy Python
///   driver UUID format.
#[cfg(all(feature = "serde_with-3", feature = "uuid-1"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "serde_with-3", feature = "uuid-1"))))]
pub mod uuid_1 {
    use crate::{macros::serde_conv_doc, Binary};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use serde_with::{DeserializeAs, SerializeAs};
    use uuid::Uuid;

    serde_conv_doc!(
        /// Serializes a [`Uuid`] as a [`Binary`] and deserializes a [`Uuid`] from a [`Binary`].
        /// ```rust
        /// # #[cfg(all(feature = "uuid-1", feature = "serde_with-3"))]
        /// # {
        /// use bson::serde_helpers::uuid_1;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// use uuid::Uuid;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "uuid_1::AsBinary")]
        ///     pub id: Uuid,
        /// }
        /// # }
        /// ```
        pub AsBinary,
        Uuid,
        |uuid: &Uuid| -> Result<crate::uuid::Uuid, String> {
            Ok(crate::uuid::Uuid::from(*uuid))
        },
        |bson_uuid: crate::uuid::Uuid| -> Result<Uuid, String> {
            Ok(bson_uuid.into())
        }
    );

    serde_conv_doc!(
        /// Serializes a [`Uuid`] to a [`Binary`] in the legacy C# driver UUID format and
        /// deserializes [`Uuid`] from a [`Binary`] in the legacy C# driver format.
        /// ```rust
        /// # #[cfg(all(feature = "uuid-1", feature = "serde_with-3"))]
        /// # {
        /// use bson::serde_helpers::uuid_1;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// use uuid::Uuid;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "uuid_1::AsCSharpLegacyBinary")]
        ///     pub id: Uuid,
        /// }
        /// # }
        /// ```
        pub AsCSharpLegacyBinary,
        Uuid,
        |uuid: &Uuid| -> Result<crate::Binary, String> {
            let inner = crate::uuid::Uuid::from(*uuid);
            Ok(crate::Binary::from_uuid_with_representation(
                inner,
                crate::uuid::UuidRepresentation::CSharpLegacy,
            ))
        },
        |binary: crate::Binary| -> Result<Uuid, String> {
            let inner = binary
                .to_uuid_with_representation(crate::uuid::UuidRepresentation::CSharpLegacy)
                .map_err(|e| e.to_string())?;
            Ok(inner.into())
        }
    );

    serde_conv_doc!(
        /// Serializes a [`Uuid`] to a [`Binary`] in the legacy Java driver UUID format and
        /// deserializes [`Uuid`] from a [`Binary`] in the legacy Java driver format.
        /// ```rust
        /// # #[cfg(all(feature = "uuid-1", feature = "serde_with-3"))]
        /// # {
        /// use bson::serde_helpers::uuid_1;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// use uuid::Uuid;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "uuid_1::AsJavaLegacyBinary")]
        ///     pub id: Uuid,
        /// }
        /// # }
        /// ```
        pub AsJavaLegacyBinary,
        Uuid,
        |uuid: &Uuid| -> Result<crate::Binary, String> {
            let inner = crate::uuid::Uuid::from(*uuid);
            Ok(crate::Binary::from_uuid_with_representation(
                inner,
                crate::uuid::UuidRepresentation::JavaLegacy,
            ))
        },
        |binary: crate::Binary| -> Result<Uuid, String> {
            let inner = binary
                .to_uuid_with_representation(crate::uuid::UuidRepresentation::JavaLegacy)
                .map_err(|e| e.to_string())?;
            Ok(inner.into())
        }
    );

    serde_conv_doc!(
        /// Serializes a [`Uuid`] to a [`Binary`] in the legacy Python driver UUID format and
        /// deserializes [`Uuid`] from a [`Binary`] in the legacy Python driver format.
        /// ```rust
        /// # #[cfg(all(feature = "uuid-1", feature = "serde_with-3"))]
        /// # {
        /// use bson::serde_helpers::uuid_1;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// use uuid::Uuid;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "uuid_1::AsPythonLegacyBinary")]
        ///     pub id: Uuid,
        /// }
        /// # }
        /// ```
        pub AsPythonLegacyBinary,
        Uuid,
        |uuid: &Uuid| -> Result<crate::Binary, String> {
            let inner = crate::uuid::Uuid::from(*uuid);
            Ok(crate::Binary::from_uuid_with_representation(
                inner,
                crate::uuid::UuidRepresentation::PythonLegacy,
            ))
        },
        |binary: crate::Binary| -> Result<Uuid, String> {
            let inner = binary
                .to_uuid_with_representation(crate::uuid::UuidRepresentation::PythonLegacy)
                .map_err(|e| e.to_string())?;
            Ok(inner.into())
        }
    );
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

/// Wrapping a type in `HumanReadable` signals to the BSON serde integration that it and all
/// recursively contained types should be serialized to and deserialized from their human-readable
/// formats.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct HumanReadable<T>(pub T);

pub(crate) const HUMAN_READABLE_NEWTYPE: &str = "$__bson_private_human_readable";

impl<T: Serialize> Serialize for HumanReadable<T> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_newtype_struct(HUMAN_READABLE_NEWTYPE, &self.0)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for HumanReadable<T> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V<T>(PhantomData<fn() -> T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for V<T> {
            type Value = HumanReadable<T>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("HumanReadable wrapper")
            }
            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                T::deserialize(deserializer).map(HumanReadable)
            }
        }
        deserializer.deserialize_newtype_struct(HUMAN_READABLE_NEWTYPE, V(PhantomData))
    }
}

impl<T: std::fmt::Display> std::fmt::Display for HumanReadable<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> From<T> for HumanReadable<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for HumanReadable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for HumanReadable<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T, R> AsRef<R> for HumanReadable<T>
where
    R: ?Sized,
    <HumanReadable<T> as Deref>::Target: AsRef<R>,
{
    fn as_ref(&self) -> &R {
        self.deref().as_ref()
    }
}

impl<T, R: ?Sized> AsMut<R> for HumanReadable<T>
where
    <HumanReadable<T> as Deref>::Target: AsMut<R>,
{
    fn as_mut(&mut self) -> &mut R {
        self.deref_mut().as_mut()
    }
}

// One could imagine passthrough Borrow impls; however, it turns out that can't be made to work
// because of the existing base library impl of Borrow<T> for T will conflict despite that not
// actually being possible to construct (https://github.com/rust-lang/rust/issues/50237).  So,
// sadly, Borrow impls for HumanReadable are deliberately omitted :(

/// Wrapper type for deserializing BSON bytes with invalid UTF-8 sequences.
///
/// Any invalid UTF-8 strings contained in the wrapped type will be replaced with the Unicode
/// replacement character. This wrapper type only has an effect when deserializing from BSON bytes.
///
/// This wrapper type has no impact on serialization. Serializing a `Utf8LossyDeserialization<T>`
/// will call the `serialize` method for the wrapped `T`.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct Utf8LossyDeserialization<T>(pub T);

pub(crate) const UTF8_LOSSY_NEWTYPE: &str = "$__bson_private_utf8_lossy";

impl<T: Serialize> Serialize for Utf8LossyDeserialization<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Utf8LossyDeserialization<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V<T>(PhantomData<fn() -> T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for V<T> {
            type Value = Utf8LossyDeserialization<T>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Utf8Lossy wrapper")
            }
            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                T::deserialize(deserializer).map(Utf8LossyDeserialization)
            }
        }
        deserializer.deserialize_newtype_struct(UTF8_LOSSY_NEWTYPE, V(PhantomData))
    }
}

impl<T: std::fmt::Display> std::fmt::Display for Utf8LossyDeserialization<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> From<T> for Utf8LossyDeserialization<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for Utf8LossyDeserialization<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Utf8LossyDeserialization<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T, R> AsRef<R> for Utf8LossyDeserialization<T>
where
    R: ?Sized,
    <Utf8LossyDeserialization<T> as Deref>::Target: AsRef<R>,
{
    fn as_ref(&self) -> &R {
        self.deref().as_ref()
    }
}

impl<T, R: ?Sized> AsMut<R> for Utf8LossyDeserialization<T>
where
    <Utf8LossyDeserialization<T> as Deref>::Target: AsMut<R>,
{
    fn as_mut(&mut self) -> &mut R {
        self.deref_mut().as_mut()
    }
}
