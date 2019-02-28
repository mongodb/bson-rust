//! [BSON Decimal128](https://github.com/mongodb/specifications/blob/master/source/bson-decimal128/decimal128.rst) data type representation

use std::fmt;
use std::str::FromStr;

use decimal::d128;

/// Decimal128 type
#[derive(Clone, PartialEq, PartialOrd)]
pub struct Decimal128 {
    d128: d128,
}

impl Decimal128 {
    /// Construct a `Decimal128` from string.
    ///
    /// For example:
    ///
    /// * `NaN`
    /// * `Infinity` or `Inf`
    /// * `1.0`, `+37.0`, `0.73e-7`, `.5`
    ///
    /// ```rust
    /// use bson::decimal128::Decimal128;
    ///
    /// let dec128 = Decimal128::from_str("1.05E+3");
    /// ```
    pub fn from_str(s: &str) -> Decimal128 {
        Decimal128 { d128: s.parse::<d128>().expect("Invalid Decimal128 string"), }
    }

    /// Construct a `Decimal128` from a `i32` number.
    ///
    /// ```rust
    /// use bson::decimal128::Decimal128;
    ///
    /// let num: i32 = 23;
    /// let dec128 = Decimal128::from_i32(num);
    /// ```
    pub fn from_i32(d: i32) -> Decimal128 {
        Decimal128 { d128: From::from(d) }
    }

    /// Construct a `Decimal128` from a `u32` number.
    ///
    /// ```rust
    /// use bson::decimal128::Decimal128;
    ///
    /// let num: u32 = 78;
    /// let dec128 = Decimal128::from_u32(num);
    /// ```
    pub fn from_u32(d: u32) -> Decimal128 {
        Decimal128 { d128: From::from(d) }
    }

    /// Create a new Decimal128 as `0`.
    ///
    /// ```rust
    /// use bson::decimal128::Decimal128;
    ///
    /// let dec128 = Decimal128::zero();
    /// ```
    pub fn zero() -> Decimal128 {
        Decimal128 { d128: d128::zero() }
    }

    #[doc(hidden)]
    pub unsafe fn from_raw_bytes_le(mut raw: [u8; 16]) -> Decimal128 {
        if cfg!(target_endian = "big") {
            raw.reverse();
        }

        Decimal128 { d128: d128::from_raw_bytes(raw), }
    }

    #[doc(hidden)]
    pub fn to_raw_bytes_le(&self) -> [u8; 16] {
        let mut buf = self.d128.to_raw_bytes();
        if cfg!(target_endian = "big") {
            buf.reverse();
        }
        buf
    }

    /// Check if value is `NaN`
    ///
    /// ```rust
    /// use bson::decimal128::Decimal128;
    ///
    /// let num: u32 = 78;
    /// let dec128 = Decimal128::from_u32(num);
    /// assert!(!dec128.is_nan());
    /// ```
    pub fn is_nan(&self) -> bool {
        self.d128.is_nan()
    }

    /// Check if value is 0
    ///
    /// ```rust
    /// use bson::decimal128::Decimal128;
    ///
    /// let num: u32 = 0;
    /// let dec128 = Decimal128::from_u32(num);
    /// assert!(dec128.is_zero());
    /// ```
    pub fn is_zero(&self) -> bool {
        self.d128.is_zero()
    }
}

impl fmt::Debug for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Decimal(\"{:?}\")", self.d128)
    }
}

impl fmt::Display for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.d128)
    }
}

impl fmt::LowerHex for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <d128 as fmt::LowerHex>::fmt(&self.d128, f)
    }
}

impl fmt::LowerExp for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <d128 as fmt::LowerExp>::fmt(&self.d128, f)
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
        self.d128
    }
}

impl From<d128> for Decimal128 {
    fn from(d: d128) -> Decimal128 {
        Decimal128 { d128: d }
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
