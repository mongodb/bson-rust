//! Pluggable bson binary conversion traits.  These traits can either be implemented as a blanket
//! adapter (e.g. [`serde`], [`facet`]) or bespoke per-type (e.g. [`direct`]).
//!
//! Library authors should keep interfaces generic on the format and not enable any of the specific
//! format features:
//! ```rust
//! use bson::convert::{Format, IntoBsonBytes};
//!
//! pub fn bson_binary_size<F: Format>(v: impl IntoBsonBytes<F>) -> usize {
//!     let tmp = v.into_bson_bytes().unwrap();
//!     tmp.len()
//! }
//! ```
//!
//! Application authors should enable relevant format features:
//! ```toml
//! # Cargo.toml
//! bson = { version = "...", features = ["unstable-convert", "unstable-convert-serde", "unstable-format-direct"] }
//! ```
//! ```rust
//! # #[cfg(all(feature = "unstable-convert-serde", feature = "unstable-convert-direct"))]
//! # {
//! # use bson::{self, doc, convert::{Format, IntoBsonBytes}};
//! # use serde::Serialize;
//! # fn bson_binary_size<F: Format>(v: impl IntoBsonBytes<F>) -> usize {
//! #    let tmp = v.into_bson_bytes().unwrap();
//! #    tmp.len()
//! # }
//! #[derive(Serialize)]
//! struct MyData {};
//!
//! // Inference applies where a type only uses a single format:
//! let size = bson_binary_size(&MyData {});
//! // When a type can use multiple formats, the one to use must be specified at the call site:
//! let doc = doc! { "hello": "bytes" };
//! let size = bson_binary_size::<bson::convert::Direct>(&doc);
//! // The format trait provides convenience methods for inline types:
//! let bytes = bson::convert::Direct::into_bson_bytes(doc);
//! # }
//! ```
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
pub trait IntoBsonBytes<F: Format>: Sized {
    /// Append the BSON bytes representing this value to the end of a buffer.  The buffer will be
    /// unchanged on failure.
    fn append_bson_bytes(self, bytes: &mut Vec<u8>) -> Result<()>;

    /// Return the BSON bytes representing this value as an owned buffer.
    fn into_bson_bytes(self) -> Result<Vec<u8>> {
        let mut out = vec![];
        self.append_bson_bytes(&mut out)?;
        Ok(out)
    }
}

/// This type can be constructed from BSON bytes.
pub trait FromBsonBytes<'a, F: Format>: Sized {
    /// Construct a value from a slice of BSON bytes.
    fn from_bson_bytes(bytes: &'a [u8]) -> Result<Self>;
}

/// This type can be constructed from BSON bytes of any lifetime, i.e. it does not borrow from its
/// input.
pub trait FromBsonBytesOwned<F: Format>: for<'a> FromBsonBytes<'a, F> {
    /// Construct a value by consuming an owned buffer of BSON bytes.
    fn from_bson_bytes_owned(bytes: Vec<u8>) -> Result<Self> {
        Self::from_bson_bytes(&bytes[..])
    }
}

/// A format used to convert types to or from BSON bytes.
pub trait Format: Sized {
    /// Append the BSON bytes representing the value to the end of a buffer using this conversion
    /// format.  The buffer will be unchanged on failure.
    fn append_bson_bytes<V: IntoBsonBytes<Self>>(v: V, bytes: &mut Vec<u8>) -> Result<()> {
        v.append_bson_bytes(bytes)
    }

    /// Return the BSON bytes representing this value as an owned buffer using this conversion
    /// format.
    fn into_bson_bytes<V: IntoBsonBytes<Self>>(v: V) -> Result<Vec<u8>> {
        v.into_bson_bytes()
    }

    /// Construct a value from a slice of BSON bytes using this conversion format.
    fn from_bson_bytes<'a, V: FromBsonBytes<'a, Self>>(bytes: &'a [u8]) -> Result<V> {
        V::from_bson_bytes(bytes)
    }

    /// Construct a value by consuming an owned buffer of BSON bytes using this conversion format.
    fn from_bson_bytes_owned<V: FromBsonBytesOwned<Self>>(bytes: Vec<u8>) -> Result<V> {
        V::from_bson_bytes_owned(bytes)
    }
}
