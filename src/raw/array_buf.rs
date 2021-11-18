use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
    iter::FromIterator,
};

use serde::{Deserialize, Serialize};

use crate::{
    raw::{
        serde::{OwnedOrBorrowedRawBson, OwnedOrBorrowedRawBsonVisitor},
        RAW_ARRAY_NEWTYPE,
    },
    spec::BinarySubtype,
    RawArray,
    RawBson,
    RawDocument,
    RawDocumentBuf,
};

use super::{owned_bson::OwnedRawBson, RawArrayIter};

/// An owned BSON array value (akin to [`std::path::PathBuf`]), backed by a buffer of raw BSON
/// bytes. This type can be used to construct owned array values, which can be used to append to
/// [`RawDocumentBuf`] or as a field in a `Deserialize` struct.
///
/// Iterating over a [`RawArrayBuf`] yields either an error or a key-value pair that borrows from
/// the original document without making any additional allocations.
/// ```
/// # use bson::raw::Error;
/// use bson::raw::RawDocumentBuf;
///
/// let mut array = RawArrayBuf::new();
/// array.append("a string");
/// array.append(12_i32);
///
/// let mut iter = array.iter();
///
/// let value = iter.next().unwrap()?;
/// assert_eq!(value.as_str(), Some("a string"));
///
/// let value = iter.next().unwrap()?;
/// assert_eq!(value.as_i32(), Some(12));
///
/// assert!(iter.next().is_none());
/// # Ok::<(), Error>(())
/// ```
///
/// This type implements `Deref` to [`RawArray`], meaning that all methods on [`RawArray`] are
/// available on [`RawArrayBuf`] values as well. This includes [`RawArray::get`] or any of the
/// type-specific getters, such as [`RawArray::get_object_id`] or [`RawArray::get_str`]. Note
/// that accessing elements is an O(N) operation, as it requires iterating through the document from
/// the beginning to find the requested key.
#[derive(Clone, PartialEq)]
pub struct RawArrayBuf {
    inner: RawDocumentBuf,
    len: usize,
}

impl RawArrayBuf {
    /// Construct a new, empty `RawArrayBuf`.
    pub fn new() -> RawArrayBuf {
        Self {
            inner: RawDocumentBuf::new(),
            len: 0,
        }
    }

    /// Construct a new `RawArrayBuf` from the provided `Vec` of bytes.
    ///
    /// This involves a traversal of the array to count the values.
    pub(crate) fn from_raw_document_buf(doc: RawDocumentBuf) -> Self {
        let len = doc.iter().count();
        Self { inner: doc, len }
    }

    /// Append a value to the end of the array.
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::raw::{RawArrayBuf, RawDocumentBuf};
    ///
    /// let mut array = RawArrayBuf::new();
    /// array.push("a string");
    /// array.push(12_i32);
    ///
    /// let mut doc = RawDocumentBuf::new();
    /// doc.append("a key", "a value");
    /// array.push(doc);
    ///
    /// assert_eq!(array.len(), 3);
    ///
    /// let mut iter = array.iter();
    ///
    /// let value = iter.next().unwrap()?;
    /// assert_eq!(value.as_str(), Some("a string"));
    ///
    /// let value = iter.next().unwrap()?;
    /// assert_eq!(value.as_i32(), Some(12));
    ///
    /// let value = iter.next().unwrap()?;
    /// assert_eq!(value.as_document(), Some(&doc));
    ///
    /// assert!(iter.next().is_none());
    /// # Ok::<(), Error>(())
    /// ```
    pub fn push(&mut self, value: impl Into<OwnedRawBson>) {
        self.inner.append(self.len.to_string(), value);
        self.len += 1;
    }

    /// Returns the number of elements in the [`RawArrayBuf`].
    ///
    /// To retrieve the number of BSON bytes in the backing buffer, use
    /// `.as_bytes().len()`.
    pub fn len(&self) -> usize {
        self.len
    }

    pub(crate) fn into_vec(self) -> Vec<u8> {
        self.inner.into_vec()
    }
}

impl Debug for RawArrayBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawArrayBuf")
            .field("data", &hex::encode(self.as_bytes()))
            .field("len", &self.len)
            .finish()
    }
}

impl std::ops::Deref for RawArrayBuf {
    type Target = RawArray;

    fn deref(&self) -> &Self::Target {
        RawArray::from_doc(&self.inner)
    }
}

impl AsRef<RawArray> for RawArrayBuf {
    fn as_ref(&self) -> &RawArray {
        RawArray::from_doc(&self.inner)
    }
}

impl Borrow<RawArray> for RawArrayBuf {
    fn borrow(&self) -> &RawArray {
        self.as_ref()
    }
}

impl<'a> IntoIterator for &'a RawArrayBuf {
    type IntoIter = RawArrayIter<'a>;
    type Item = super::Result<RawBson<'a>>;

    fn into_iter(self) -> RawArrayIter<'a> {
        self.as_ref().into_iter()
    }
}

impl<'a> From<RawArrayBuf> for Cow<'a, RawArray> {
    fn from(rd: RawArrayBuf) -> Self {
        Cow::Owned(rd)
    }
}

impl<'a> From<&'a RawArrayBuf> for Cow<'a, RawArray> {
    fn from(rd: &'a RawArrayBuf) -> Self {
        Cow::Borrowed(rd.as_ref())
    }
}

impl<T: Into<OwnedRawBson>> FromIterator<T> for RawArrayBuf {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut array_buf = RawArrayBuf::new();
        for item in iter {
            array_buf.push(item);
        }
        array_buf
    }
}

impl<'de> Deserialize<'de> for RawArrayBuf {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match deserializer
            .deserialize_newtype_struct(RAW_ARRAY_NEWTYPE, OwnedOrBorrowedRawBsonVisitor)?
        {
            OwnedOrBorrowedRawBson::Borrowed(RawBson::Array(d)) => Ok(d.to_owned()),
            OwnedOrBorrowedRawBson::Borrowed(RawBson::Binary(b))
                if b.subtype == BinarySubtype::Generic =>
            {
                let doc = RawDocument::new(b.bytes).map_err(serde::de::Error::custom)?;
                Ok(RawArray::from_doc(doc).to_owned())
            }
            OwnedOrBorrowedRawBson::Owned(OwnedRawBson::Array(d)) => Ok(d),
            OwnedOrBorrowedRawBson::Owned(OwnedRawBson::Binary(b))
                if b.subtype == BinarySubtype::Generic =>
            {
                let doc = RawDocumentBuf::from_bytes(b.bytes).map_err(serde::de::Error::custom)?;
                Ok(RawArrayBuf::from_raw_document_buf(doc))
            }
            b => Err(serde::de::Error::custom(format!(
                "expected raw BSON array, instead got {:?}",
                b
            ))),
        }
    }
}

impl Serialize for RawArrayBuf {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_ref().serialize(serializer)
    }
}
