//! Pluggable conversion impls for types that can directly convert to BSON bytes.

use super::{FromBson, FromBsonOwned, IntoBson};
use crate::{Array, Document, RawArray, RawArrayBuf, RawDocument, RawDocumentBuf, error::Result};

/// Conversion to BSON bytes via type-specific implementation.
pub struct BsonRaw;

// RawDocument

impl IntoBson<BsonRaw> for &RawDocument {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.extend(self.as_bytes());
        Ok(())
    }
}

impl<'a> FromBson<'a, BsonRaw> for &'a RawDocument {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        RawDocument::from_bytes(bytes)
    }
}

// RawDocumentBuf

impl IntoBson<BsonRaw> for RawDocumentBuf {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.append(&mut self.into_bytes());
        Ok(())
    }

    fn into_bson_vec(self) -> Result<Vec<u8>> {
        Ok(self.into_bytes())
    }
}

impl IntoBson<BsonRaw> for &RawDocumentBuf {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.extend(self.as_bytes());
        Ok(())
    }
}

impl<'a> FromBson<'a, BsonRaw> for RawDocumentBuf {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        Self::from_bytes(bytes.to_owned())
    }
}

impl FromBsonOwned<BsonRaw> for RawDocumentBuf {
    fn from_bson_vec(bytes: Vec<u8>) -> Result<Self> {
        Self::from_bytes(bytes)
    }
}

// RawArray

impl IntoBson<BsonRaw> for &RawArray {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.extend(self.as_bytes());
        Ok(())
    }
}

impl<'a> FromBson<'a, BsonRaw> for &'a RawArray {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        RawDocument::from_bytes(bytes).map(RawArray::from_doc)
    }
}

// RawArrayBuf

impl IntoBson<BsonRaw> for RawArrayBuf {
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

impl IntoBson<BsonRaw> for &RawArrayBuf {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        bytes.extend(self.as_bytes());
        Ok(())
    }
}

impl<'a> FromBson<'a, BsonRaw> for RawArrayBuf {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        Ok(Self::from_raw_document_buf(RawDocumentBuf::from_bytes(
            bytes.to_owned(),
        )?))
    }
}

impl FromBsonOwned<BsonRaw> for RawArrayBuf {
    fn from_bson_vec(bytes: Vec<u8>) -> Result<Self> {
        Ok(Self::from_raw_document_buf(RawDocumentBuf::from_bytes(
            bytes,
        )?))
    }
}

// Document

impl IntoBson<BsonRaw> for Document {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        let raw: RawDocumentBuf = self.try_into()?;
        raw.into_bson(bytes)
    }

    fn into_bson_vec(self) -> Result<Vec<u8>> {
        let raw: RawDocumentBuf = self.try_into()?;
        raw.into_bson_vec()
    }
}

impl IntoBson<BsonRaw> for &Document {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        let raw: RawDocumentBuf = self.try_into()?;
        raw.into_bson(bytes)
    }

    fn into_bson_vec(self) -> Result<Vec<u8>> {
        let raw: RawDocumentBuf = self.try_into()?;
        raw.into_bson_vec()
    }
}

impl<'a> FromBson<'a, BsonRaw> for Document {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        let raw = super::from_bson::<BsonRaw, RawDocumentBuf>(bytes)?;
        raw.try_into()
    }
}

impl FromBsonOwned<BsonRaw> for Document {
    fn from_bson_vec(bytes: Vec<u8>) -> Result<Self> {
        let raw = super::from_bson_vec::<BsonRaw, RawDocumentBuf>(bytes)?;
        raw.try_into()
    }
}

// Array

impl IntoBson<BsonRaw> for Array {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        let raw: RawArrayBuf = self.try_into()?;
        raw.into_bson(bytes)
    }

    fn into_bson_vec(self) -> Result<Vec<u8>> {
        let raw: RawArrayBuf = self.try_into()?;
        raw.into_bson_vec()
    }
}

impl IntoBson<BsonRaw> for &Array {
    fn into_bson(self, bytes: &mut Vec<u8>) -> Result<()> {
        let raw: RawArrayBuf = self.try_into()?;
        raw.into_bson(bytes)
    }

    fn into_bson_vec(self) -> Result<Vec<u8>> {
        let raw: RawArrayBuf = self.try_into()?;
        raw.into_bson_vec()
    }
}

impl<'a> FromBson<'a, BsonRaw> for Array {
    fn from_bson(bytes: &'a [u8]) -> Result<Self> {
        let raw = super::from_bson::<BsonRaw, RawArrayBuf>(bytes)?;
        raw.try_into()
    }
}

impl FromBsonOwned<BsonRaw> for Array {
    fn from_bson_vec(bytes: Vec<u8>) -> Result<Self> {
        let raw = super::from_bson_vec::<BsonRaw, RawArrayBuf>(bytes)?;
        raw.try_into()
    }
}
