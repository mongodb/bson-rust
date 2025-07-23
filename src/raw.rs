//! An API for interacting with raw BSON bytes.
//!
//! This module provides two document types, [`RawDocumentBuf`] and [`&RawDocument`](RawDocument)
//! (an owned buffer and a reference respectively, akin to [`String`] and [`&str`](str)), for
//! working with raw BSON documents. These types differ from the regular
//! [`Document`](crate::Document) type in that their storage is BSON bytes rather than a hash-map
//! like Rust type. In certain circumstances, these types can be leveraged for increased
//! performance.
//!
//! This module also provides a [`RawBson`] type for modeling any borrowed BSON element and a
//! [`RawArray`] type for modeling a borrowed slice of a document containing a BSON array element.
//!
//! A [`RawDocumentBuf`] can be created from a `Vec<u8>` containing raw BSON data. A
//! [`&RawDocument`](RawDocument) can be created from anything that can be borrowed as a `&[u8]`.
//! Both types can access elements via methods similar to those available on the
//! [`Document`](crate::Document) type. Note that [`RawDocument::get`] (which [`RawDocumentBuf`]
//! calls through to via its [`Deref`](std::ops::Deref) implementation) returns a [`Result`], since
//! the bytes contained in the document are not fully validated until trying to access the contained
//! data.
//!
//! ```rust
//! use bson::raw::{
//!     RawBson,
//!     RawDocumentBuf,
//! };
//!
//! // See http://bsonspec.org/spec.html for details on the binary encoding of BSON.
//! let doc = RawDocumentBuf::decode_from_bytes(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
//! let elem = doc.get("hi")?.unwrap();
//!
//! assert_eq!(
//!   elem.as_str(),
//!   Some("y'all"),
//! );
//! # Ok::<(), bson::error::Error>(())
//! ```
//!
//! ### [`Document`](crate::Document) interop
//!
//! A [`RawDocumentBuf`] can be created from a [`Document`](crate::Document) via its [`TryFrom`]
//! impl. This encodes the `Document` as a byte buffer, and then returns those bytes as a
//! `RawDocumentBuf`; this will fail if the `Document` contains values not allowed in encoded BSON
//! (such as embedded nul bytes in string keys).
//!
//! ```rust
//! use bson::{
//!     raw::RawDocumentBuf,
//!     doc,
//! };
//!
//! let document = doc! {
//!    "goodbye": {
//!        "cruel": "world"
//!    }
//! };
//!
//! let raw = RawDocumentBuf::try_from(&document)?;
//! let value = raw
//!     .get_document("goodbye")?
//!     .get_str("cruel")?;
//!
//! assert_eq!(
//!     value,
//!     "world",
//! );
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! Conversion in the other direction, from [`&RawDocument`](RawDocument) to
//! [`Document`](crate::Document), can also be done via [`TryFrom`].  This will fail if the byte
//! buffer in the `&RawDocument` contains invalid BSON.
//!
//! [`RawBson`] and [`RawArrayBuf`] can similary be constructed from their equivalent base crate
//! types, and their corresponding reference types can be converted to the base crate types.
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
//! ```rust
//! use bson::raw::RawDocument;
//!
//! let bytes = b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00";
//! assert_eq!(RawDocument::decode_from_bytes(bytes)?.get_str("hi")?, "y'all");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! ### Iteration
//!
//! [`RawDocument`] implements [`IntoIterator`], which can also be
//! accessed via [`RawDocumentBuf::iter`].

//! ```rust
//! use bson::{
//!    raw::{
//!        cstr,
//!        CStr,
//!        RawBsonRef,
//!        RawDocumentBuf,
//!    },
//!    doc,
//! };
//!
//! let original_doc = doc! {
//!     "crate": "bson",
//!     "year": "2021",
//! };
//!
//! let doc = RawDocumentBuf::try_from(&original_doc)?;
//! let mut doc_iter = doc.iter();
//!
//! let (key, value): (&CStr, RawBsonRef) = doc_iter.next().unwrap()?;
//! assert_eq!(key, cstr!("crate"));
//! assert_eq!(value.as_str(), Some("bson"));
//!
//! let (key, value): (&CStr, RawBsonRef) = doc_iter.next().unwrap()?;
//! assert_eq!(key, cstr!("year"));
//! assert_eq!(value.as_str(), Some("2021"));
//! # Ok::<(), bson::error::Error>(())
//! ```

mod array;
mod array_buf;
mod bson;
mod bson_ref;
mod cstr;
mod document;
mod document_buf;
mod iter;
#[cfg(feature = "serde")]
pub(crate) mod serde;
#[cfg(test)]
mod test;

use std::{
    convert::{TryFrom, TryInto},
    io::Read,
};

use crate::error::{Error, ErrorKind, Result};

pub use self::{
    array::{RawArray, RawArrayIter},
    array_buf::RawArrayBuf,
    bson::{RawBson, RawJavaScriptCodeWithScope},
    bson_ref::{
        RawBinaryRef,
        RawBsonRef,
        RawDbPointerRef,
        RawJavaScriptCodeWithScopeRef,
        RawRegexRef,
    },
    cstr::{assert_valid_cstr, cstr, validate_cstr, CStr, CString, IsValidCStr},
    document::RawDocument,
    document_buf::{BindRawBsonRef, BindValue, RawDocumentBuf},
    iter::{RawElement, RawIter},
};

