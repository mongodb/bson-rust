//! Module containing functionality related to BSON DateTimes.
//! For more information, see the documentation for the [`DateTime`] type.

use std::{
    convert::TryInto,
    error,
    fmt::{self, Display},
    result,
    time::{Duration, SystemTime},
};

pub(crate) mod builder;
pub use crate::datetime::builder::DateTimeBuilder;
use time::format_description::well_known::Rfc3339;

#[cfg(feature = "chrono-0_4")]
use chrono::{LocalResult, TimeZone, Utc};
#[cfg(all(
    feature = "serde_with",
    any(feature = "chrono-0_4", feature = "time-0_3")
))]
use serde::{Deserialize, Deserializer, Serialize};
#[cfg(all(
    feature = "serde_with",
    any(feature = "chrono-0_4", feature = "time-0_3")
))]
use serde_with::{DeserializeAs, SerializeAs};

/// Struct representing a BSON datetime.
/// Note: BSON datetimes have millisecond precision.
///
/// To enable conversions between this type and [`chrono::DateTime`], enable the `"chrono-0_4"`
/// feature flag in your `Cargo.toml`.
/// ```
/// use chrono::prelude::*;
/// # fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
/// # #[cfg(feature = "chrono-0_4")]
/// # {
/// let chrono_dt: chrono::DateTime<Utc> = "2014-11-28T12:00:09Z".parse()?;
/// let bson_dt: bson::DateTime = chrono_dt.into();
/// let bson_dt = bson::DateTime::from_chrono(chrono_dt);
/// let back_to_chrono: chrono::DateTime<Utc> = bson_dt.into();
/// let back_to_chrono = bson_dt.to_chrono();
/// # }
/// # Ok(())
/// # }
/// ```
///
/// You may also construct this type from a given `year`, `month`, `day`, and optionally,
/// an `hour`, `minute`, `second` and `millisecond`, which default to 0 if not explicitly set.
///
/// ```
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let dt = bson::DateTime::builder().year(1998).month(2).day(12).minute(1).millisecond(23).build()?;
/// let expected = bson::DateTime::parse_rfc3339_str("1998-02-12T00:01:00.023Z")?;
/// assert_eq!(dt, expected);
/// # Ok(())
/// # }
/// ```
///
/// ## Serde integration
///
/// This type differs from [`chrono::DateTime`] in that it serializes to and deserializes from a
/// BSON datetime rather than an RFC 3339 formatted string.
///
/// e.g.
/// ```rust
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Foo {
///     // serializes as a BSON datetime.
///     date_time: bson::DateTime,
///
///     // serializes as an RFC 3339 / ISO-8601 string.
///     chrono_datetime: chrono::DateTime<chrono::Utc>,
/// }
///
/// # fn main() -> bson::ser::Result<()> {
/// let f = Foo { date_time: bson::DateTime::now(), chrono_datetime: chrono::Utc::now() };
/// println!("{:?}", bson::to_document(&f)?);
/// # Ok(())
/// # }
/// ```
/// Produces the following output:
/// ```js
/// { "date_time": DateTime("2023-01-23 20:11:47.316 +00:00:00"), "chrono_datetime": "2023-01-23T20:11:47.316114543Z" }
/// ```
///
/// Additionally, in non-BSON formats, it will serialize to and deserialize from that format's
/// equivalent of the [extended JSON representation](https://www.mongodb.com/docs/manual/reference/mongodb-extended-json/) of a datetime.
///
/// e.g.
/// ```rust
/// # use serde::Serialize;
/// # #[derive(Serialize)]
/// # struct Foo {
/// #    // serializes as a BSON datetime.
/// #    date_time: bson::DateTime,
/// #
/// #   // serializes as an RFC 3339 / ISO-8601 string.
/// #   chrono_datetime: chrono::DateTime<chrono::Utc>,
/// # }
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let f = Foo { date_time: bson::DateTime::now(), chrono_datetime: chrono::Utc::now() };
/// println!("{}", serde_json::to_string(&f)?);
/// # Ok(())
/// # }
/// ```
/// Produces the following output:
/// ```js
/// {"date_time":{"$date":{"$numberLong":"1674504029491"}},"chrono_datetime":"2023-01-23T20:00:29.491791120Z"}
/// ```
///
/// ### `serde_helpers`
/// The `bson` crate provides a number of useful helpers for serializing and deserializing
/// various datetime types to and from different formats. For example, to serialize a
/// [`chrono::DateTime`] as a BSON datetime, you can use
/// [`crate::serde_helpers::chrono_datetime_as_bson_datetime`]. Similarly, to serialize a BSON
/// [`DateTime`] to a string, you can use [`crate::serde_helpers::bson_datetime_as_rfc3339_string`].
/// Check out the [`crate::serde_helpers`] module documentation for a list of all of the helpers
/// offered by the crate.
///
/// ```rust
/// # #[cfg(feature = "chrono-0_4")]
/// # {
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize)]
/// struct Foo {
///     // serializes as a BSON datetime.
///     date_time: bson::DateTime,
///
///     // serializes as an RFC 3339 / ISO-8601 string.
///     chrono_datetime: chrono::DateTime<chrono::Utc>,
///
///     // serializes as a BSON datetime.
///     // this requires the "chrono-0_4" feature flag
///     #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
///     chrono_as_bson: chrono::DateTime<chrono::Utc>,
///
///     // serializes as an RFC 3339 / ISO-8601 string.
///     #[serde(with = "bson::serde_helpers::bson_datetime_as_rfc3339_string")]
///     bson_as_string: bson::DateTime,
/// }
/// # }
/// ```
/// ### The `serde_with` feature flag
///
/// The `serde_with` feature can be enabled to support more ergonomic serde attributes for
/// (de)serializing [`chrono::DateTime`] from/to BSON via the [`serde_with`](https://docs.rs/serde_with/1.11.0/serde_with/)
/// crate. The main benefit of this compared to the regular `serde_helpers` is that `serde_with` can
/// handle nested [`chrono::DateTime`] values (e.g. in [`Option`]), whereas the former only works on
/// fields that are exactly [`chrono::DateTime`].
/// ```
/// # #[cfg(all(feature = "chrono-0_4", feature = "serde_with"))]
/// # {
/// use serde::{Deserialize, Serialize};
/// use bson::doc;
///
/// #[serde_with::serde_as]
/// #[derive(Deserialize, Serialize, PartialEq, Debug)]
/// struct Foo {
///   /// Serializes as a BSON datetime rather than using [`chrono::DateTime`]'s serialization
///   #[serde_as(as = "Option<bson::DateTime>")]
///   as_bson: Option<chrono::DateTime<chrono::Utc>>,
/// }
///
/// let dt = chrono::Utc::now();
/// let foo = Foo {
///   as_bson: Some(dt),
/// };
///
/// let expected = doc! {
///   "as_bson": bson::DateTime::from_chrono(dt),
/// };
///
/// assert_eq!(bson::to_document(&foo)?, expected);
/// # }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
pub struct DateTime(i64);

