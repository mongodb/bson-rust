//! Conversion format for types that implement [`facet::Facet`].
//!
//! Blanket impls of [`IntoBsonBytes<Facet>`], [`FromBsonBytes<'a, Facet>`], and
//! [`FromBsonBytesOwned<Facet>`] are provided for any type implementing [`facet::Facet`].

use super::{Format, FromBsonBytes, FromBsonBytesOwned, IntoBsonBytes};
use crate::error::Result;

/// Conversion to BSON bytes via [`facet`].
pub struct Facet;

impl Format for Facet {}

impl<'a, 'facet, T: facet::Facet<'facet> + ?Sized> IntoBsonBytes<Facet> for &'a T {
    fn append_bson_bytes(self, bytes: &mut Vec<u8>) -> Result<()> {
        crate::facet::serialize_to_buffer(self, bytes)
    }

    fn into_bson_bytes(self) -> Result<Vec<u8>> {
        crate::facet::serialize_to_vec(self)
    }
}

impl<'a, T: facet::Facet<'a>> FromBsonBytes<'a, Facet> for T {
    fn from_bson_bytes(bytes: &'a [u8]) -> Result<Self> {
        crate::facet::deserialize_from_slice(bytes)
    }
}

impl<T: for<'a> facet::Facet<'a>> FromBsonBytesOwned<Facet> for T {}
