//! Collection of helper functions for serializing to and deserializing from BSON using Serde.
//!
//! The submodules here provide converter types that can be used with the `#[serde(with = ...)]`
//! annotation.  These modules follow a naming convention:
//! * _module name_ - the _base type_ to be converted
//! * _module_`::AsFoo` - when serializing/deserializing a field of the base type, store it as a
//!   _Foo_ value
//! * _module_`::FromFoo` - when serializing/deserializing a field of type _Foo_, store it as a
//!   value of the base type
//!
//! For example, the [`object_id`] module provides both [`object_id::AsHexString`] and
//! [`object_id::FromHexString`]:
//! ```
//! # use serde::{Deserialize, Serialize};
//! use bson::{doc, serde_helpers::object_id, oid::ObjectId};
//!
//! #[derive(Deserialize, Serialize)]
//! struct Example {
//!   // No conversions applied; will serialize as the BSON value.
//!   basic: ObjectId,
//!   // In code, an ObjectId; when serialized, a hex string.
//!   #[serde(with = "object_id::AsHexString")]
//!   as_hex: ObjectId,
//!   // In code, a hex string; serializes as a BSON objectid.
//!   #[serde(with = "object_id::FromHexString")]
//!   from_hex: String,
//! }
//! ```
//!
//! If the `serde_with-3` feature is enabled, these converters can also be used with the
//! `#[serde_as(as = ...)]` annotation, which provides similar conversion functionality with the
//! added flexibility of handling many container types automatically:
//! ```
//! # #[cfg(feature = "serde_with-3")]
//! # {
//! # use serde::{Deserialize, Serialize};
//! use bson::{doc, serde_helpers::object_id, oid::ObjectId};
//!
//! #[serde_with::serde_as]
//! #[derive(Deserialize, Serialize)]
//! struct Example {
//!   #[serde_as(as = "Option<object_id::AsHexString>")]
//!   optional: Option<ObjectId>,
//! }
//! # }
//! ```
//! See the crate documentation for [`serde_with`](https://docs.rs/serde_with/latest/serde_with/) for more details.

use serde::{de::Visitor, Deserialize, Serialize};
use std::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    result::Result,
};

