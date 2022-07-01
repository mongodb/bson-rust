use super::*;
use std::convert::TryFrom;
use time::Date;

/// Builder for constructing a BSON [`DateTime`]
pub struct DateTimeBuilder<Y = NoYear, M = NoMonth, D = NoDay> {
    pub(crate) year: Y,
    pub(crate) month: M,
    pub(crate) day: D,

    pub(crate) hour: Option<u8>,
    pub(crate) minute: Option<u8>,
    pub(crate) second: Option<u8>,
    pub(crate) millisecond: Option<u16>,
}

impl Default for DateTimeBuilder {
    fn default() -> Self {
        Self {
            year: NoYear,
            month: NoMonth,
            day: NoDay,
            hour: None,
            minute: None,
            second: None,
            millisecond: None,
        }
    }
}

pub struct Year(i32);
pub struct NoYear;

pub struct Month(u8);
pub struct NoMonth;

pub struct Day(u8);
pub struct NoDay;

impl<M, D> DateTimeBuilder<NoYear, M, D> {
    /// Sets the year for the builder instance. Years between ±9999 inclusive are valid.
    /// If the specified value is out of range, calling the `build()` method will return
    /// an error.
    ///
    /// Note: This is a required method. You will not be able to call `build()` before calling
    /// this method.
    pub fn year(self, y: i32) -> DateTimeBuilder<Year, M, D> {
        let Self {
            year: _,
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
        } = self;
        DateTimeBuilder {
            year: Year(y),
            month,
            day,
            hour,
            minute,
            second,
            millisecond,
        }
    }
}

impl<Y, D> DateTimeBuilder<Y, NoMonth, D> {
    /// Sets the month for the builder instance. Maps months as 1-January to 12-December.
    /// If the specified value is out of range, calling the `build()` method will return
    /// an error.
    ///
    /// Note: This is a required method. You will not be able to call `build()` before calling
    /// this method.
    pub fn month(self, m: u8) -> DateTimeBuilder<Y, Month, D> {
        let Self {
            year,
            month: _,
            day,
            hour,
            minute,
            second,
            millisecond,
        } = self;
        DateTimeBuilder {
            year,
            month: Month(m),
            day,
            hour,
            minute,
            second,
            millisecond,
        }
    }
}

impl<Y, M> DateTimeBuilder<Y, M, NoDay> {
    /// Sets the day for the builder instance. Values in the range `1..=31` are valid.
    /// If the specified value does not exist for the provided month/year or is out of range,
    /// calling the `build()` method will return an error.
    ///
    /// Note: This is a required method. You will not be able to call `build()` before calling
    /// this method.
    pub fn day(self, d: u8) -> DateTimeBuilder<Y, M, Day> {
        let Self {
            year,
            month,
            day: _,
            hour,
            minute,
            second,
            millisecond,
        } = self;
        DateTimeBuilder {
            year,
            month,
            day: Day(d),
            hour,
            minute,
            second,
            millisecond,
        }
    }
}

impl<Y, M, D> DateTimeBuilder<Y, M, D> {
    /// Sets the hour (24-hour format) for the builder instance. Values must be in the range
    /// `0..=23`. If the specified value is out of range, calling the `build()` method will
    /// return an error.
    ///
    /// Note: This is an optional method. The hour will default to 0 if not explicitly set.
    pub fn hour(mut self, hour: u8) -> DateTimeBuilder<Y, M, D> {
        self.hour = Some(hour);
        self
    }

    /// Sets the minute for the builder instance. Values must be in the range `0..=59`.
    /// If the specified value is out of range, calling the `build()` method will return an error.
    ///
    /// Note: This is an optional method. The minute will default to 0 if not explicitly set.
    pub fn minute(mut self, minute: u8) -> DateTimeBuilder<Y, M, D> {
        self.minute = Some(minute);
        self
    }

    /// Sets the second for the builder instance. Values must be in range `0..=59`.
    /// If the specified value is out of range, calling the `build()` method will return an error.
    ///
    /// Note: This is an optional method. The second will default to 0 if not explicitly set.
    pub fn second(mut self, second: u8) -> DateTimeBuilder<Y, M, D> {
        self.second = Some(second);
        self
    }

    /// Sets the millisecond for the builder instance. Values must be in the range `0..=999`.
    /// If the specified value is out of range, calling the `build()` method will return an error.
    ///
    /// Note: This is an optional method. The millisecond will default to 0 if not explicitly set.
    pub fn millisecond(mut self, millisecond: u16) -> DateTimeBuilder<Y, M, D> {
        self.millisecond = Some(millisecond);
        self
    }
}

impl DateTimeBuilder<Year, Month, Day> {
    /// Convert a builder with a specified year, month, day, and optionally, an hour, minute, second
    /// and millisecond to a [`DateTime`].
    ///
    /// Note: You cannot call `build()` before setting at least the year, month and day.
    pub fn build(self) -> Result<DateTime> {
        let err = |e: time::error::ComponentRange| Error::InvalidTimestamp {
            message: e.to_string(),
        };
        let month = time::Month::try_from(self.month.0).map_err(err)?;
        let dt = Date::from_calendar_date(self.year.0, month, self.day.0)
            .map_err(err)?
            .with_hms_milli(
                self.hour.unwrap_or(0),
                self.minute.unwrap_or(0),
                self.second.unwrap_or(0),
                self.millisecond.unwrap_or(0),
            )
            .map_err(err)?;
        Ok(DateTime::from_time_private(dt.assume_utc()))
    }
}