impl crate::DateTime {
    /// The latest possible date that can be represented in BSON.
    pub const MAX: Self = Self::from_millis(i64::MAX);

    /// The earliest possible date that can be represented in BSON.
    pub const MIN: Self = Self::from_millis(i64::MIN);

    /// Makes a new [`DateTime`] from the number of non-leap milliseconds since
    /// January 1, 1970 0:00:00 UTC (aka "UNIX timestamp").
    pub const fn from_millis(date: i64) -> Self {
        Self(date)
    }

    /// Returns a [`DateTime`] which corresponds to the current date and time.
    pub fn now() -> DateTime {
        Self::from_system_time(SystemTime::now())
    }

    /// Convert the given [`chrono::DateTime`] into a [`bson::DateTime`](DateTime), truncating it to
    /// millisecond precision.
    #[cfg(feature = "chrono-0_4")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono-0_4")))]
    pub fn from_chrono<T: chrono::TimeZone>(dt: chrono::DateTime<T>) -> Self {
        Self::from_millis(dt.timestamp_millis())
    }

    /// Returns a builder used to construct a [`DateTime`] from a given year, month,
    /// day, and optionally, an hour, minute, second and millisecond, which default to
    /// 0 if not explicitly set.
    ///
    /// Note: You cannot call `build()` before setting at least the year, month and day.
    pub fn builder() -> DateTimeBuilder {
        DateTimeBuilder::default()
    }