/// Type converters for serializing and deserializing [`crate::oid::ObjectId`].
///
/// ## Available converters
/// - [`object_id::AsHexString`] — converts an [`crate::oid::ObjectId`] to and from a hex string.
/// - [`object_id::FromHexString`] — converts a hex string to and from an [`crate::oid::ObjectId`].
pub mod object_id {
    use crate::{macros::serde_conv_doc, oid::ObjectId};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    serde_conv_doc!(
        /// Converts an [`ObjectId`] to and from a hex string.
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
        /// Converts a hex string to and from an [`ObjectId`].
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

/// Type converters for serializing and deserializing [`crate::DateTime`].
///
/// ## Available converters
/// - [`datetime::AsRfc3339String`] — converts a [`crate::DateTime`] to and from an RFC 3339 string.
/// - [`datetime::FromRfc3339String`] — converts a RFC 3339 string to and from a
///   [`crate::DateTime`].
/// - [`datetime::FromI64`] — converts an `i64` to and from a [`crate::DateTime`].
/// - [`datetime::FromChrono04DateTime`] — converts a [`chrono::DateTime`] to and from a
///   [`crate::DateTime`].
/// - [`datetime::FromJiff02Timestamp`] — converts a [`jiff::Timestamp`] to and from a
///   [`crate::DateTime`].
/// - [`datetime::FromTime03OffsetDateTime`] — converts a [`time::OffsetDateTime`] to and from a
///   [`crate::DateTime`].
pub mod datetime {
    use crate::{macros::serde_conv_doc, DateTime};
    #[cfg(feature = "chrono-0_4")]
    use chrono::Utc;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    serde_conv_doc!(
        /// Converts a [`DateTime`] to and from an RFC 3339 (ISO 8601) formatted string.
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
                format!("Cannot format DateTime {} as RFC 3339 string: {}", date, e)
            })
        },
        |string: String| -> Result<DateTime, String> {
            DateTime::parse_rfc3339_str(&string).map_err(|e| format!("Cannot format RFC 3339 string {} as DateTime: {}", string, e))
        }
    );

    serde_conv_doc!(
        /// Converts an RFC 3339 (ISO 8601) formatted string to and from a [`DateTime`].
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
        pub FromRfc3339String,
        String,
        |string: &String| -> Result<DateTime, String> {
            DateTime::parse_rfc3339_str(string).map_err(|e| format!("Cannot format RFC 3339 string {} as DateTime: {}", string, e))
        },
        |date: DateTime| -> Result<String, String> {
            date.try_to_rfc3339_string().map_err(|e| {
                format!("Cannot format DateTime {} as RFC 3339 string: {}", date, e)
            })
        }
    );

    serde_conv_doc!(
        /// Converts an `i64` integer to and from a [`DateTime`].
        ///
        /// The `i64` should represent milliseconds. See [`DateTime::from_millis`] for more details.
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

    #[cfg(feature = "chrono-0_4")]
    serde_conv_doc!(
        /// Converts a [`chrono::DateTime`] to and from a [`DateTime`].
        /// ```rust
        /// # #[cfg(all(feature = "chrono-0_4", feature = "serde_with-3"))]
        /// # {
        /// use bson::serde_helpers::datetime;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Event {
        ///     #[serde_as(as = "datetime::FromChrono04DateTime")]
        ///     pub date: chrono::DateTime<chrono::Utc>,
        /// }
        /// # }
        /// ```
        pub FromChrono04DateTime,
        chrono::DateTime<Utc>,
        |chrono_date: &chrono::DateTime<Utc>| -> Result<DateTime, String> {
            Ok(DateTime::from_chrono(*chrono_date))
        },
        |bson_date: DateTime| -> Result<chrono::DateTime<Utc>, String> {
            Ok(bson_date.to_chrono())
        }
    );

    #[cfg(feature = "jiff-0_2")]
    serde_conv_doc!(
        /// Converts a [`jiff::Timestamp`] to and from a [`DateTime`].
        /// ```rust
        /// # #[cfg(all(feature = "jiff-0_2", feature = "serde_with-3"))]
        /// # {
        /// use bson::serde_helpers::datetime;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Event {
        ///     #[serde_as(as = "datetime::FromJiff02Timestamp")]
        ///     pub date: jiff::Timestamp,
        /// }
        /// # }
        /// ```
        pub FromJiff02Timestamp,
        jiff::Timestamp,
        |jiff_ts: &jiff::Timestamp| -> Result<DateTime, String> {
            Ok(DateTime::from_jiff(*jiff_ts))
        },
        |bson_date: DateTime| -> Result<jiff::Timestamp, String> {
            Ok(bson_date.to_jiff())
        }
    );

    #[cfg(feature = "time-0_3")]
    serde_conv_doc!(
        /// Converts a [`time::OffsetDateTime`] to and from a [`DateTime`].
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

/// Type converters for serializing and deserializing [`crate::Timestamp`].
///
/// ## Available converters
/// - [`timestamp::AsU32`] — converts a [`crate::Timestamp`] to and from a `u32`.
/// - [`timestamp::FromU32`] — converts a `u32` to and from a [`crate::Timestamp`].
pub mod timestamp {
    use crate::{macros::serde_conv_doc, Timestamp};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    serde_conv_doc!(
        /// Converts a [`Timestamp`] to and from a `u32`.
        ///
        /// The `u32` should represent seconds since the Unix epoch.
        ///
        /// Serialization errors if the Timestamp has a non-zero increment.
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::{serde_helpers::timestamp, Timestamp};
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "timestamp::AsU32")]
        ///     pub timestamp: Timestamp,
        /// }
        /// # }
        /// ```
        pub AsU32,
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
        /// Converts a `u32` to and from a [`Timestamp`].
        ///
        /// The `u32` should represent seconds since the Unix epoch.
        ///
        /// Deserialization errors if the Timestamp has a non-zero increment.
        /// ```rust
        /// # #[cfg(feature = "serde_with-3")]
        /// # {
        /// use bson::serde_helpers::timestamp;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Event {
        ///     #[serde_as(as = "timestamp::FromU32")]
        ///     pub time: u32,
        /// }
        /// # }
        /// ```
        pub FromU32,
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
}

/// Type converters for serializing and deserializing `u32`.
///
/// ## Available converters
/// - [`u32::AsF64`] — converts a `u32` to and from an `f64`.
/// - [`u32::AsI32`] — converts a `u32` to and from an `i32`.
/// - [`u32::AsI64`] — converts a `u32` to and from an `i64`.
pub mod u32 {
    use crate::macros::serde_conv_doc;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    serde_conv_doc!(
        /// Converts a `u32` to and from an `f64`.
        ///
        /// Deserialization errors if an exact conversion is not possible.
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
            Ok(f64::from(*value))
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
        /// Converts a `u32` to and from an `i32`.
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
        /// Converts a `u32` to and from an `i64`.
        ///
        /// Deserialization errors if an exact conversion is not possible.
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
            Ok(i64::from(*value))
        },
        |value: i64| -> Result<u32, String> {
            u32::try_from(value).map_err(|e| format!("Cannot convert i64 {} to u32: {}", value, e))
        }
    );
}

