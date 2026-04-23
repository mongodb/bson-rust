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
//! let doc = RawDocumentBuf::from_bytes(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
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
//! assert_eq!(RawDocument::from_bytes(bytes)?.get_str("hi")?, "y'all");
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
pub(crate) mod doc_writer;
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

use crate::{
    DateTime,
    Decimal128,
    Timestamp,
    error::{Error, ErrorKind, Result},
    oid::ObjectId,
    spec::ElementType,
};

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
    cstr::{CStr, CString, IsValidCStr, assert_valid_cstr, cstr, validate_cstr},
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
pub(crate) fn i32_from_slice(val: &[u8]) -> Result<i32> {
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

pub(crate) fn read_lenencode(buf: &[u8]) -> Result<&str> {
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

pub(crate) fn cstring_bytes(buf: &[u8]) -> Result<&[u8]> {
    let mut splits = buf.splitn(2, |x| *x == 0);
    let value = splits
        .next()
        .ok_or_else(|| Error::malformed_bytes("no value"))?;
    if splits.next().is_some() {
        Ok(value)
    } else {
        Err(Error::malformed_bytes("expected null terminator"))
    }
}

pub(crate) fn read_cstring(buf: &[u8]) -> Result<&CStr> {
    let bytes = cstring_bytes(buf)?;
    let s = try_to_str(bytes)?;
    s.try_into()
}

#[derive(Clone)]
pub(crate) struct RawValue<'a> {
    kind: ElementType,
    bytes: &'a [u8],
    source_offset: usize,
}

impl<'a> RawValue<'a> {
    fn new(kind: ElementType, bytes: &'a [u8]) -> Self {
        Self {
            kind,
            bytes,
            source_offset: 0,
        }
    }

    fn parse(&self) -> Result<RawBsonRef<'a>> {
        Ok(match self.kind {
            ElementType::Null => RawBsonRef::Null,
            ElementType::Undefined => RawBsonRef::Undefined,
            ElementType::MinKey => RawBsonRef::MinKey,
            ElementType::MaxKey => RawBsonRef::MaxKey,
            ElementType::ObjectId => RawBsonRef::ObjectId(ObjectId::parse(self.bytes)?),
            ElementType::Int32 => RawBsonRef::Int32(i32_from_slice(self.bytes)?),
            ElementType::Int64 => RawBsonRef::Int64(i64_from_slice(self.bytes)?),
            ElementType::Double => RawBsonRef::Double(f64_from_slice(self.bytes)?),
            ElementType::String => RawBsonRef::String(self.read_str()?),
            ElementType::EmbeddedDocument => {
                RawBsonRef::Document(RawDocument::from_bytes(self.bytes)?)
            }
            ElementType::Array => {
                RawBsonRef::Array(RawArray::from_doc(RawDocument::from_bytes(self.bytes)?))
            }
            ElementType::Boolean => RawBsonRef::Boolean(
                bool_from_slice(self.bytes).map_err(|e| Error::malformed_bytes(e))?,
            ),
            ElementType::DateTime => RawBsonRef::DateTime(DateTime::parse(self.bytes)?),
            ElementType::Decimal128 => RawBsonRef::Decimal128(Decimal128::parse(self.bytes)?),
            ElementType::JavaScriptCode => RawBsonRef::JavaScriptCode(self.read_str()?),
            ElementType::Symbol => RawBsonRef::Symbol(self.read_str()?),
            ElementType::DbPointer => RawBsonRef::DbPointer(RawDbPointerRef::parse(self.bytes)?),
            ElementType::RegularExpression => {
                RawBsonRef::RegularExpression(RawRegexRef::parse(self.bytes)?)
            }
            ElementType::Timestamp => RawBsonRef::Timestamp(Timestamp::parse(self.bytes)?),
            ElementType::Binary => RawBsonRef::Binary(RawBinaryRef::parse(self.bytes)?),
            ElementType::JavaScriptCodeWithScope => RawBsonRef::JavaScriptCodeWithScope(
                RawJavaScriptCodeWithScopeRef::parse(self.bytes)?,
            ),
        })
    }

    pub(crate) fn parse_utf8_lossy(&self) -> Result<Option<Utf8LossyBson<'a>>> {
        Ok(Some(match self.kind {
            ElementType::String => Utf8LossyBson::String(self.read_utf8_lossy()),
            ElementType::JavaScriptCode => Utf8LossyBson::JavaScriptCode(self.read_utf8_lossy()),
            ElementType::JavaScriptCodeWithScope => {
                if self.bytes.len() < MIN_CODE_WITH_SCOPE_SIZE as usize {
                    return Err(Error::malformed_bytes("code with scope length too small"));
                }

                let slice = self.bytes;
                let code = String::from_utf8_lossy(read_lenencode_bytes(&slice[4..])?).into_owned();
                let scope_start = 4 + 4 + code.len() + 1;
                if scope_start >= slice.len() {
                    return Err(Error::malformed_bytes("code with scope length overrun"));
                }
                let scope = RawDocument::from_bytes(&slice[scope_start..])?;

                Utf8LossyBson::JavaScriptCodeWithScope(Utf8LossyJavaScriptCodeWithScope {
                    code,
                    scope,
                })
            }
            ElementType::Symbol => Utf8LossyBson::Symbol(self.read_utf8_lossy()),
            ElementType::DbPointer => Utf8LossyBson::DbPointer(crate::DbPointer {
                namespace: String::from_utf8_lossy(read_lenencode_bytes(self.bytes)?).into_owned(),
                id: self.get_oid_at(self.bytes.len() - 12)?,
            }),
            ElementType::RegularExpression => {
                let pattern = String::from_utf8_lossy(cstring_bytes(self.bytes)?).into_owned();
                let pattern_len = pattern.len();
                Utf8LossyBson::RegularExpression(crate::Regex {
                    pattern: pattern.try_into()?,
                    options: String::from_utf8_lossy(cstring_bytes(
                        &self.bytes[pattern_len + 1..],
                    )?)
                    .into_owned()
                    .try_into()?,
                })
            }
            _ => return Ok(None),
        }))
    }

    pub(crate) fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub(crate) fn source_offset(&self) -> usize {
        self.source_offset
    }

    #[cfg(feature = "facet-unstable")]
    pub(crate) fn span(&self) -> facet_reflect::Span {
        facet_reflect::Span::new(self.source_offset, self.bytes.len())
    }

    fn slice_bounds(&self, start_at: usize, size: usize) -> &'a [u8] {
        &self.bytes[start_at..(start_at + size)]
    }

    fn read_str(&self) -> Result<&'a str> {
        try_to_str(self.str_bytes())
    }

    fn str_bytes(&self) -> &'a [u8] {
        self.slice_bounds(4, self.bytes.len() - 4 - 1)
    }

    fn read_utf8_lossy(&self) -> String {
        String::from_utf8_lossy(self.str_bytes()).into_owned()
    }

    fn get_oid_at(&self, start_at: usize) -> Result<ObjectId> {
        Ok(ObjectId::from_bytes(
            self.bytes[start_at..(start_at + 12)]
                .try_into()
                .map_err(|e| Error::malformed_bytes(e))?,
        ))
    }
}
