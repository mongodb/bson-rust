//! Convert unsigned types to/from `Bson::Double`

use serde::{Deserialize, Deserializer, Serializer};

/// Converts primitive unsigned types to `f64`
pub trait ToF64 {
    /// Converts to `f64` value
    fn to_f64(&self) -> f64;
}

impl ToF64 for u8 {
    fn to_f64(&self) -> f64 {
        *self as f64
    }
}

impl ToF64 for u16 {
    fn to_f64(&self) -> f64 {
        *self as f64
    }
}

impl ToF64 for u32 {
    fn to_f64(&self) -> f64 {
        *self as f64
    }
}

impl ToF64 for u64 {
    fn to_f64(&self) -> f64 {
        *self as f64
    }
}

/// Serialize unsigned types to `Bson::Double`
pub fn serialize<T, S>(v: &T, s: S) -> Result<S::Ok, S::Error>
where
    T: ToF64,
    S: Serializer,
{
    s.serialize_f64(v.to_f64())
}

/// Converts from `f64` value
pub trait FromF64 {
    /// Converts from `f64` value
    fn from_f64(v: f64) -> Self;
}

impl FromF64 for u8 {
    fn from_f64(v: f64) -> u8 {
        v as u8
    }
}

impl FromF64 for u16 {
    fn from_f64(v: f64) -> u16 {
        v as u16
    }
}

impl FromF64 for u32 {
    fn from_f64(v: f64) -> u32 {
        v as u32
    }
}

impl FromF64 for u64 {
    fn from_f64(v: f64) -> u64 {
        v as u64
    }
}

/// Deserialize unsigned types to `Bson::Double`
pub fn deserialize<'de, T, D>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromF64,
{
    f64::deserialize(d).map(T::from_f64)
}
