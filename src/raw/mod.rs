//! An API for interacting with raw BSON bytes.
//!
//! This module provides two document types, [`RawDocument`] and [`RawDoc`] (akin to [`std::String`]
//! and [`std::str`], for working with raw BSON documents. These types differ from the regular
//! [`Document`] type in that their storage is BSON bytes rather than a hash-map like Rust type. In
//! certain circumstances, these types can be leveraged for increased performance.
//!
//! This module also provides a [`RawBson`] type for modeling any borrowed BSON element and a
//! [`RawArr`] type for modeling a borrowed slice of a document containing a BSON array element.
//!
//! A [`RawDocument`] can be created from a `Vec<u8>` containing raw BSON data, and elements
//! accessed via methods similar to those available on the [`Document`] type. Note that
//! [`RawDocument::get`] returns a [`raw::Result<Option<RawBson>>`], since the bytes contained in
//! the document are not fully validated until trying to access the contained data.
//!
//! ```rust
//! use bson::raw::{
//!     RawBson,
//!     RawDocument,
//! };
//!
//! // See http://bsonspec.org/spec.html for details on the binary encoding of BSON.
//! let doc = RawDocument::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
//! let elem = doc.get("hi")?.unwrap();
//!
//! assert_eq!(
//!   elem.as_str()?,
//!   "y'all",
//! );
//! # Ok::<(), bson::raw::Error>(())
//! ```
//!
//! ### [`Document`] interop
//!
//! A [`RawDocument`] can be created from a [`Document`]. Internally, this
//! serializes the [`Document`] to a `Vec<u8>`, and then includes those bytes in the
//! [`RawDocument`].
//!
//! ```rust
//! use bson::{
//!     raw::RawDocument,
//!     doc,
//! };
//!
//! let document = doc! {
//!    "goodbye": {
//!        "cruel": "world"
//!    }
//! };
//!
//! let raw = RawDocument::from_document(&document)?;
//! let value: Option<&str> = raw
//!     .get_document("goodbye")?
//!     .map(|doc| doc.get_str("cruel"))
//!     .transpose()?
//!     .flatten();
//!
//! assert_eq!(
//!     value,
//!     Some("world"),
//! );
//! # Ok::<(), bson::raw::Error>(())
//! ```
//!
//! ### Reference types ([`RawDoc`])
//!
//! A BSON document can also be accessed with the [`RawDoc`] type, which is an
//! unsized type that represents the BSON payload as a `[u8]`. This allows accessing nested
//! documents without reallocation. [`RawDoc`] must always be accessed via a pointer type,
//! similarly to `[T]` and `str`.
//!
//! The below example constructs a bson document in a stack-based array,
//! and extracts a &str from it, performing no heap allocation.
//! ```rust
//! use bson::raw::RawDoc;
//!
//! let bytes = b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00";
//! assert_eq!(RawDoc::new(bytes)?.get_str("hi")?, Some("y'all"));
//! # Ok::<(), bson::raw::Error>(())
//! ```
//!
//! ### Iteration
//!
//! [`RawDoc`] implements [`IntoIterator`](std::iter::IntoIterator), which can also be
//! accessed via [`RawDocument::iter`].

//! ```rust
//! use bson::{
//!    raw::{
//!        RawBson,
//!        RawDocument,
//!    },
//!    doc,
//! };
//!
//! let original_doc = doc! {
//!     "crate": "bson",
//!     "year": "2021",
//! };
//!
//! let doc = RawDocument::from_document(&original_doc)?;
//! let mut doc_iter = doc.iter();
//!
//! let (key, value): (&str, RawBson) = doc_iter.next().unwrap()?;
//! assert_eq!(key, "crate");
//! assert_eq!(value.as_str()?, "bson");
//!
//! let (key, value): (&str, RawBson) = doc_iter.next().unwrap()?;
//! assert_eq!(key, "year");
//! assert_eq!(value.as_str()?, "2021");
//! # Ok::<(), bson::raw::Error>(())
//! ```

mod array;
mod doc;
mod elem;
mod error;
#[cfg(test)]
mod test;