pub(crate) const MIN_BSON_STRING_SIZE: i32 = 4 + 1; // 4 bytes for length, one byte for null terminator
pub(crate) const MIN_BSON_DOCUMENT_SIZE: i32 = 4 + 1; // 4 bytes for length, one byte for null terminator
pub(crate) const MIN_CODE_WITH_SCOPE_SIZE: i32 = 4 + MIN_BSON_STRING_SIZE + MIN_BSON_DOCUMENT_SIZE;

#[cfg(feature = "serde")]
pub(crate) use self::iter::{Utf8LossyBson, Utf8LossyJavaScriptCodeWithScope};

/// Special newtype name indicating that the type being (de)serialized is a raw BSON document.
#[cfg(feature = "serde")]
pub(crate) const RAW_DOCUMENT_NEWTYPE: &str = "$__private__bson_RawDocument";

/// Special newtype name indicating that the type being (de)serialized is a raw BSON array.
#[cfg(feature = "serde")]
pub(crate) const RAW_ARRAY_NEWTYPE: &str = "$__private__bson_RawArray";

/// Special newtype name indicating that the type being (de)serialized is a raw BSON value.
#[cfg(feature = "serde")]
pub(crate) const RAW_BSON_NEWTYPE: &str = "$__private__bson_RawBson";

/// Given a u8 slice, return an i32 calculated from the first four bytes in
/// little endian order.
fn f64_from_slice(val: &[u8]) -> Result<f64> {
    let arr = val
        .get(0..8)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| {
            Error::malformed_bytes(format!(
                "expected 8 bytes to read double, instead got {}",
                val.len()
            ))
        })?;
    Ok(f64::from_le_bytes(arr))
}

/// Given a u8 slice, return an i32 calculated from the first four bytes in
/// little endian order.
fn i32_from_slice(val: &[u8]) -> Result<i32> {
    let arr: [u8; 4] = val
        .get(0..4)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| {
            Error::malformed_bytes(format!(
                "expected 4 bytes to read i32, instead got {}",
                val.len()
            ))
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
            Error::malformed_bytes(format!(
                "expected 8 bytes to read i64, instead got {}",
                val.len()
            ))
        })?;
    Ok(i64::from_le_bytes(arr))
}

fn u8_from_slice(val: &[u8]) -> Result<u8> {
    let arr = val
        .get(0..1)
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| {
            Error::malformed_bytes(format!(
                "expected 1 byte to read u8, instead got {}",
                val.len()
            ))
        })?;
    Ok(u8::from_le_bytes(arr))
}

pub(crate) fn bool_from_slice(val: &[u8]) -> Result<bool> {
    let val = u8_from_slice(val)?;
    if val > 1 {
        return Err(Error::malformed_bytes(format!(
            "boolean must be stored as 0 or 1, got {}",
            val
        )));
    }

    Ok(val != 0)
}

fn read_len(buf: &[u8]) -> Result<usize> {
    if buf.len() < 4 {
        return Err(Error::malformed_bytes(format!(
            "expected buffer with string to contain at least 4 bytes, but it only has {}",
            buf.len()
        )));
    }

    let length = i32_from_slice(&buf[..4])?;
    let end = checked_add(usize_try_from_i32(length)?, 4)?;

    if end < MIN_BSON_STRING_SIZE as usize {
        return Err(Error::malformed_bytes(format!(
            "BSON length encoded string needs to be at least {} bytes, instead got {}",
            MIN_BSON_STRING_SIZE, end
        )));
    }

    if buf.len() < end {
        return Err(Error::malformed_bytes(format!(
            "expected buffer to contain at least {} bytes, but it only has {}",
            end,
            buf.len()
        )));
    }

    if buf[end - 1] != 0 {
        return Err(Error::malformed_bytes(
            "expected string to be null-terminated",
        ));
    }

    Ok(length as usize + 4)
}

fn read_lenencode_bytes(buf: &[u8]) -> Result<&[u8]> {
    let end = read_len(buf)?;

    // exclude length-prefix and null byte suffix
    Ok(&buf[4..(end - 1)])
}

fn read_lenencode(buf: &[u8]) -> Result<&str> {
    try_to_str(read_lenencode_bytes(buf)?)
}

fn try_to_str(data: &[u8]) -> Result<&str> {
    simdutf8::basic::from_utf8(data).map_err(|_| ErrorKind::Utf8Encoding {}.into())
}

fn usize_try_from_i32(i: i32) -> Result<usize> {
    usize::try_from(i).map_err(Error::malformed_bytes)
}

fn checked_add(lhs: usize, rhs: usize) -> Result<usize> {
    lhs.checked_add(rhs)
        .ok_or_else(|| Error::malformed_bytes("attempted to add with overflow"))
}

pub(crate) fn reader_to_vec<R: Read>(mut reader: R) -> Result<Vec<u8>> {
    let mut buf = [0; 4];
    reader.read_exact(&mut buf)?;
    let length = i32::from_le_bytes(buf);

    if length < MIN_BSON_DOCUMENT_SIZE {
        return Err(Error::malformed_bytes("document size too small"));
    }

    let mut bytes = Vec::with_capacity(length as usize);
    bytes.extend(buf);

    reader.take(length as u64 - 4).read_to_end(&mut bytes)?;
    Ok(bytes)
}

pub(crate) fn write_string(buf: &mut Vec<u8>, s: &str) {
    buf.extend(&(s.len() as i32 + 1).to_le_bytes());
    buf.extend(s.as_bytes());
    buf.push(0);
}
