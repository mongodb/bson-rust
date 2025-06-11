pub(crate) mod bson_visitor;
pub(crate) mod seeded_visitor;

use std::{borrow::Cow, convert::TryFrom, fmt::Debug};

use serde::{de::Error as SerdeError, Deserialize};

use crate::{
    raw::{RAW_ARRAY_NEWTYPE, RAW_DOCUMENT_NEWTYPE},
    spec::BinarySubtype,
    RawArray,
    RawArrayBuf,
    RawBsonRef,
    RawDocument,
    RawDocumentBuf,
};

use super::{bson::RawBson, RAW_BSON_NEWTYPE};
use bson_visitor::*;
use seeded_visitor::*;

/// Wrapper around a `Cow<str>` to enable borrowed deserialization.
/// The default [`Deserialize`] impl for [`Cow`] always uses the owned version.
#[derive(Debug, Deserialize)]
pub(crate) struct CowStr<'a>(#[serde(borrow)] Cow<'a, str>);

/// A raw BSON value that may either be borrowed or owned.
///
/// This is used to consolidate the [`Serialize`] and [`Deserialize`] implementations for
/// [`RawBson`] and [`OwnedRawBson`].
pub(crate) enum OwnedOrBorrowedRawBson<'a> {
    Owned(RawBson),
    Borrowed(RawBsonRef<'a>),
}

impl<'a> OwnedOrBorrowedRawBson<'a> {
    pub(crate) fn as_ref<'b>(&'b self) -> RawBsonRef<'b>
    where
        'a: 'b,
    {
        match self {
            Self::Borrowed(r) => *r,
            Self::Owned(bson) => bson.as_raw_bson_ref(),
        }
    }
}

impl<'a, 'de: 'a> Deserialize<'de> for OwnedOrBorrowedRawBson<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(RAW_BSON_NEWTYPE, OwnedOrBorrowedRawBsonVisitor)
    }
}

impl Debug for OwnedOrBorrowedRawBson<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owned(o) => o.fmt(f),
            Self::Borrowed(b) => b.fmt(f),
        }
    }
}

impl<'a> From<RawBsonRef<'a>> for OwnedOrBorrowedRawBson<'a> {
    fn from(b: RawBsonRef<'a>) -> Self {
        OwnedOrBorrowedRawBson::Borrowed(b)
    }
}

impl From<RawBson> for OwnedOrBorrowedRawBson<'_> {
    fn from(b: RawBson) -> Self {
        OwnedOrBorrowedRawBson::Owned(b)
    }
}

/// Wrapper type that can deserialize either an owned or a borrowed raw BSON document.
#[derive(Debug)]
pub(crate) enum OwnedOrBorrowedRawDocument<'a> {
    Owned(RawDocumentBuf),
    Borrowed(&'a RawDocument),
}

impl OwnedOrBorrowedRawDocument<'_> {
    pub(crate) fn into_owned(self) -> RawDocumentBuf {
        match self {
            Self::Owned(o) => o,
            Self::Borrowed(b) => b.to_owned(),
        }
    }
}

impl From<RawDocumentBuf> for OwnedOrBorrowedRawDocument<'_> {
    fn from(doc: RawDocumentBuf) -> Self {
        Self::Owned(doc)
    }
}

impl<'a> From<&'a RawDocument> for OwnedOrBorrowedRawDocument<'a> {
    fn from(doc: &'a RawDocument) -> Self {
        Self::Borrowed(doc)
    }
}

impl<'a, 'de: 'a> TryFrom<CowByteBuffer<'de>> for OwnedOrBorrowedRawDocument<'a> {
    type Error = crate::raw::Error;

    fn try_from(buffer: CowByteBuffer<'de>) -> Result<Self, Self::Error> {
        let doc = match buffer.0 {
            Some(Cow::Borrowed(borrowed)) => RawDocument::decode_from_bytes(borrowed)?.into(),
            Some(Cow::Owned(owned)) => RawDocumentBuf::decode_from_bytes(owned)?.into(),
            None => RawDocumentBuf::new().into(),
        };
        Ok(doc)
    }
}

impl<'a, 'de: 'a> Deserialize<'de> for OwnedOrBorrowedRawDocument<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match deserializer
            .deserialize_newtype_struct(RAW_DOCUMENT_NEWTYPE, OwnedOrBorrowedRawBsonVisitor)?
        {
            OwnedOrBorrowedRawBson::Borrowed(RawBsonRef::Document(d)) => Ok(Self::Borrowed(d)),
            OwnedOrBorrowedRawBson::Owned(RawBson::Document(d)) => Ok(Self::Owned(d)),

            // For non-BSON formats, RawDocument gets serialized as bytes, so we need to deserialize
            // from them here too. For BSON, the deserializer will return an error if it
            // sees the RAW_DOCUMENT_NEWTYPE but the next type isn't a document.
            OwnedOrBorrowedRawBson::Borrowed(RawBsonRef::Binary(b))
                if b.subtype == BinarySubtype::Generic =>
            {
                Ok(Self::Borrowed(
                    RawDocument::decode_from_bytes(b.bytes).map_err(SerdeError::custom)?,
                ))
            }
            OwnedOrBorrowedRawBson::Owned(RawBson::Binary(b))
                if b.subtype == BinarySubtype::Generic =>
            {
                Ok(Self::Owned(
                    RawDocumentBuf::decode_from_bytes(b.bytes).map_err(SerdeError::custom)?,
                ))
            }

            o => Err(SerdeError::custom(format!(
                "expected raw document, instead got {:?}",
                o
            ))),
        }
    }
}

/// Wrapper type that can deserialize either an owned or a borrowed raw BSON array.
#[derive(Debug)]
pub(crate) enum OwnedOrBorrowedRawArray<'a> {
    Owned(RawArrayBuf),
    Borrowed(&'a RawArray),
}

impl OwnedOrBorrowedRawArray<'_> {
    pub(crate) fn into_owned(self) -> RawArrayBuf {
        match self {
            Self::Owned(o) => o,
            Self::Borrowed(b) => b.to_owned(),
        }
    }
}

impl<'a, 'de: 'a> Deserialize<'de> for OwnedOrBorrowedRawArray<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match deserializer
            .deserialize_newtype_struct(RAW_ARRAY_NEWTYPE, OwnedOrBorrowedRawBsonVisitor)?
        {
            OwnedOrBorrowedRawBson::Borrowed(RawBsonRef::Array(d)) => Ok(Self::Borrowed(d)),
            OwnedOrBorrowedRawBson::Owned(RawBson::Array(d)) => Ok(Self::Owned(d)),

            // For non-BSON formats, RawArray gets serialized as bytes, so we need to deserialize
            // from them here too. For BSON, the deserializer will return an error if it
            // sees the RAW_DOCUMENT_NEWTYPE but the next type isn't a document.
            OwnedOrBorrowedRawBson::Borrowed(RawBsonRef::Binary(b))
                if b.subtype == BinarySubtype::Generic =>
            {
                let doc = RawDocument::decode_from_bytes(b.bytes).map_err(SerdeError::custom)?;
                Ok(Self::Borrowed(RawArray::from_doc(doc)))
            }
            OwnedOrBorrowedRawBson::Owned(RawBson::Binary(b))
                if b.subtype == BinarySubtype::Generic =>
            {
                let doc = RawDocumentBuf::decode_from_bytes(b.bytes).map_err(SerdeError::custom)?;
                Ok(Self::Owned(RawArrayBuf::from_raw_document_buf(doc)))
            }

            o => Err(SerdeError::custom(format!(
                "expected raw array, instead got {:?}",
                o
            ))),
        }
    }
}