/// Type converters for serializing and deserializing `u64`.
///
/// ## Available converters
/// - [`u64::AsF64`] — converts a `u64` to and from an `f64`.
/// - [`u64::AsI32`] — converts a `u64` to and from an `i32`.
/// - [`u64::AsI64`] — converts a `u64` to and from an `i64`.
pub mod u64 {
    use crate::macros::serde_conv_doc;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    serde_conv_doc!(
        /// Converts a `u64` to and from an `f64`.
        ///
        /// Errors if an exact conversion is not possible.
        ///
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
        /// Converts a `u64` to and from an `i32`.
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
        /// Converts a `u64` to and from an `i64`.
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

/// Type converters for serializing and deserializing [`uuid::Uuid`].
///
/// ## Available converters
/// - [`uuid_1::FromBson`] - serializes a [`crate::Uuid`] as a [`uuid::Uuid`].
/// - [`uuid_1::AsBinary`] — serializes a [`uuid::Uuid`] as a [`crate::Binary`].
/// - [`uuid_1::AsCSharpLegacyBinary`] — serializes a [`uuid::Uuid`] as a [`crate::Binary`] in the
///   legacy C# driver UUID format.
/// - [`uuid_1::AsJavaLegacyBinary`] — serializes a [`uuid::Uuid`] as a [`crate::Binary`] in the
///   legacy Java driver UUID format.
/// - [`uuid_1::AsPythonLegacyBinary`] — serializes a [`uuid::Uuid`] as a [`crate::Binary`] in the
///   legacy Python driver UUID format.
#[cfg(feature = "uuid-1")]
pub mod uuid_1 {
    use crate::macros::serde_conv_doc;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use uuid::Uuid;

    serde_conv_doc!(
        /// Converts a [`crate::Uuid`] to and from a [`uuid::Uuid`].
        /// ```
        /// # #[cfg(all(feature = "uuid-1", feature = "serde_with-3"))]
        /// # {
        /// use bson::serde_helpers::uuid_1;
        /// use serde::{Serialize, Deserialize};
        /// use serde_with::serde_as;
        /// #[serde_as]
        /// #[derive(Serialize, Deserialize)]
        /// struct Item {
        ///     #[serde_as(as = "uuid_1::FromBson")]
        ///     pub id: bson::Uuid,
        /// }
        /// # }
        /// ```
        pub FromBson,
        crate::Uuid,
        |bson_uuid: &crate::Uuid| -> Result<Uuid, String> {
            Ok((*bson_uuid).into())
        },
        |uuid: Uuid| -> Result<crate::Uuid, String> {
            Ok(crate::Uuid::from(uuid))
        }
    );

    serde_conv_doc!(
        /// Serializes a [`Uuid`] as a [`crate::Binary`] and deserializes a [`Uuid`] from a [`crate::Binary`].
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
        /// Serializes a [`Uuid`] to a [`crate::Binary`] in the legacy C# driver UUID format and
        /// deserializes [`Uuid`] from a [`crate::Binary`] in the legacy C# driver format.
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
        /// Serializes a [`Uuid`] to a [`crate::Binary`] in the legacy Java driver UUID format and
        /// deserializes [`Uuid`] from a [`crate::Binary`] in the legacy Java driver format.
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
        /// Serializes a [`Uuid`] to a [`crate::Binary`] in the legacy Python driver UUID format and
        /// deserializes [`Uuid`] from a [`crate::Binary`] in the legacy Python driver format.
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
