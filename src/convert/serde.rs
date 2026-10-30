//! Pluggable conversion blanket impls for types that implement serde traits.

use super::{FromBson, FromBsonOwned, IntoBson};
use crate::error::Result;

/// Conversion to BSON bytes via serde traits.
pub struct BsonSerde;

impl<T: serde::Serialize + ?Sized> IntoBson<BsonSerde> for &T {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        crate::serialize_to_buffer(self, bytes)
    }
}

impl<'a, T: serde::Deserialize<'a>> FromBson<'a, BsonSerde> for T {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        crate::deserialize_from_slice(bytes)
    }
}

impl<T: serde::de::DeserializeOwned> FromBsonOwned<BsonSerde> for T {}
