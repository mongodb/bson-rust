//! Conversion format for types that implement serde traits.
//!
//! Blanket impls are provided for:
//! * [`serde::Serialize`] => [`IntoBsonBytes<Serde>`]
//! * [`serde::Deserialize<'a>`] => [`FromBsonBytes<'a, Serde>`]
//! * [`serde::de::DeserializeOwned`] => [`FromBsonBytesOwned<Serde>`]

use super::{Format, FromBsonBytes, FromBsonBytesOwned, IntoBsonBytes};
use crate::error::Result;

/// Conversion to BSON bytes via [`serde`].
pub struct Serde;

impl Format for Serde {}

impl<T: serde::Serialize + ?Sized> IntoBsonBytes<Serde> for &T {
    fn append_bson_bytes(self, bytes: &mut Vec<u8>) -> Result<()> {
        crate::serialize_to_buffer(self, bytes)
    }
}

impl<'a, T: serde::Deserialize<'a>> FromBsonBytes<'a, Serde> for T {
    fn from_bson_bytes(bytes: &'a [u8]) -> Result<Self> {
        crate::deserialize_from_slice(bytes)
    }
}

impl<T: serde::de::DeserializeOwned> FromBsonBytesOwned<Serde> for T {}