    /// Convert this [`DateTime`] to a [`chrono::DateTime<Utc>`].
    ///
    /// Note: Not every BSON datetime can be represented as a [`chrono::DateTime`]. For such dates,
    /// [`chrono::DateTime::MIN_UTC`] or [`chrono::DateTime::MAX_UTC`] will be returned, whichever
    /// is closer.
    ///
    /// ```
    /// let bson_dt = bson::DateTime::now();
    /// let chrono_dt = bson_dt.to_chrono();
    /// assert_eq!(bson_dt.timestamp_millis(), chrono_dt.timestamp_millis());
    ///
    /// let big = bson::DateTime::from_millis(i64::MAX);
    /// let chrono_big = big.to_chrono();
    /// assert_eq!(chrono_big, chrono::DateTime::<chrono::Utc>::MAX_UTC)
    /// ```
    #[cfg(feature = "chrono-0_4")]
    #[cfg_attr(docsrs, doc(cfg(feature = "chrono-0_4")))]
    pub fn to_chrono(self) -> chrono::DateTime<Utc> {
        match Utc.timestamp_millis_opt(self.0) {
            LocalResult::Single(dt) => dt,
            _ => {
                if self.0 < 0 {
                    chrono::DateTime::<Utc>::MIN_UTC
                } else {
                    chrono::DateTime::<Utc>::MAX_UTC
                }
            }
        }
    }

    fn from_time_private(dt: time::OffsetDateTime) -> Self {
        let millis = dt.unix_timestamp_nanos() / 1_000_000;
        match millis.try_into() {
            Ok(ts) => Self::from_millis(ts),
            _ => {
                if millis > 0 {
                    Self::MAX
                } else {
                    Self::MIN
                }
            }
        }
    }

    #[cfg(not(feature = "time-0_3"))]
    #[allow(unused)]
    pub(crate) fn from_time_0_3(dt: time::OffsetDateTime) -> Self {
        Self::from_time_private(dt)
    }

    /// Convert the given [`time::OffsetDateTime`] into a [`bson::DateTime`](DateTime), truncating
    /// it to millisecond precision.
    ///
    /// If the provided time is too far in the future or too far in the past to be represented
    /// by a BSON datetime, either [`DateTime::MAX`] or [`DateTime::MIN`] will be
    /// returned, whichever is closer.
    #[cfg(feature = "time-0_3")]
    pub fn from_time_0_3(dt: time::OffsetDateTime) -> Self {
        Self::from_time_private(dt)
    }

    fn to_time_private(self) -> time::OffsetDateTime {
        match self.to_time_opt() {
            Some(dt) => dt,
            None => if self.0 < 0 {
                time::PrimitiveDateTime::MIN
            } else {
                time::PrimitiveDateTime::MAX
            }
            .assume_utc(),
        }
    }

    pub(crate) fn to_time_opt(self) -> Option<time::OffsetDateTime> {
        time::OffsetDateTime::UNIX_EPOCH.checked_add(time::Duration::milliseconds(self.0))
    }

    #[cfg(not(feature = "time-0_3"))]
    #[allow(unused)]
    pub(crate) fn to_time_0_3(self) -> time::OffsetDateTime {
        self.to_time_private()
    }

