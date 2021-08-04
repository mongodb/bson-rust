//! [BSON Decimal128](https://github.com/mongodb/specifications/blob/master/source/bson-decimal128/decimal128.rst) data type representation

#[cfg(feature = "decimal128")]
use decimal::d128;
use std::fmt;

/// Decimal128 type.
///
/// Currently, this type does not have any functionality and can only be serialized and
/// deserialized from existing documents that contain BSON decimal128s.
///
/// Experimental functionality can be enabled through the usage of the `"decimal128"`
/// feature flag. The flag is not recommended for use however, as it causes `Decimal128` values to
/// serialize to BSON incorrectly. See [this issue](https://github.com/mongodb/bson-rust/issues/282#issuecomment-889958970) for
/// more information.
///
/// Note that the API and behavior of the feature-gated functionality are unstable and subject to
/// change, and the feature flag will be removed completely in 2.0.0.
#[derive(Clone, PartialEq, PartialOrd)]
pub struct Decimal128 {
    #[cfg(not(feature = "decimal128"))]
    /// BSON bytes containing the decimal128. Stored for round tripping.
    pub(crate) bytes: [u8; 128 / 8],

    #[cfg(feature = "decimal128")]
    inner: decimal::d128,
}

#[cfg(feature = "decimal128")]
#[deprecated = "The feature-gated decimal128 implementation serializes to BSON incorrectly and \
                should not be used. It will be removed completely in 2.0.0"]
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
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Decimal128 {
        Decimal128 {
            inner: s.parse::<d128>().expect("Invalid Decimal128 string"),
        }
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
        Decimal128 {
            inner: From::from(d),
        }
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
        Decimal128 {
            inner: From::from(d),
        }
    }

    /// Construct a `Decimal128` from a `i32` number.
    ///
    /// ```rust
    /// use bson::decimal128::Decimal128;
    ///
    /// let num: i32 = 23;
    /// let dec128 = Decimal128::from_i32(num);
    /// let int = dec128.into_i32();
    /// assert_eq!(int, num);
    /// ```
    #[allow(clippy::wrong_self_convention)]
    #[deprecated(since = "0.15.0", note = "Replaced by `to_i32`")]
    pub fn into_i32(&self) -> i32 {
        Into::into(self.inner)
    }

    /// Construct a `Decimal128` from a `i32` number.
    ///
    /// ```rust
    /// use bson::decimal128::Decimal128;
    ///
    /// let num: i32 = 23;
    /// let dec128 = Decimal128::from_i32(num);
    /// let int = dec128.to_i32();
    /// assert_eq!(int, num);
    /// ```
    pub fn to_i32(&self) -> i32 {
        Into::into(self.inner)
    }

    /// Construct a `Decimal128` from a `i32` number.
    ///
    /// ```rust
    /// use bson::decimal128::Decimal128;
    ///
    /// let num: u32 = 23;
    /// let dec128 = Decimal128::from_u32(num);
    /// let int = dec128.into_u32();
    /// assert_eq!(int, num);
    /// ```
    #[allow(clippy::wrong_self_convention)]
    #[deprecated(since = "0.15.0", note = "Replaced by `to_u32`")]
    pub fn into_u32(&self) -> u32 {
        Into::into(self.inner)
    }

    /// Construct a `Decimal128` from a `i32` number.
    ///
    /// ```rust
    /// use bson::decimal128::Decimal128;
    ///
    /// let num: u32 = 23;
    /// let dec128 = Decimal128::from_u32(num);
    /// let int = dec128.to_u32();
    /// assert_eq!(int, num);
    /// ```
    pub fn to_u32(&self) -> u32 {
        Into::into(self.inner)
    }

    /// Create a new Decimal128 as `0`.
    ///
    /// ```rust
    /// use bson::decimal128::Decimal128;
    ///
    /// let dec128 = Decimal128::zero();
    /// ```
    pub fn zero() -> Decimal128 {
        Decimal128 {
            inner: d128::zero(),
        }
    }

    #[doc(hidden)]
    pub unsafe fn from_raw_bytes_le(mut raw: [u8; 16]) -> Decimal128 {
        if cfg!(target_endian = "big") {
            raw.reverse();
        }

        Decimal128 {
            inner: d128::from_raw_bytes(raw),
        }
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
    ///
    /// ```rust
    /// use bson::decimal128::Decimal128;
    ///
    /// let num: u32 = 78;
    /// let dec128 = Decimal128::from_u32(num);
    /// assert!(!dec128.is_nan());
    /// ```
    pub fn is_nan(&self) -> bool {
        self.inner.is_nan()
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
        self.inner.is_zero()
    }
}

impl fmt::Debug for Decimal128 {
    #[cfg(feature = "decimal128")]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Decimal128(\"{:?}\")", self.inner)
    }

    #[cfg(not(feature = "decimal128"))]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Decimal128(...)")
    }
}

impl fmt::Display for Decimal128 {
    #[cfg(feature = "decimal128")]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.inner)
    }

    #[cfg(not(feature = "decimal128"))]
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(feature = "decimal128")]
impl fmt::LowerHex for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <d128 as fmt::LowerHex>::fmt(&self.inner, f)
    }
}

#[cfg(feature = "decimal128")]
impl fmt::LowerExp for Decimal128 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <d128 as fmt::LowerExp>::fmt(&self.inner, f)
    }
}

#[cfg(feature = "decimal128")]
#[allow(deprecated)]
impl std::str::FromStr for Decimal128 {
    type Err = ();
    fn from_str(s: &str) -> Result<Decimal128, ()> {
        Ok(Decimal128::from_str(s))
    }
}

#[cfg(feature = "decimal128")]
impl From<Decimal128> for d128 {
    fn from(decimal: Decimal128) -> d128 {
        decimal.inner
    }
}

#[cfg(feature = "decimal128")]
impl From<d128> for Decimal128 {
    fn from(d: d128) -> Decimal128 {
        Decimal128 { inner: d }
    }
}

#[cfg(feature = "decimal128")]
#[allow(deprecated)]
impl Default for Decimal128 {
    fn default() -> Decimal128 {
        Decimal128::zero()
    }
}

#[cfg(test)]
#[cfg(feature = "decimal128")]
#[allow(deprecated)]
mod test {
    use super::*;

    #[test]
    fn decimal128_string() {
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

    #[test]
    fn decimal128_i32() {
        let num: i32 = 89;
        let dec128 = Decimal128::from_i32(num);

        assert!(!dec128.is_nan());
        assert!(!dec128.is_zero());
        assert_eq!(dec128.to_i32(), num);
    }

    #[test]
    fn decimal128_u32() {
        let num: u32 = 89;
        let dec128 = Decimal128::from_u32(num);

        assert!(!dec128.is_nan());
        assert!(!dec128.is_zero());
        assert_eq!(dec128.to_u32(), num);
    }

    #[test]
    fn decimal128_0() {
        let dec128 = Decimal128::zero();
        assert!(dec128.is_zero());
    }

    #[test]
    fn decimal128_is_zero() {
        let dec128 = Decimal128::from_i32(234);
        assert!(!dec128.is_zero());

        let dec128_0 = Decimal128::from_i32(0);
        assert!(dec128_0.is_zero());
    }

    #[test]
    fn decimal128_is_nan() {
        let dec128 = Decimal128::from_str("NaN");
        assert!(dec128.is_nan());

        let dec128 = Decimal128::from_i32(234);
        assert!(!dec128.is_nan());
    }
}
