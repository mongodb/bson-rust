//! `Decimal128` data type representation
//!
//! Specification is https://github.com/mongodb/specifications/blob/master/source/bson-decimal128/decimal128.rst

use std::fmt;
use std::str::FromStr;

use decimal::d128;

/// Decimal128 type
#[derive(Clone, PartialEq, PartialOrd)]
pub struct Decimal128 {
    inner: d128,
}

impl Decimal128 {
    /// Construct a `Decimal128` from string.
    ///
    /// For example:
    ///
    /// * `NaN`
    /// * `Infinity` or `Inf`
    /// * `1.0`, `+37.0`, `0.73e-7`, `.5`
    pub fn from_str(s: &str) -> Decimal128 {
        Decimal128 { inner: s.parse::<d128>().expect("Invalid Decimal128 string") }
    }

    /// Construct a `Decimal128` from a `i32` number
    pub fn from_i32(d: i32) -> Decimal128 {
        Decimal128 { inner: From::from(d) }
    }

    /// Construct a `Decimal128` from a `u32` number
    pub fn from_u32(d: u32) -> Decimal128 {
        Decimal128 { inner: From::from(d) }
    }

    /// Get a `0`
    pub fn zero() -> Decimal128 {
        Decimal128 { inner: d128::zero() }
    }

    #[doc(hidden)]
    pub unsafe fn from_raw_bytes_le(mut raw: [u8; 16]) -> Decimal128 {
        if cfg!(target_endian = "big") {
            raw.reverse();
        }

        Decimal128 { inner: d128::from_raw_bytes(raw) }
    }

    #[doc(hidden)]
    pub fn to_raw_bytes_le(&self) -> [u8; 16] {
        let mut buf = self.inner.to_raw_bytes();
        if cfg!(target_endian = "big") {
            buf.reverse();
        }
        buf
    }

    /// Check if value is `NaN`
    pub fn is_nan(&self) -> bool {
        self.inner.is_nan()
    }

    /// Check if value is 0
    pub fn is_zero(&self) -> bool {
        self.inner.is_zero()
    }
}

impl fmt::Debug for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Decimal(\"{:?}\")", self.inner)
    }
}

impl fmt::Display for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl fmt::LowerHex for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <d128 as fmt::LowerHex>::fmt(&self.inner, f)
    }
}

impl fmt::LowerExp for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <d128 as fmt::LowerExp>::fmt(&self.inner, f)
    }
}

impl FromStr for Decimal128 {
    type Err = ();
    fn from_str(s: &str) -> Result<Decimal128, ()> {
        Ok(Decimal128::from_str(s))
    }
}

impl Into<d128> for Decimal128 {
    fn into(self) -> d128 {
        self.inner
    }
}

impl From<d128> for Decimal128 {
    fn from(d: d128) -> Decimal128 {
        Decimal128 { inner: d }
    }
}

impl Default for Decimal128 {
    fn default() -> Decimal128 {
        Decimal128::zero()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_decimal128_string() {
        assert!(Decimal128::from_str("0").is_zero());
        assert!(!Decimal128::from_str("12").is_nan());
        assert!(!Decimal128::from_str("-76").is_nan());
        assert!(!Decimal128::from_str("12.70").is_nan());
        assert!(!Decimal128::from_str("+0.003").is_nan());
        assert!(!Decimal128::from_str("017.").is_nan());
        assert!(!Decimal128::from_str(".5").is_nan());
        assert!(!Decimal128::from_str("4E+9").is_nan());
        assert!(!Decimal128::from_str("0.73e-7").is_nan());
        assert!(!Decimal128::from_str("Inf").is_nan());
        assert!(!Decimal128::from_str("-infinity").is_nan());
        assert!(Decimal128::from_str("NaN").is_nan());
    }
}