    /// Convert this [`DateTime`] to a [`time::OffsetDateTime`].
    ///
    /// Note: Not every BSON datetime can be represented as a [`time::OffsetDateTime`]. For such
    /// dates, [`time::PrimitiveDateTime::MIN`] or [`time::PrimitiveDateTime::MAX`] will be
    /// returned, whichever is closer.
    ///
    /// ```
    /// let bson_dt = bson::DateTime::now();
    /// let time_dt = bson_dt.to_time_0_3();
    /// assert_eq!(bson_dt.timestamp_millis() / 1000, time_dt.unix_timestamp());
    ///
    /// let big = bson::DateTime::from_millis(i64::MIN);
    /// let time_big = big.to_time_0_3();
    /// assert_eq!(time_big, time::PrimitiveDateTime::MIN.assume_utc())
    /// ```
    #[cfg(feature = "time-0_3")]
    #[cfg_attr(docsrs, doc(cfg(feature = "time-0_3")))]
    pub fn to_time_0_3(self) -> time::OffsetDateTime {
        self.to_time_private()
    }

    /// Convert the given [`std::time::SystemTime`] to a [`DateTime`].
    ///
    /// If the provided time is too far in the future or too far in the past to be represented
    /// by a BSON datetime, either [`DateTime::MAX`] or [`DateTime::MIN`] will be
    /// returned, whichever is closer.
    pub fn from_system_time(st: SystemTime) -> Self {
        match st.duration_since(SystemTime::UNIX_EPOCH) {
            Ok(d) => {
                if d.as_millis() <= i64::MAX as u128 {
                    Self::from_millis(d.as_millis() as i64)
                } else {
                    Self::MAX
                }
            }
            // handle SystemTime from before the Unix Epoch
            Err(e) => {
                let millis = e.duration().as_millis();
                if millis > i64::MAX as u128 {
                    Self::MIN
                } else {
                    Self::from_millis(-(millis as i64))
                }
            }
        }
    }

    /// Convert this [`DateTime`] to a [`std::time::SystemTime`].
    pub fn to_system_time(self) -> SystemTime {
        if self.0 >= 0 {
            SystemTime::UNIX_EPOCH + Duration::from_millis(self.0 as u64)
        } else {
            // need to convert to i128 before calculating absolute value since i64::MIN.abs()
            // overflows and panics.
            SystemTime::UNIX_EPOCH - Duration::from_millis((self.0 as i128).unsigned_abs() as u64)
        }
    }

    /// Returns the number of non-leap-milliseconds since January 1, 1970 UTC.
    pub const fn timestamp_millis(self) -> i64 {
        self.0
    }

    #[deprecated(since = "2.3.0", note = "Use try_to_rfc3339_string instead.")]
    /// Convert this [`DateTime`] to an RFC 3339 formatted string.  Panics if it could not be
    /// represented in that format.
    pub fn to_rfc3339_string(self) -> String {
        self.try_to_rfc3339_string().unwrap()
    }

    /// Convert this [`DateTime`] to an RFC 3339 formatted string.
    pub fn try_to_rfc3339_string(self) -> Result<String> {
        self.to_time_0_3()
            .format(&Rfc3339)
            .map_err(|e| Error::CannotFormat {
                message: e.to_string(),
            })
    }

    /// Convert the given RFC 3339 formatted string to a [`DateTime`], truncating it to millisecond
    /// precision.
    pub fn parse_rfc3339_str(s: impl AsRef<str>) -> Result<Self> {
        let odt = time::OffsetDateTime::parse(s.as_ref(), &Rfc3339).map_err(|e| {
            Error::InvalidTimestamp {
                message: e.to_string(),
            }
        })?;
        Ok(Self::from_time_0_3(odt))
    }
}

impl fmt::Debug for crate::DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tup = f.debug_tuple("DateTime");
        match self.to_time_opt() {
            Some(dt) => tup.field(&dt),
            _ => tup.field(&self.0),
        };
        tup.finish()
    }
}

