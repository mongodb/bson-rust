use std::{
    borrow::{Borrow, Cow},
    convert::TryFrom,
    ops::Deref,
};

use serde::{Deserialize, Serialize};

use crate::Document;

use super::{Error, ErrorKind, Iter, RawBson, RawDocument, Result};

/// An owned BSON document (akin to [`std::path::PathBuf`]), backed by a buffer of raw BSON bytes.
/// This can be created from a `Vec<u8>` or a [`crate::Document`].
///
/// Accessing elements within a [`RawDocumentBuf`] is similar to element access in
/// [`crate::Document`], but because the contents are parsed during iteration instead of at creation
/// time, format errors can happen at any time during use.
///
/// Iterating over a [`RawDocumentBuf`] yields either an error or a key-value pair that borrows from
/// the original document without making any additional allocations.
///
/// This type implements `Deref` to [`RawDocument`], meaning that all methods on [`RawDocument`] are
/// available on [`RawDocumentBuf`] values as well. This includes [`RawDocument::get`] or any of the
/// type-specific getters, such as [`RawDocument::get_object_id`] or [`RawDocument::get_str`]. Note
/// that accessing elements is an O(N) operation, as it requires iterating through the document from
/// the beginning to find the requested key.
#[derive(Clone, PartialEq)]
pub struct RawDocumentBuf {
    data: Vec<u8>,
}

impl RawDocumentBuf {
    /// Constructs a new [`RawDocumentBuf`], validating _only_ the
    /// following invariants:
    ///   * `data` is at least five bytes long (the minimum for a valid BSON document)
    ///   * the initial four bytes of `data` accurately represent the length of the bytes as
    ///     required by the BSON spec.
    ///   * the last byte of `data` is a 0
    ///
    /// Note that the internal structure of the bytes representing the
    /// BSON elements is _not_ validated at all by this method. If the
    /// bytes do not conform to the BSON spec, then method calls on
    /// the RawDocument will return Errors where appropriate.
    pub fn new(data: Vec<u8>) -> Result<RawDocumentBuf> {
        let _ = RawDocument::new(data.as_slice())?;
        Ok(Self { data })
    }

    /// Create a [`RawDocumentBuf`] from a [`Document`].
    pub fn from_document(doc: &Document) -> Result<RawDocumentBuf> {
        let mut data = Vec::new();
        doc.to_writer(&mut data).map_err(|e| Error {
            key: None,
            kind: ErrorKind::MalformedValue {
                message: e.to_string(),
            },
        })?;

        Ok(Self { data })
    }

    /// Gets an iterator over the elements in the [`RawDocumentBuf`], which yields
    /// `Result<(&str, RawBson<'_>)>`.
    ///
    /// # Note:
    ///
    /// There is no owning iterator for [`RawDocumentBuf`]. If you need ownership over
    /// elements that might need to allocate, you must explicitly convert
    /// them to owned types yourself.
    pub fn iter(&self) -> Iter<'_> {
        self.into_iter()
    }

    /// Return the contained data as a `Vec<u8>`
    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }
}

impl<'de> Deserialize<'de> for RawDocumentBuf {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // TODO: RUST-1045 implement visit_map to deserialize from arbitrary maps.
        let doc: &'de RawDocument = Deserialize::deserialize(deserializer)?;
        Ok(doc.to_owned())
    }
}

impl Serialize for RawDocumentBuf {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let doc: &RawDocument = self.deref();
        doc.serialize(serializer)
    }
}

impl std::fmt::Debug for RawDocumentBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawDocumentBuf")
            .field("data", &hex::encode(&self.data))
            .finish()
    }
}

impl<'a> From<RawDocumentBuf> for Cow<'a, RawDocument> {
    fn from(rd: RawDocumentBuf) -> Self {
        Cow::Owned(rd)
    }
}

impl<'a> From<&'a RawDocumentBuf> for Cow<'a, RawDocument> {
    fn from(rd: &'a RawDocumentBuf) -> Self {
        Cow::Borrowed(rd.as_ref())
    }
}

impl TryFrom<RawDocumentBuf> for Document {
    type Error = Error;

    fn try_from(raw: RawDocumentBuf) -> Result<Document> {
        Document::try_from(raw.as_ref())
    }
}

impl<'a> IntoIterator for &'a RawDocumentBuf {
    type IntoIter = Iter<'a>;
    type Item = Result<(&'a str, RawBson<'a>)>;

    fn into_iter(self) -> Iter<'a> {
        Iter::new(self)
    }
}

impl AsRef<RawDocument> for RawDocumentBuf {
    fn as_ref(&self) -> &RawDocument {
        RawDocument::new_unchecked(&self.data)
    }
}

impl Deref for RawDocumentBuf {
    type Target = RawDocument;

    fn deref(&self) -> &Self::Target {
        RawDocument::new_unchecked(&self.data)
    }
}

impl Borrow<RawDocument> for RawDocumentBuf {
    fn borrow(&self) -> &RawDocument {
        &*self
    }
}
