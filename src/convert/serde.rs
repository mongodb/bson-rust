//! Pluggable conversion blanket impls for types that implement serde traits.

use super::{Format, FromBson, FromBsonOwned, IntoBson};
use crate::error::Result;

/// Conversion to BSON bytes via serde traits.
pub struct Serde;

impl Format for Serde {}

impl<T: serde::Serialize + ?Sized> IntoBson<Serde> for &T {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        crate::serialize_to_buffer(self, bytes)
    }
}

impl<'a, T: serde::Deserialize<'a>> FromBson<'a, Serde> for T {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        crate::deserialize_from_slice(bytes)
    }
}

impl<T: serde::de::DeserializeOwned> FromBsonOwned<Serde> for T {}
