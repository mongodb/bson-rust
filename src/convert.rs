//! Pluggable bson conversion traits.  These traits can either be implemented as a blanket adapter
//! (e.g. serde, facet) or bespoke per-type (e.g. direct).

#[cfg(feature = "unstable-convert-direct")]
pub mod direct;
#[cfg(feature = "unstable-convert-facet")]
pub mod facet;
#[cfg(feature = "unstable-convert-serde")]
pub mod serde;

use crate::error::Result;

#[cfg(feature = "unstable-convert-direct")]
pub use direct::Direct;

#[cfg(feature = "unstable-convert-facet")]
pub use facet::Facet;

#[cfg(feature = "unstable-convert-serde")]
pub use serde::Serde;

/// This type can be converted into BSON bytes.
pub trait IntoBson<F: Format>: Sized {
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
pub trait FromBson<'a, F: Format>: Sized {
    /// Construct a value from a slice of BSON bytes.
    fn from_bson(bytes: &'a [u8]) -> Result<Self>;
}

/// This type can be constructed from BSON bytes of any lifetime, i.e. it does not borrow from its
/// input.
pub trait FromBsonOwned<F: Format>: for<'a> FromBson<'a, F> {
    /// Construct a value by consuming an owned buffer of BSON bytes.
    fn from_bson_vec(bytes: Vec<u8>) -> Result<Self> {
        Self::from_bson(&bytes[..])
    }
}

/// A format used to convert types to or from BSON bytes.
pub trait Format: Sized {
    /// Append the BSON bytes representing the value to the end of a buffer using this conversion
    /// format.
    fn into_bson<V: IntoBson<Self>>(v: V, bytes: &mut Vec<u8>) -> Result<()> {
        v.into_bson(bytes)
    }

    /// Write the BSON bytes representing this value into a fresh buffer using this conversion
    /// format.
    fn into_bson_vec<V: IntoBson<Self>>(v: V) -> Result<Vec<u8>> {
        v.into_bson_vec()
    }

    /// Construct a value from a slice of BSON bytes using this conversion format.
    fn from_bson<'a, V: FromBson<'a, Self>>(bytes: &'a [u8]) -> Result<V> {
        V::from_bson(bytes)
    }

    /// Construct a value by consuming an owned buffer of BSON bytes using this conversion format.
    fn from_bson_vec<V: FromBsonOwned<Self>>(bytes: Vec<u8>) -> Result<V> {
        V::from_bson_vec(bytes)
    }
}
