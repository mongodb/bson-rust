//! Pluggable conversion impls for types that can directly convert to BSON bytes.

use super::{FromBson, FromBsonOwned, IntoBson};
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
pub struct BsonDirect;

// RawDocument

impl IntoBson<BsonDirect> for &RawDocument {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.extend(self.as_bytes());
        Ok(())
    }
}

impl<'a> FromBson<'a, BsonDirect> for &'a RawDocument {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        RawDocument::from_bytes(bytes)
    }
}

// RawDocumentBuf

impl IntoBson<BsonDirect> for RawDocumentBuf {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.append(&mut self.into_bytes());
        Ok(())
    }

    fn into_bson_vec(self) -> Result<Vec<u8>> {
        Ok(self.into_bytes())
    }
}

impl IntoBson<BsonDirect> for &RawDocumentBuf {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.extend(self.as_bytes());
        Ok(())
    }
}

impl<'a> FromBson<'a, BsonDirect> for RawDocumentBuf {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        Self::from_bytes(bytes.to_owned())
    }
}

impl FromBsonOwned<BsonDirect> for RawDocumentBuf {
    fn from_bson_vec(bytes: Vec<u8>) -> Result<Self> {
        Self::from_bytes(bytes)
    }
}

// RawArray

impl IntoBson<BsonDirect> for &RawArray {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.extend(self.as_bytes());
        Ok(())
    }
}

impl<'a> FromBson<'a, BsonDirect> for &'a RawArray {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        RawDocument::from_bytes(bytes).map(RawArray::from_doc)
    }
}

// RawArrayBuf

impl IntoBson<BsonDirect> for RawArrayBuf {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        let doc: RawDocumentBuf = self.into();
        bytes.append(&mut doc.into_bytes());
        Ok(())
    }

    fn into_bson_vec(self) -> Result<Vec<u8>> {
        let doc: RawDocumentBuf = self.into();
        Ok(doc.into_bytes())
    }
}

impl IntoBson<BsonDirect> for &RawArrayBuf {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.extend(self.as_bytes());
        Ok(())
    }
}

impl<'a> FromBson<'a, BsonDirect> for RawArrayBuf {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        Ok(Self::from_raw_document_buf(RawDocumentBuf::from_bytes(
            bytes.to_owned(),
        )?))
    }
}

impl FromBsonOwned<BsonDirect> for RawArrayBuf {
    fn from_bson_vec(bytes: Vec<u8>) -> Result<Self> {
        Ok(Self::from_raw_document_buf(RawDocumentBuf::from_bytes(
            bytes,
        )?))
    }
}

// Document

impl IntoBson<BsonDirect> for Document {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        ok_or_truncate(bytes, |b| self.append_to(b))
    }
}

impl IntoBson<BsonDirect> for &Document {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
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

impl<'a> FromBson<'a, BsonDirect> for Document {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        RawDocument::from_bytes(bytes)?.try_into()
    }
}

impl FromBsonOwned<BsonDirect> for Document {}

// Array

impl IntoBson<BsonDirect> for Array {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        ok_or_truncate(bytes, |b| array_append_to(&self, b))
    }
}

impl IntoBson<BsonDirect> for &Array {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        ok_or_truncate(bytes, |b| array_append_to(&self, b))
    }
}

impl<'a> FromBson<'a, BsonDirect> for Array {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        RawArray::from_doc(RawDocument::from_bytes(bytes)?).try_into()
    }
}

impl FromBsonOwned<BsonDirect> for Array {}
