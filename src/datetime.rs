//! Module containing functionality related to BSON DateTimes.
//! For more information, see the documentation for the [`DateTime`] type.
pub(crate) mod builder;

use std::{
    convert::TryInto,
    fmt::{self, Display},
    time::{Duration, SystemTime},
};

#[cfg(feature = "chrono-0_4")]
use chrono::{LocalResult, TimeZone, Utc};
use time::format_description::well_known::Rfc3339;

pub use crate::datetime::builder::DateTimeBuilder;
use crate::error::{Error, Result};

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
/// # fn main() -> bson::error::Result<()> {
/// let f = Foo { date_time: bson::DateTime::now(), chrono_datetime: chrono::Utc::now() };
/// println!("{:?}", bson::serialize_to_document(&f)?);
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
/// various datetime types to and from different formats using the [`serde_with`](https://docs.rs/serde_with/latest/serde_with/)
/// crate.
///
/// > **Note:** All helpers in this module require use of the [`#[serde_as]`](https://docs.rs/serde_with/latest/serde_with/attr.serde_as.html)
/// > attribute on the struct. This enables the enhanced serialization behavior provided by
/// > `serde_with-3`.
///
/// For example, to serialize a [`chrono::DateTime`] as a BSON datetime, you can use
/// [`crate::serde_helpers::datetime::FromChrono04DateTime`].
/// Similarly, to serialize a BSON [`DateTime`] to a string, you can use
/// [`crate::serde_helpers::datetime::AsRfc3339String`]. Check out the
/// [`crate::serde_helpers`] module documentation for a list of all of the helpers
/// offered by the crate.
/// ```rust
/// # #[cfg(all(feature = "chrono-0_4", feature = "serde_with-3"))]
/// # {
/// use serde::{Serialize, Deserialize};
/// use serde_with::serde_as;
/// use bson::serde_helpers::datetime;
///
/// #[serde_as]
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
///     #[serde_as(as = "datetime::FromChrono04DateTime")]
///     chrono_as_bson: chrono::DateTime<chrono::Utc>,
///
///     // serializes as an RFC 3339 / ISO-8601 string.
///     // this requires the "serde_with-3" feature flag
///     #[serde_as(as = "datetime::AsRfc3339String")]
///     bson_as_string: bson::DateTime,
/// }
/// # }
/// ```
/// The main benefit of using the [`serde_with`](https://docs.rs/serde_with/latest/serde_with/) crate
/// is that it can handle nested [`chrono::DateTime`] values (e.g. in [`Option`] or [`Vec`]).
/// ```
/// # #[cfg(all(feature = "chrono-0_4", feature = "serde_with-3"))]
/// # {
/// use serde::{Deserialize, Serialize};
/// use serde_with::serde_as;
/// use bson::{doc, serde_helpers::datetime};
///
/// #[serde_as]
/// #[derive(Deserialize, Serialize, PartialEq, Debug)]
/// struct Foo {
///   /// serializes as a BSON datetime rather than using [`chrono::DateTime`]'s serialization
///   #[serde_as(as = "Option<datetime::FromChrono04DateTime>")]
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
/// assert_eq!(bson::serialize_to_document(&foo)?, expected);
/// # }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ## Large Dates
/// The range of dates supported by `DateTime` is defined by [`DateTime::MIN`] and
/// [`DateTime::MAX`]. However, some utilities for constructing and converting `DateTimes`, such as
/// interop with the [`time::OffsetDateTime`] type and with RFC 3339 strings, are bounded by the
/// [`time`] crate's supported date range. The `large_dates` feature can be enabled to expand this
/// range, which enables the
/// [`large-dates` feature for `time`](https://docs.rs/time/latest/time/#feature-flags).
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
    pub fn from_chrono<T: chrono::TimeZone>(dt: chrono::DateTime<T>) -> Self {
        Self::from_millis(dt.timestamp_millis())
    }

    /// Convert the given [`jiff::Timestamp`] into a [`bson::DateTime`](DateTime), truncating it to
    /// millisecond precision.
    #[cfg(feature = "jiff-0_2")]
    pub fn from_jiff(ts: jiff::Timestamp) -> Self {
        Self::from_millis(ts.as_millisecond())
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

    /// Convert this [`DateTime`] to a [`jiff::Timestamp`].
    ///
    /// Note: Not every BSON datetime can be represented as a [`jiff::Timestamp`]. For such dates,
    /// [`jiff::Timestamp::MIN`] or [`jiff::Timestamp::MAX`] will be returned, whichever
    /// is closer.
    ///
    /// ```
    /// let bson_dt = bson::DateTime::now();
    /// let jiff_ts = bson_dt.to_jiff();
    /// assert_eq!(bson_dt.timestamp_millis(), jiff_ts.as_millisecond());
    ///
    /// let big = bson::DateTime::from_millis(i64::MAX);
    /// let jiff_big = big.to_jiff();
    /// assert_eq!(jiff_big, jiff::Timestamp::MAX)
    /// ```
    #[cfg(feature = "jiff-0_2")]
    pub fn to_jiff(self) -> jiff::Timestamp {
        jiff::Timestamp::from_millisecond(self.0).unwrap_or({
            if self.0 < 0 {
                jiff::Timestamp::MIN
            } else {
                jiff::Timestamp::MAX
            }
        })
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

    /// Adds `millis` milliseconds to the [`DateTime`] saturating at [`DateTime::MIN`] and
    /// [`DateTime::MAX`].
    pub const fn saturating_add_millis(self, millis: i64) -> Self {
        Self::from_millis(self.0.saturating_add(millis))
    }

    /// Adds `duration` to the [`DateTime`] saturating at [`DateTime::MAX`].
    ///
    /// As [`DateTime`] only have millisecond-precision this will only use the whole milliseconds
    /// of `duration`.
    pub const fn saturating_add_duration(self, duration: Duration) -> Self {
        // i64::try_from isn't const
        let millis = duration.as_millis();
        if millis > i64::MAX as u128 {
            Self::from_millis(i64::MAX)
        } else {
            self.saturating_add_millis(millis as i64)
        }
    }

    /// Convert this [`DateTime`] to an RFC 3339 formatted string.
    pub fn try_to_rfc3339_string(self) -> Result<String> {
        self.to_time_0_3().format(&Rfc3339).map_err(Error::datetime)
    }

    /// Convert the given RFC 3339 formatted string to a [`DateTime`], truncating it to millisecond
    /// precision.
    pub fn parse_rfc3339_str(s: impl AsRef<str>) -> Result<Self> {
        let odt = time::OffsetDateTime::parse(s.as_ref(), &Rfc3339).map_err(Error::datetime)?;
        Ok(Self::from_time_0_3(odt))
    }

    /// Returns the time elapsed since `earlier`, or `None` if the given `DateTime` is later than
    /// this one.
    pub fn checked_duration_since(self, earlier: Self) -> Option<Duration> {
        if earlier.0 > self.0 {
            return None;
        }
        Some(Duration::from_millis((self.0 - earlier.0) as u64))
    }

    /// Returns the time elapsed since `earlier`, or a [`Duration`] of zero if the given `DateTime`
    /// is later than this one.
    pub fn saturating_duration_since(self, earlier: Self) -> Duration {
        self.checked_duration_since(earlier)
            .unwrap_or(Duration::ZERO)
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
impl From<crate::DateTime> for chrono::DateTime<Utc> {
    fn from(bson_dt: DateTime) -> Self {
        bson_dt.to_chrono()
    }
}

#[cfg(feature = "chrono-0_4")]
impl<T: chrono::TimeZone> From<chrono::DateTime<T>> for crate::DateTime {
    fn from(x: chrono::DateTime<T>) -> Self {
        Self::from_chrono(x)
    }
}

#[cfg(feature = "jiff-0_2")]
impl From<crate::DateTime> for jiff::Timestamp {
    fn from(bson_dt: DateTime) -> Self {
        bson_dt.to_jiff()
    }
}

#[cfg(feature = "jiff-0_2")]
impl From<jiff::Timestamp> for crate::DateTime {
    fn from(x: jiff::Timestamp) -> Self {
        Self::from_jiff(x)
    }
}

#[cfg(feature = "time-0_3")]
impl From<crate::DateTime> for time::OffsetDateTime {
    fn from(bson_dt: DateTime) -> Self {
        bson_dt.to_time_0_3()
    }
}

#[cfg(feature = "time-0_3")]
impl From<time::OffsetDateTime> for crate::DateTime {
    fn from(x: time::OffsetDateTime) -> Self {
        Self::from_time_0_3(x)
    }
}
