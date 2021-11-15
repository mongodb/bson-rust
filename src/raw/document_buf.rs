use std::{
    borrow::{Borrow, Cow},
    convert::{TryFrom, TryInto},
    ffi::CString,
    ops::Deref,
};

use serde::{Deserialize, Serialize};

use crate::{de::MIN_BSON_DOCUMENT_SIZE, spec::ElementType, Document};

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
/// ```
/// # use bson::raw::Error;
/// use bson::raw::RawDocumentBuf;
///
/// let doc = RawDocumentBuf::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
/// let mut iter = doc.iter();
/// let (key, value) = iter.next().unwrap()?;
/// assert_eq!(key, "hi");
/// assert_eq!(value.as_str(), Some("y'all"));
/// assert!(iter.next().is_none());
/// # Ok::<(), Error>(())
/// ```
///
/// This type implements `Deref` to [`RawDocument`], meaning that all methods on [`RawDocument`] are
/// available on [`RawDocumentBuf`] values as well. This includes [`RawDocument::get`] or any of the
/// type-specific getters, such as [`RawDocument::get_object_id`] or [`RawDocument::get_str`]. Note
/// that accessing elements is an O(N) operation, as it requires iterating through the document from
/// the beginning to find the requested key.
///
/// ```
/// use bson::raw::RawDocumentBuf;
///
/// let doc = RawDocumentBuf::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
/// assert_eq!(doc.get_str("hi")?, "y'all");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, PartialEq)]
pub struct RawDocumentBuf {
    data: Vec<u8>,
}

impl RawDocumentBuf {
    pub fn empty() -> RawDocumentBuf {
        let mut data: Vec<u8> = MIN_BSON_DOCUMENT_SIZE.to_le_bytes().to_vec();
        data.push(0);
        Self { data }
    }

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
    ///
    /// ```
    /// # use bson::raw::{RawDocumentBuf, Error};
    /// let doc = RawDocumentBuf::new(b"\x05\0\0\0\0".to_vec())?;
    /// # Ok::<(), Error>(())
    /// ```
    pub fn new(data: Vec<u8>) -> Result<RawDocumentBuf> {
        let _ = RawDocument::new(data.as_slice())?;
        Ok(Self { data })
    }

    /// Create a [`RawDocumentBuf`] from a [`Document`].
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, oid::ObjectId, raw::RawDocumentBuf};
    ///
    /// let document = doc! {
    ///     "_id": ObjectId::new(),
    ///     "name": "Herman Melville",
    ///     "title": "Moby-Dick",
    /// };
    /// let doc = RawDocumentBuf::from_document(&document)?;
    /// # Ok::<(), Error>(())
    /// ```
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
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, raw::RawDocumentBuf};
    ///
    /// let doc = RawDocumentBuf::from_document(&doc! { "ferris": true })?;
    ///
    /// for element in doc.iter() {
    ///     let (key, value) = element?;
    ///     assert_eq!(key, "ferris");
    ///     assert_eq!(value.as_bool(), Some(true));
    /// }
    /// # Ok::<(), Error>(())
    /// ```
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
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, raw::RawDocumentBuf};
    ///
    /// let doc = RawDocumentBuf::from_document(&doc!{})?;
    /// assert_eq!(doc.into_vec(), b"\x05\x00\x00\x00\x00".to_vec());
    /// # Ok::<(), Error>(())
    /// ```
    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }

    pub fn append_document_buf(&mut self, key: impl AsRef<str>, value: RawDocumentBuf) {
        self.append(key, RawBson::Document(&value))
    }

    pub fn append<'a>(&mut self, key: impl AsRef<str>, value: RawBson<'a>) {
        let original_len = self.data.len();
        let key_bytes = key.as_ref().as_bytes();
        let key = CString::new(key_bytes).unwrap();
        self.data.extend(key.to_bytes_with_nul());
        match value {
            RawBson::Int32(i) => {
                self.data.extend(i.to_le_bytes());
            },
            RawBson::String(s) => {
                self.data.extend(((s.as_bytes().len() + 1) as i32).to_le_bytes());
                self.data.extend(s.as_bytes());
                self.data.push(0);
            },
            RawBson::Document(d) => {
                self.data.extend(d.as_bytes());
            }
            _ => todo!(),
        }
        // update element type
        self.data[original_len - 1] = value.element_type() as u8;
        // append trailing null byte
        self.data.push(0);
        // update length
        self.data
            .splice(0..4, (self.data.len() as i32).to_le_bytes());
    }

    pub fn to_document(&self) -> Result<Document> {
        self.as_ref().try_into()
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