impl Display for crate::DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_time_opt() {
            Some(dt) => Display::fmt(&dt, f),
            _ => Display::fmt(&self.0, f),
        }
    }
}

impl From<SystemTime> for crate::DateTime {
    fn from(st: SystemTime) -> Self {
        Self::from_system_time(st)
    }
}

impl From<crate::DateTime> for SystemTime {
    fn from(dt: crate::DateTime) -> Self {
        dt.to_system_time()
    }
}

#[cfg(feature = "chrono-0_4")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono-0_4")))]
impl From<crate::DateTime> for chrono::DateTime<Utc> {
    fn from(bson_dt: DateTime) -> Self {
        bson_dt.to_chrono()
    }
}

#[cfg(feature = "chrono-0_4")]
#[cfg_attr(docsrs, doc(cfg(feature = "chrono-0_4")))]
impl<T: chrono::TimeZone> From<chrono::DateTime<T>> for crate::DateTime {
    fn from(x: chrono::DateTime<T>) -> Self {
        Self::from_chrono(x)
    }
}

#[cfg(all(feature = "chrono-0_4", feature = "serde_with"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "chrono-0_4", feature = "serde_with"))))]
impl<'de> DeserializeAs<'de, chrono::DateTime<Utc>> for crate::DateTime {
    fn deserialize_as<D>(deserializer: D) -> std::result::Result<chrono::DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let dt = DateTime::deserialize(deserializer)?;
        Ok(dt.to_chrono())
    }
}

#[cfg(all(feature = "chrono-0_4", feature = "serde_with"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "chrono-0_4", feature = "chrono-0_4"))))]
impl SerializeAs<chrono::DateTime<Utc>> for crate::DateTime {
    fn serialize_as<S>(
        source: &chrono::DateTime<Utc>,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let dt = DateTime::from_chrono(*source);
        dt.serialize(serializer)
    }
}

#[cfg(feature = "time-0_3")]
#[cfg_attr(docsrs, doc(cfg(feature = "time-0_3")))]
impl From<crate::DateTime> for time::OffsetDateTime {
    fn from(bson_dt: DateTime) -> Self {
        bson_dt.to_time_0_3()
    }
}

#[cfg(feature = "time-0_3")]
#[cfg_attr(docsrs, doc(cfg(feature = "time-0_3")))]
impl From<time::OffsetDateTime> for crate::DateTime {
    fn from(x: time::OffsetDateTime) -> Self {
        Self::from_time_0_3(x)
    }
}

#[cfg(all(feature = "time-0_3", feature = "serde_with"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "time-0_3", feature = "serde_with"))))]
impl<'de> DeserializeAs<'de, time::OffsetDateTime> for crate::DateTime {
    fn deserialize_as<D>(deserializer: D) -> std::result::Result<time::OffsetDateTime, D::Error>
    where
        D: Deserializer<'de>,
    {
        let dt = DateTime::deserialize(deserializer)?;
        Ok(dt.to_time_0_3())
    }
}

#[cfg(all(feature = "time-0_3", feature = "serde_with"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "time-0_3", feature = "chrono-0_4"))))]
impl SerializeAs<time::OffsetDateTime> for crate::DateTime {
    fn serialize_as<S>(
        source: &time::OffsetDateTime,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let dt = DateTime::from_time_0_3(*source);
        dt.serialize(serializer)
    }
}

/// Errors that can occur during [`DateTime`] construction and generation.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Error returned when an invalid datetime format is provided to a conversion method.
    #[non_exhaustive]
    InvalidTimestamp { message: String },
    /// Error returned when a [`DateTime`] cannot be represented in a particular format.
    #[non_exhaustive]
    CannotFormat { message: String },
}

/// Alias for `Result<T, DateTime::Error>`
pub type Result<T> = result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::InvalidTimestamp { message } | Error::CannotFormat { message } => {
                write!(fmt, "{}", message)
            }
        }
    }
}

impl error::Error for Error {}
