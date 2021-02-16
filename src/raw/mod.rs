//! A RawDocument can be created from a `Vec<u8>` containing raw BSON data, and elements
//! accessed via methods similar to those available on the Document type. Note that rawbson returns
//! a raw::Result<Option<T>>, since the bytes contained in the document are not fully validated
//! until trying to access the contained data.
//!
//! ```rust
//! use bson::raw::{
//!     RawBson,
//!     RawDocument,
//! };
//!
//! // See http://bsonspec.org/spec.html for details on the binary encoding of BSON.
//! let doc = RawDocument::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
//! let elem: Option<RawBson> = doc.get("hi")?;
//!
//! assert_eq!(
//!   elem?.as_str()?,
//!   "y'all",
//! );
//! # Ok::<(), bson::raw::RawError>(())
//! ```
//!
//! ### bson-rust interop
//!
//! A [`RawDocument`] can be created from a [`bson::document::Document`]. Internally, this
//! serializes the `Document` to a `Vec<u8>`, and then includes those bytes in the [`RawDocument`].
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

//! let raw = RawDocument::from_document(&document);
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
//! # Ok::<(), bson::raw::RawError>(())
//! ```
//! 
//! ### Reference types
//!
//! A BSON document can also be accessed with the [`RawDocumentRef`] reference type, which is an
//! unsized type that represents the BSON payload as a `[u8]`. This allows accessing nested
//! documents without reallocation. [RawDocumentRef] must always be accessed via a pointer type,
//! similarly to `[T]` and `str`.
//!
//! The below example constructs a bson document in a stack-based array,
//! and extracts a &str from it, performing no heap allocation.

//! ```rust
//! use bson::raw::Doc;
//!
//! let bytes = b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00";
//! assert_eq!(RawDocumentRef::new(bytes)?.get_str("hi")?, Some("y'all"));
//! # Ok::<(), bson::raw::RawError>(())
//! ```
//!
//! ### Iteration
//!
//! [`RawDocumentRef`] implements [`IntoIterator`](std::iter::IntoIterator), which can also be
//! accessed via [`RawDocument::iter`].

//! ```rust
//! use bson::doc;
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
//! let doc = RawDocument::from_document(&original_doc);
//! let mut doc_iter = doc.iter();
//!
//! let (key, value): (&str, Element) = doc_iter.next().unwrap()?;
//! assert_eq!(key, "crate");
//! assert_eq!(value.as_str()?, "rawbson");
//!
//! let (key, value): (&str, Element) = doc_iter.next().unwrap()?;
//! assert_eq!(key, "year");
//! assert_eq!(value.as_str()?, "2021");
//! # Ok::<(), bson::raw::RawError>(())
//! ```

mod array;
mod doc;
mod elem;
mod error;
#[cfg(test)]
mod props;
#[cfg(test)]
mod test;

pub use self::{
    array::{RawArray, RawArrayIter},
    doc::{RawDocument, RawDocumentIter, RawDocumentRef},
    elem::{RawBinary, RawBson, RawJavaScriptCodeWithScope, RawRegex, RawTimestamp},
    error::{Error, Result},
};
/// Given a 4 byte u8 slice, return an i32 calculated from the bytes in
/// little endian order
///
/// # Panics
///
/// This function panics if given a slice that is not four bytes long.
fn i32_from_slice(val: &[u8]) -> i32 {
    i32::from_le_bytes(val.try_into().expect("i32 is four bytes"))
}

/// Given an 8 byte u8 slice, return an i64 calculated from the bytes in
/// little endian order
///
/// # Panics
///
/// This function panics if given a slice that is not eight bytes long.
fn i64_from_slice(val: &[u8]) -> i64 {
    i64::from_le_bytes(val.try_into().expect("i64 is eight bytes"))
}

/// Given a 4 byte u8 slice, return a u32 calculated from the bytes in
/// little endian order
///
/// # Panics
///
/// This function panics if given a slice that is not four bytes long.
fn u32_from_slice(val: &[u8]) -> u32 {
    u32::from_le_bytes(val.try_into().expect("u32 is four bytes"))
}

#[cfg(feature = "decimal128")]
fn d128_from_slice(val: &[u8]) -> Decimal128 {
    // TODO: Handle Big Endian platforms
    let d =
        unsafe { decimal::d128::from_raw_bytes(val.try_into().expect("d128 is sixteen bytes")) };
    Decimal128::from(d)
}

fn read_nullterminated(buf: &[u8]) -> Result<&str> {
    let mut splits = buf.splitn(2, |x| *x == 0);
    let value = splits.next().ok_or_else(|| Error::MalformedValue {
        message: "no value".into(),
    })?;
    if splits.next().is_some() {
        Ok(try_to_str(value)?)
    } else {
        Err(Error::MalformedValue {
            message: "expected null terminator".into(),
        })
    }
}

fn read_lenencoded(buf: &[u8]) -> Result<&str> {
    let length = i32_from_slice(&buf[..4]);
    assert!(buf.len() as i32 >= length + 4);
    try_to_str(&buf[4..4 + length as usize - 1])
}

fn try_to_str(data: &[u8]) -> Result<&str> {
    match std::str::from_utf8(data) {
        Ok(s) => Ok(s),
        Err(_) => Err(Error::Utf8EncodingError(data.into())),
    }
}
