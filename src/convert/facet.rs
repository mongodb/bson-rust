//! Pluggable conversion blanket impls for types that implement [`facet::Facet`].
use super::{FromBson, FromBsonOwned, IntoBson};
use crate::error::Result;

/// Conversion to BSON bytes via serde traits.
pub struct BsonFacet;

impl<'a, T: facet::Facet<'a> + ?Sized> IntoBson<BsonFacet> for &'a T {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.append(&mut self.into_bson_vec()?);
        Ok(())
    }

    fn into_bson_vec(self) -> Result<Vec<u8>> {
        crate::facet::serialize_to_vec(self)
    }
}
