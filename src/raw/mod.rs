//! An API for interacting with raw BSON bytes.
//!
//! This module provides two document types, [`RawDocumentBuf`] and [`RawDocument`] (akin to
//! [`std::string::String`] and [`str`]), for working with raw BSON documents. These types differ
//! from the regular [`crate::Document`] type in that their storage is BSON bytes rather than a
//! hash-map like Rust type. In certain circumstances, these types can be leveraged for increased
//! performance.
//!
//! This module also provides a [`RawBson`] type for modeling any borrowed BSON element and a
//! [`RawArray`] type for modeling a borrowed slice of a document containing a BSON array element.
//!
//! A [`RawDocumentBuf`] can be created from a `Vec<u8>` containing raw BSON data. A
//! [`RawDocument`] can be created from anything that can be borrowed as a `&[u8]`. Both types
//! can access elements via methods similar to those available on the [`crate::Document`] type.
//! Note that [`RawDocument::get`] (which [`RawDocument`] calls through to via its `Deref`
//! implementation) returns a `Result`, since the bytes contained in the document are not fully
//! validated until trying to access the contained data.
//!
//! ### [`crate::Document`] interop
//!
//! A [`RawDocument`] can be created from a [`crate::Document`]. Internally, this
//! serializes the [`crate::Document`] to a `Vec<u8>`, and then includes those bytes in the
//! [`RawDocument`].
//!
//! ### Reference type ([`RawDocument`])
//!
//! A BSON document can also be accessed with the [`RawDocument`] type, which is an
//! unsized type that represents the BSON payload as a `[u8]`. This allows accessing nested
//! documents without reallocation. [`RawDocument`] must always be accessed via a pointer type,
//! similar to `[T]` and `str`.
//!
//! The below example constructs a bson document in a stack-based array,
//! and extracts a `&str` from it, performing no heap allocation.
//!
//! ### Iteration
//!
//! [`RawDocument`] implements [`IntoIterator`](std::iter::IntoIterator), which can also be
//! accessed via [`RawDocumentBuf::iter`].

mod array;
mod bson;
mod document;
mod document_buf;
mod error;
mod iter;
#[cfg(test)]
mod test;

use std::convert::{TryFrom, TryInto};

use crate::de::MIN_BSON_STRING_SIZE;

pub use self::{
    array::{RawArray, RawArrayIter},
    bson::{RawBinary, RawBson, RawDbPointer, RawJavaScriptCodeWithScope, RawRegex},
    document::RawDocument,
    document_buf::RawDocumentBuf,
    error::{Error, ErrorKind, Result, ValueAccessError, ValueAccessErrorKind, ValueAccessResult},
    iter::Iter,
};

pub(crate) use self::bson::RawBsonVisitor;

/// Special newtype name indicating that the type being (de)serialized is a raw BSON document.
pub(crate) const RAW_DOCUMENT_NEWTYPE: &str = "$__private__bson_RawDocument";

/// Special newtype name indicating that the type being (de)serialized is a raw BSON array.
pub(crate) const RAW_ARRAY_NEWTYPE: &str = "$__private__bson_RawArray";

/// Special newtype name indicating that the type being (de)serialized is a raw BSON value.
pub(crate) const RAW_BSON_NEWTYPE: &str = "$__private__bson_RawBson";

/// Given a u8 slice, return an i32 calculated from the first four bytes in
/// little endian order.
fn f64_from_slice(val: &[u8]) -> Result<f64> {
    let arr = val
        .get(0..8)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| {
            Error::new_without_key(ErrorKind::MalformedValue {
                message: format!("expected 8 bytes to read double, instead got {}", val.len()),
            })
        })?;
    Ok(f64::from_le_bytes(arr))
}

/// Given a u8 slice, return an i32 calculated from the first four bytes in
/// little endian order.
fn i32_from_slice(val: &[u8]) -> Result<i32> {
    let arr = val
        .get(0..4)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| {
            Error::new_without_key(ErrorKind::MalformedValue {
                message: format!("expected 4 bytes to read i32, instead got {}", val.len()),
            })
        })?;
    Ok(i32::from_le_bytes(arr))
}

/// Given an u8 slice, return an i64 calculated from the first 8 bytes in
/// little endian order.
fn i64_from_slice(val: &[u8]) -> Result<i64> {
    let arr = val
        .get(0..8)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| {
            Error::new_without_key(ErrorKind::MalformedValue {
                message: format!("expected 8 bytes to read i64, instead got {}", val.len()),
            })
        })?;
    Ok(i64::from_le_bytes(arr))
}

fn read_nullterminated(buf: &[u8]) -> Result<&str> {
    let mut splits = buf.splitn(2, |x| *x == 0);
    let value = splits.next().ok_or_else(|| {
        Error::new_without_key(ErrorKind::MalformedValue {
            message: "no value".into(),
        })
    })?;
    if splits.next().is_some() {
        Ok(try_to_str(value)?)
    } else {
        Err(Error::new_without_key(ErrorKind::MalformedValue {
            message: "expected null terminator".into(),
        }))
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
        return Err(Error::new_without_key(ErrorKind::MalformedValue {
            message: format!(
                "expected buffer to contain at least {} bytes, but it only has {}",
                end,
                buf.len()
            ),
        }));
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
    std::str::from_utf8(data).map_err(|e| Error::new_without_key(ErrorKind::Utf8EncodingError(e)))
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
