//! Conversion format for types that can directly convert to BSON bytes.
//!
//! Conversion trait impls with this format are implemented for the types in this crate that
//! represent toplevel documents, i.e. [`Document`], [`Array`], [`RawDocument`], [`RawDocumentBuf`],
//! [`RawArray`], [`RawArrayBuf`].

use super::{Format, FromBsonBytes, FromBsonBytesOwned, IntoBsonBytes};
use crate::{
    Array,
    Document,
    RawArray,
    RawArrayBuf,
    RawDocument,
    RawDocumentBuf,
    bson::array_append_to,
    error::Result,
};

/// Conversion to BSON bytes via type-specific implementation.
pub struct Direct;

impl Format for Direct {}

// RawDocument

impl IntoBsonBytes<Direct> for &RawDocument {
    fn append_bson_bytes(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.extend(self.as_bytes());
        Ok(())
    }
}

impl<'a> FromBsonBytes<'a, Direct> for &'a RawDocument {
    fn from_bson_bytes(bytes: &'a [u8]) -> Result<Self> {
        RawDocument::from_bytes(bytes)
    }
}

// RawDocumentBuf

impl IntoBsonBytes<Direct> for RawDocumentBuf {
    fn append_bson_bytes(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.append(&mut self.into_bytes());
        Ok(())
    }

    fn into_bson_bytes(self) -> Result<Vec<u8>> {
        Ok(self.into_bytes())
    }
}

impl IntoBsonBytes<Direct> for &RawDocumentBuf {
    fn append_bson_bytes(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.extend(self.as_bytes());
        Ok(())
    }
}

impl<'a> FromBsonBytes<'a, Direct> for RawDocumentBuf {
    fn from_bson_bytes(bytes: &'a [u8]) -> Result<Self> {
        Self::from_bytes(bytes.to_owned())
    }
}

impl FromBsonBytesOwned<Direct> for RawDocumentBuf {
    fn from_bson_bytes_owned(bytes: Vec<u8>) -> Result<Self> {
        Self::from_bytes(bytes)
    }
}

// RawArray

impl IntoBsonBytes<Direct> for &RawArray {
    fn append_bson_bytes(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.extend(self.as_bytes());
        Ok(())
    }
}

impl<'a> FromBsonBytes<'a, Direct> for &'a RawArray {
    fn from_bson_bytes(bytes: &'a [u8]) -> Result<Self> {
        RawDocument::from_bytes(bytes).map(RawArray::from_doc)
    }
}

// RawArrayBuf

impl IntoBsonBytes<Direct> for RawArrayBuf {
    fn append_bson_bytes(self, bytes: &mut Vec<u8>) -> Result<()> {
        let doc: RawDocumentBuf = self.into();
        bytes.append(&mut doc.into_bytes());
        Ok(())
    }

    fn into_bson_bytes(self) -> Result<Vec<u8>> {
        let doc: RawDocumentBuf = self.into();
        Ok(doc.into_bytes())
    }
}

impl IntoBsonBytes<Direct> for &RawArrayBuf {
    fn append_bson_bytes(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.extend(self.as_bytes());
        Ok(())
    }
}

impl<'a> FromBsonBytes<'a, Direct> for RawArrayBuf {
    fn from_bson_bytes(bytes: &'a [u8]) -> Result<Self> {
        Ok(Self::from_raw_document_buf(RawDocumentBuf::from_bytes(
            bytes.to_owned(),
        )?))
    }
}

impl FromBsonBytesOwned<Direct> for RawArrayBuf {
    fn from_bson_bytes_owned(bytes: Vec<u8>) -> Result<Self> {
        Ok(Self::from_raw_document_buf(RawDocumentBuf::from_bytes(
            bytes,
        )?))
    }
}

// Document

impl IntoBsonBytes<Direct> for Document {
    fn append_bson_bytes(self, bytes: &mut Vec<u8>) -> Result<()> {
        ok_or_truncate(bytes, |b| self.append_to(b))
    }
}

impl IntoBsonBytes<Direct> for &Document {
    fn append_bson_bytes(self, bytes: &mut Vec<u8>) -> Result<()> {
        ok_or_truncate(bytes, |b| self.append_to(b))
    }
}

fn ok_or_truncate(bytes: &mut Vec<u8>, f: impl FnOnce(&mut Vec<u8>) -> Result<()>) -> Result<()> {
    let start = bytes.len();
    let result = f(bytes);
    if result.is_err() {
        bytes.truncate(start);
    }
    result
}

impl<'a> FromBsonBytes<'a, Direct> for Document {
    fn from_bson_bytes(bytes: &'a [u8]) -> Result<Self> {
        RawDocument::from_bytes(bytes)?.try_into()
    }
}

impl FromBsonBytesOwned<Direct> for Document {}

// Array

impl IntoBsonBytes<Direct> for Array {
    fn append_bson_bytes(self, bytes: &mut Vec<u8>) -> Result<()> {
        ok_or_truncate(bytes, |b| array_append_to(&self, b))
    }
}

impl IntoBsonBytes<Direct> for &Array {
    fn append_bson_bytes(self, bytes: &mut Vec<u8>) -> Result<()> {
        ok_or_truncate(bytes, |b| array_append_to(self, b))
    }
}

impl<'a> FromBsonBytes<'a, Direct> for Array {
    fn from_bson_bytes(bytes: &'a [u8]) -> Result<Self> {
        RawArray::from_doc(RawDocument::from_bytes(bytes)?).try_into()
    }
}

impl FromBsonBytesOwned<Direct> for Array {}
