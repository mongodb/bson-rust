//! Pluggable conversion blanket impls for types that implement [`facet::Facet`].
use super::{Format, FromBson, FromBsonOwned, IntoBson};
use crate::error::Result;

/// Conversion to BSON bytes via serde traits.
pub struct Facet;

impl Format for Facet {}

impl<'a, T: facet::Facet<'a> + ?Sized> IntoBson<Facet> for &'a T {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        crate::facet::serialize_to_buffer(self, bytes)
    }

    fn into_bson_vec(self) -> Result<Vec<u8>> {
        crate::facet::serialize_to_vec(self)
    }
}

impl<'a, T: facet::Facet<'a>> FromBson<'a, Facet> for T {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        crate::facet::deserialize_from_slice(bytes)
    }
}

impl<T: for<'a> facet::Facet<'a>> FromBsonOwned<Facet> for T {}
