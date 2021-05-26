use std::{
    fmt::{self, Display},
    time::{Duration, SystemTime},
};

use chrono::{LocalResult, TimeZone, Utc};

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
/// This type differs from [`chrono::DateTime`] in that it serializes to and deserializes from a
/// BSON datetime rather than an RFC 3339 formatted string. Additionally, in non-BSON formats, it
/// will serialize to and deserialize from that format's equivalent of the
/// [extended JSON representation](https://docs.mongodb.com/manual/reference/mongodb-extended-json/) of a datetime.
/// To serialize a [`chrono::DateTime`] as a BSON datetime, you can use
/// [`serde_helpers::chrono_datetime_as_bson_datetime`].
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
/// }
/// # }
/// ```
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
pub struct DateTime(i64);

impl crate::DateTime {
    /// The latest possibile date that can be represented in BSON.
    pub const MAX: Self = Self(i64::MAX);

    /// The earliest possibile date that can be represented in BSON.
    pub const MIN: Self = Self(i64::MIN);

    /// Makes a new [`DateTime`] from the number of non-leap milliseconds since
    /// January 1, 1970 0:00:00 UTC (aka "UNIX timestamp").
    pub fn from_millis(date: i64) -> Self {
        Self(date)
    }

    /// Returns a [`DateTime`] which corresponds to the current date and time.
    pub fn now() -> DateTime {
        Self::from_chrono(Utc::now())
    }

    #[cfg(not(feature = "chrono-0_4"))]
    pub(crate) fn from_chrono<T: chrono::TimeZone>(dt: chrono::DateTime<T>) -> Self {
        Self::from_millis(dt.timestamp_millis())
    }

    /// Convert the given `chrono::DateTime` into a `bson::DateTime`, truncating it to millisecond
    /// precision.
    #[cfg(feature = "chrono-0_4")]
    pub fn from_chrono<T: chrono::TimeZone>(dt: chrono::DateTime<T>) -> Self {
        Self::from_millis(dt.timestamp_millis())
    }

    fn to_chrono_private(self) -> chrono::DateTime<Utc> {
        match Utc.timestamp_millis_opt(self.0) {
            LocalResult::Single(dt) => dt,
            _ => {
                if self.0 < 0 {
                    chrono::MIN_DATETIME
                } else {
                    chrono::MAX_DATETIME
                }
            }
        }
    }

    #[cfg(not(feature = "chrono-0_4"))]
    #[allow(unused)]
    pub(crate) fn to_chrono(self) -> chrono::DateTime<Utc> {
        self.to_chrono_private()
    }

    /// Convert this [`DateTime`] to a [`chrono::DateTime<Utc>`].
    ///
    /// Note: Not every BSON datetime can be represented as a [`chrono::DateTime`]. For such dates,
    /// [`chrono::MIN_DATETIME`] or [`chrono::MAX_DATETIME`] will be returned, whichever is closer.
    ///
    /// ```
    /// let bson_dt = bson::DateTime::now();
    /// let chrono_dt = bson_dt.to_chrono();
    /// assert_eq!(bson_dt.timestamp_millis(), chrono_dt.timestamp_millis());
    ///
    /// let big = bson::DateTime::from_millis(i64::MAX);
    /// let chrono_big = big.to_chrono();
    /// assert_eq!(chrono_big, chrono::MAX_DATETIME)
    /// ```
    #[cfg(feature = "chrono-0_4")]
    pub fn to_chrono(self) -> chrono::DateTime<Utc> {
        self.to_chrono_private()
    }

    /// Convert the given [`std::SystemTime`] to a [`DateTime`].
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

    /// Convert this [`DateTime`] to a [`std::SystemTime`].
    pub fn to_system_time(self) -> SystemTime {
        if self.0 >= 0 {
            SystemTime::UNIX_EPOCH + Duration::from_millis(self.0 as u64)
        } else {
            // need to convert to i128 beforce calculating absolute value since i64::MIN.abs()
            // overflows and panics.
            SystemTime::UNIX_EPOCH - Duration::from_millis((self.0 as i128).abs() as u64)
        }
    }

    /// Returns the number of non-leap-milliseconds since January 1, 1970 UTC.
    pub fn timestamp_millis(&self) -> i64 {
        self.0
    }

    pub(crate) fn to_rfc3339(&self) -> String {
        self.to_chrono()
            .to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
    }
}

impl fmt::Debug for crate::DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut tup = f.debug_tuple("DateTime");
        match Utc.timestamp_millis_opt(self.0) {
            LocalResult::Single(ref dt) => tup.field(dt),
            _ => tup.field(&self.0),
        };
        tup.finish()
    }
}

impl Display for crate::DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match Utc.timestamp_millis_opt(self.0) {
            LocalResult::Single(ref dt) => Display::fmt(dt, f),
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
