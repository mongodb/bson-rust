//! Pluggable bson conversion traits.  These traits can either be implemented as a blanket adapter
//! (e.g. serde, facet) or bespoke per-type (e.g. raw).

#[cfg(feature = "convert-facet")]
pub mod facet;
#[cfg(feature = "convert-raw")]
pub mod raw;
#[cfg(feature = "convert-serde")]
pub mod serde;

use crate::error::Result;

/// This type can be converted into BSON bytes.
pub trait IntoBson<T>: Sized {
    /// Append the BSON bytes representing this value to the end of a buffer.
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()>;

    /// Write the BSON bytes representing this value into a fresh buffer.
    fn into_bson_vec(self) -> Result<Vec<u8>> {
        let mut out = vec![];
        self.into_bson(&mut out)?;
        Ok(out)
    }
}

/// This type can be constructed from BSON bytes.
pub trait FromBson<'a, T>: Sized {
    /// Construct a value from a slice of BSON bytes.
    fn from_bson(bytes: &'a [u8]) -> Result<Self>;
}

/// This type can be constructed from BSON bytes of any lifetime, i.e. it does not borrow from its
/// input.
pub trait FromBsonOwned<T>: for<'a> FromBson<'a, T> {
    /// Construct a value by consuming an owned buffer of BSON bytes.
    fn from_bson_vec(bytes: Vec<u8>) -> Result<Self> {
        Self::from_bson(&bytes[..])
    }
}
