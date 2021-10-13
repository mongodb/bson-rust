use std::{
    borrow::{Borrow, Cow},
    convert::TryFrom,
    ops::Deref,
};

use crate::Document;

use super::{Error, ErrorKind, Iter, RawBson, RawDoc, Result};

/// A BSON document, stored as raw bytes on the heap. This can be created from a `Vec<u8>` or
/// a [`bson::Document`].
///
/// Accessing elements within a [`RawDocument`] is similar to element access in [`bson::Document`],
/// but because the contents are parsed during iteration instead of at creation time, format errors
/// can happen at any time during use.
///
/// Iterating over a [`RawDocument`] yields either an error or a key-value pair that borrows from
/// the original document without making any additional allocations.
///
/// ```
/// # use bson::raw::{RawDocument, Error};
/// let doc = RawDocument::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
/// let mut iter = doc.iter();
/// let (key, value) = iter.next().unwrap()?;
/// assert_eq!(key, "hi");
/// assert_eq!(value.as_str(), Ok("y'all"));
/// assert!(iter.next().is_none());
/// # Ok::<(), Error>(())
/// ```
///
/// This type implements `Deref` to `RawDoc`, meaning that all methods on `RawDoc` slices are
/// available on `RawDocument` values as well. This includes [`RawDoc::get`] or any of the
/// type-specific getters, such as [`RawDoc::get_object_id`] or [`RawDoc::get_str`]. Note that
/// accessing elements is an O(N) operation, as it requires iterating through the document from the
/// beginning to find the requested key.
///
/// ```
/// # use bson::raw::{RawDocument, Error};
/// let doc = RawDocument::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
/// assert_eq!(doc.get_str("hi")?, Some("y'all"));
/// # Ok::<(), Error>(())
/// ```
#[derive(Clone, PartialEq)]
pub struct RawDocument {
    data: Vec<u8>,
}

impl RawDocument {
    /// Constructs a new RawDocument, validating _only_ the
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
    /// # use bson::raw::{RawDocument, Error};
    /// let doc = RawDocument::new(b"\x05\0\0\0\0".to_vec())?;
    /// # Ok::<(), Error>(())
    /// ```
    pub fn new(data: Vec<u8>) -> Result<RawDocument> {
        let _ = RawDoc::new(data.as_slice())?;
        Ok(Self { data })
    }

    /// Create a RawDocument from a Document.
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, oid::ObjectId, raw::RawDocument};
    ///
    /// let document = doc! {
    ///     "_id": ObjectId::new(),
    ///     "name": "Herman Melville",
    ///     "title": "Moby-Dick",
    /// };
    /// let doc = RawDocument::from_document(&document)?;
    /// # Ok::<(), Error>(())
    /// ```
    pub fn from_document(doc: &Document) -> Result<RawDocument> {
        let mut data = Vec::new();
        doc.to_writer(&mut data).map_err(|e| Error {
            key: None,
            kind: ErrorKind::MalformedValue {
                message: e.to_string(),
            },
        })?;

        Ok(Self { data })
    }

    /// Gets an iterator over the elements in the `RawDocument`, which yields `Result<&str,
    /// Element<'_>>`.
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, raw::RawDocument};
    ///
    /// let doc = RawDocument::from_document(&doc! { "ferris": true })?;
    ///
    /// for element in doc.iter() {
    ///     let (key, value) = element?;
    ///     assert_eq!(key, "ferris");
    ///     assert_eq!(value.as_bool()?, true);
    /// }
    /// # Ok::<(), Error>(())
    /// ```
    ///
    /// # Note:
    ///
    /// There is no owning iterator for [`RawDocument`]. If you need ownership over
    /// elements that might need to allocate, you must explicitly convert
    /// them to owned types yourself.
    pub fn iter(&self) -> Iter<'_> {
        self.into_iter()
    }

    /// Return the contained data as a `Vec<u8>`
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, raw::RawDocument};
    ///
    /// let doc = RawDocument::from_document(&doc!{})?;
    /// assert_eq!(doc.into_vec(), b"\x05\x00\x00\x00\x00".to_vec());
    /// # Ok::<(), Error>(())
    /// ```
    pub fn into_vec(self) -> Vec<u8> {
        self.data
    }
}

impl std::fmt::Debug for RawDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawDocument")
            .field("data", &hex::encode(&self.data))
            .finish()
    }
}

impl<'a> From<RawDocument> for Cow<'a, RawDoc> {
    fn from(rd: RawDocument) -> Self {
        Cow::Owned(rd)
    }
}

impl<'a> From<&'a RawDocument> for Cow<'a, RawDoc> {
    fn from(rd: &'a RawDocument) -> Self {
        Cow::Borrowed(rd.as_ref())
    }
}

impl TryFrom<RawDocument> for Document {
    type Error = Error;

    fn try_from(raw: RawDocument) -> Result<Document> {
        Document::try_from(raw.as_ref())
    }
}

impl<'a> IntoIterator for &'a RawDocument {
    type IntoIter = Iter<'a>;
    type Item = Result<(&'a str, RawBson<'a>)>;

    fn into_iter(self) -> Iter<'a> {
        Iter::new(self)
    }
}

impl AsRef<RawDoc> for RawDocument {
    fn as_ref(&self) -> &RawDoc {
        RawDoc::new_unchecked(&self.data)
    }
}

impl Deref for RawDocument {
    type Target = RawDoc;

    fn deref(&self) -> &Self::Target {
        RawDoc::new_unchecked(&self.data)
    }
}

impl Borrow<RawDoc> for RawDocument {
    fn borrow(&self) -> &RawDoc {
        &*self
    }
}