use std::convert::{TryFrom, TryInto};

use crate::de::MIN_BSON_STRING_SIZE;

pub use self::{
    array::{RawArr, RawArrIter},
    doc::{Iter, RawDoc, RawDocument},
    elem::{RawBinary, RawBson, RawJavaScriptCodeWithScope, RawRegex},
    error::{Error, ErrorKind, Result},
};

/// Given a u8 slice, return an i32 calculated from the first four bytes in
/// little endian order.
fn f64_from_slice(val: &[u8]) -> Result<f64> {
    let arr = val
        .get(0..8)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| Error {
            key: None,
            kind: ErrorKind::MalformedValue {
                message: format!("expected 8 bytes to read double, instead got {}", val.len()),
            },
        })?;
    Ok(f64::from_le_bytes(arr))
}

/// Given a u8 slice, return an i32 calculated from the first four bytes in
/// little endian order.
fn i32_from_slice(val: &[u8]) -> Result<i32> {
    let arr = val
        .get(0..4)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| Error {
            key: None,
            kind: ErrorKind::MalformedValue {
                message: format!("expected 4 bytes to read i32, instead got {}", val.len()),
            },
        })?;
    Ok(i32::from_le_bytes(arr))
}

/// Given an u8 slice, return an i64 calculated from the first 8 bytes in
/// little endian order.
fn i64_from_slice(val: &[u8]) -> Result<i64> {
    let arr = val
        .get(0..8)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| Error {
            key: None,
            kind: ErrorKind::MalformedValue {
                message: format!("expected 8 bytes to read i64, instead got {}", val.len()),
            },
        })?;
    Ok(i64::from_le_bytes(arr))
}

fn read_nullterminated(buf: &[u8]) -> Result<&str> {
    let mut splits = buf.splitn(2, |x| *x == 0);
    let value = splits.next().ok_or_else(|| Error {
        key: None,
        kind: ErrorKind::MalformedValue {
            message: "no value".into(),
        },
    })?;
    if splits.next().is_some() {
        Ok(try_to_str(value)?)
    } else {
        Err(Error {
            key: None,
            kind: ErrorKind::MalformedValue {
                message: "expected null terminator".into(),
            },
        })
    }
}

fn read_lenencoded(buf: &[u8]) -> Result<&str> {
    let length = i32_from_slice(&buf[..4])?;
    let end = checked_add(usize_try_from_i32(length)?, 4)?;

    if end < MIN_BSON_STRING_SIZE as usize {
        return Err(Error::new_without_key(ErrorKind::MalformedValue {
            message: format!(
                "BSON length encoded string needs to be at least {} bytes, instead got {}",
                MIN_BSON_STRING_SIZE, end
            ),
        }));
    }

    if buf.len() < end {
        return Err(Error {
            key: None,
            kind: ErrorKind::MalformedValue {
                message: format!(
                    "expected buffer to contain at least {} bytes, but it only has {}",
                    end,
                    buf.len()
                ),
            },
        });
    }

    if buf[end - 1] != 0 {
        return Err(Error::new_without_key(ErrorKind::MalformedValue {
            message: "expected string to be null-terminated".to_string(),
        }));
    }

    // exclude null byte
    try_to_str(&buf[4..(end - 1)])
}

fn try_to_str(data: &[u8]) -> Result<&str> {
    match std::str::from_utf8(data) {
        Ok(s) => Ok(s),
        Err(e) => Err(Error {
            key: None,
            kind: ErrorKind::Utf8EncodingError(e),
        }),
    }
}

fn usize_try_from_i32(i: i32) -> Result<usize> {
    usize::try_from(i).map_err(|e| {
        Error::new_without_key(ErrorKind::MalformedValue {
            message: e.to_string(),
        })
    })
}

fn checked_add(lhs: usize, rhs: usize) -> Result<usize> {
    lhs.checked_add(rhs).ok_or_else(|| {
        Error::new_without_key(ErrorKind::MalformedValue {
            message: "attempted to add with overflow".to_string(),
        })
    })
}
