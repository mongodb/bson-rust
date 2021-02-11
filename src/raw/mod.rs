//! A RawDocument can be created from a `Vec<u8>` containing raw BSON data, and elements
//! accessed via methods similar to those available on the Document type. Note that rawbson returns
//! a RawResult<Option<T>>, since the bytes contained in the document are not fully validated until
//! trying to access the contained data.
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

mod elem;
#[cfg(test)]
mod props;

use std::{
    borrow::Borrow,
    convert::{TryFrom, TryInto},
    ops::Deref,
};

use chrono::{DateTime, Utc};

#[cfg(feature = "decimal128")]
use crate::decimal128::Decimal128;
use crate::{oid::ObjectId, spec::ElementType, Bson, Document};
pub use elem::{RawBinary, RawBson, RawJavaScriptCodeWithScope, RawRegex, RawTimestamp};

/// An error that occurs when attempting to parse raw BSON bytes.
#[derive(Debug, PartialEq)]
pub enum RawError {
    /// A BSON value did not fit the expected type.
    UnexpectedType,

    /// A BSON value did not fit the proper format.
    MalformedValue { message: String },

    /// Improper UTF-8 bytes were found when proper UTF-8 was expected. The error value contains
    /// the malformed data as bytes.
    Utf8EncodingError(Vec<u8>),
}

impl std::fmt::Display for RawError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::UnexpectedType => write!(f, "unexpected type"),
            Self::MalformedValue { message } => write!(f, "malformed value: {:?}", message),
            Self::Utf8EncodingError(_) => write!(f, "utf-8 encoding error"),
        }
    }
}

impl std::error::Error for RawError {}

pub type RawResult<T> = Result<T, RawError>;

/// A BSON document, stored as raw bytes on the heap. This can be created from a `Vec<u8>` or
/// a [`bson::Document`].
///
/// Accessing elements within a `RawDocument` is similar to element access in [bson::Document], but
/// because the contents are parsed during iteration, instead of at creation time, format errors can
/// happen at any time during use.
///
/// Iterating over a RawDocument yields either an error or a key-value pair that borrows from the
/// original document without making any additional allocations.
///
/// ```
/// # use bson::raw::{RawDocument, RawError};
/// let doc = RawDocument::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
/// let mut iter = doc.iter();
/// let (key, value) = iter.next().unwrap()?;
/// assert_eq!(key, "hi");
/// assert_eq!(value.as_str(), Ok("y'all"));
/// assert!(iter.next().is_none());
/// # Ok::<(), RawError>(())
/// ```
///
/// Individual elements can be accessed using [`RawDocument::get`](RawDocument::get) or any of the
/// type-specific getters, such as [`RawDocument::get_object_id`](RawDocument::get_object_id) or
/// [`RawDocument::get_str`](RawDocument::get_str). Note that accessing elements is an O(N)
/// operation, as it requires iterating through the document from the beginning to find the
/// requested key.
///
/// ```
/// # use bson::raw::{RawDocument, RawError};
/// let doc = RawDocument::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
/// assert_eq!(doc.get_str("hi")?, Some("y'all"));
/// # Ok::<(), RawError>(())
/// ```
#[derive(Clone, Debug)]
pub struct RawDocument {
    data: Box<[u8]>,
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
    /// the RawDocument will return RawErrors where appropriate.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, RawError};
    /// let doc = RawDocument::new(b"\x05\0\0\0\0".to_vec())?;
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn new(data: Vec<u8>) -> RawResult<RawDocument> {
        if data.len() < 5 {
            return Err(RawError::MalformedValue {
                message: "document too short".into(),
            });
        }

        let length = i32_from_slice(&data[..4]);

        if data.len() as i32 != length {
            return Err(RawError::MalformedValue {
                message: "document length incorrect".into(),
            });
        }

        if data[data.len() - 1] != 0 {
            return Err(RawError::MalformedValue {
                message: "document not null-terminated".into(),
            });
        }

        Ok(Self {
            data: data.into_boxed_slice(),
        })
    }

    /// Create a RawDocument from a Document.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, RawError};
    /// use bson::{doc, oid::ObjectId};
    ///
    /// let document = doc! {
    ///     "_id": ObjectId::new(),
    ///     "name": "Herman Melville",
    ///     "title": "Moby-Dick",
    /// };
    /// let doc = RawDocument::from_document(&document);
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn from_document(doc: &Document) -> RawDocument {
        let mut data = Vec::new();
        doc.to_writer(&mut data).unwrap();

        Self {
            data: data.into_boxed_slice(),
        }
    }

    /// Gets an iterator over the elements in the `RawDocument`, which yields `Result<&str,
    /// Element<'_>>`.
    ///
    /// ```
    /// # use bson::raw::{elem, RawDocument, RawError};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! { "ferris": true });
    ///
    /// for element in doc.iter() {
    ///     let (key, value) = element?;
    ///     assert_eq!(key, "ferris");
    ///     assert_eq!(value.as_bool()?, true);
    /// }
    /// # Ok::<(), RawError>(())
    /// ```
    ///
    /// # Note:
    ///
    /// There is no owning iterator for RawDocument.  If you need ownership over
    /// elements that might need to allocate, you must explicitly convert
    /// them to owned types yourself.
    pub fn iter(&self) -> RawDocumentIter<'_> {
        self.into_iter()
    }

    /// Return the contained data as a `Vec<u8>`
    ///
    /// ```
    /// # use bson::raw::RawDocument;
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc!{});
    /// assert_eq!(doc.into_inner(), b"\x05\x00\x00\x00\x00".to_vec());
    /// ```
    pub fn into_inner(self) -> Vec<u8> {
        self.data.to_vec()
    }
}

impl TryFrom<RawDocument> for Document {
    type Error = RawError;

    fn try_from(raw: RawDocument) -> RawResult<Document> {
        Document::try_from(raw.as_ref())
    }
}

impl<'a> IntoIterator for &'a RawDocument {
    type IntoIter = RawDocumentIter<'a>;
    type Item = RawResult<(&'a str, RawBson<'a>)>;

    fn into_iter(self) -> RawDocumentIter<'a> {
        RawDocumentIter {
            doc: &self,
            offset: 4,
        }
    }
}

impl AsRef<RawDocumentRef> for RawDocument {
    fn as_ref(&self) -> &RawDocumentRef {
        RawDocumentRef::new_unchecked(&self.data)
    }
}

impl Borrow<RawDocumentRef> for RawDocument {
    fn borrow(&self) -> &RawDocumentRef {
        &*self
    }
}

impl ToOwned for RawDocumentRef {
    type Owned = RawDocument;

    fn to_owned(&self) -> Self::Owned {
        self.to_raw_document()
    }
}

/// A BSON document referencing raw bytes stored elsewhere. This can be created from a
/// [RawDocument] or any type that contains valid BSON data, and can be referenced as a `[u8]`,
/// including static binary literals, [Vec<u8>](std::vec::Vec), or arrays.
///
/// Accessing elements within a `RawDocumentRef` is similar to element access in [bson::Document],
/// but because the contents are parsed during iteration, instead of at creation time, format errors
/// can happen at any time during use.
///
/// Iterating over a RawDocumentRef yields either an error or a key-value pair that borrows from the
/// original document without making any additional allocations.

/// ```
/// # use bson::raw::{Doc, RawError};
/// let doc = RawDocumentRef::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00")?;
/// let mut iter = doc.into_iter();
/// let (key, value) = iter.next().unwrap()?;
/// assert_eq!(key, "hi");
/// assert_eq!(value.as_str(), Ok("y'all"));
/// assert!(iter.next().is_none());
/// # Ok::<(), RawError>(())
/// ```
///
/// Individual elements can be accessed using [`RawDocumentRef::get`](RawDocumentRef::get) or any of
/// the type-specific getters, such as
/// [`RawDocumentRef::get_object_id`](RawDocumentRef::get_object_id) or [`RawDocumentRef::
/// get_str`](RawDocumentRef::get_str). Note that accessing elements is an O(N) operation, as it
/// requires iterating through the document from the beginning to find the requested key.
///
/// ```
/// # use bson::raw::{RawDocument, RawError};
/// let doc = RawDocument::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
/// assert_eq!(doc.get_str("hi")?, Some("y'all"));
/// # Ok::<(), RawError>(())
/// ```
#[derive(Debug)]
pub struct RawDocumentRef {
    data: [u8],
}

impl RawDocumentRef {
    /// Constructs a new RawDocumentRef, validating _only_ the
    /// following invariants:
    ///   * `data` is at least five bytes long (the minimum for a valid BSON document)
    ///   * the initial four bytes of `data` accurately represent the length of the bytes as
    ///     required by the BSON spec.
    ///   * the last byte of `data` is a 0
    ///
    /// Note that the internal structure of the bytes representing the
    /// BSON elements is _not_ validated at all by this method. If the
    /// bytes do not conform to the BSON spec, then method calls on
    /// the RawDocument will return RawErrors where appropriate.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, RawError};
    /// let doc = RawDocumentRef::new(b"\x05\0\0\0\0")?;
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn new<D: AsRef<[u8]> + ?Sized>(data: &D) -> RawResult<&RawDocumentRef> {
        let data = data.as_ref();

        if data.len() < 5 {
            return Err(RawError::MalformedValue {
                message: "document too short".into(),
            });
        }

        let length = i32_from_slice(&data[..4]);

        if data.len() as i32 != length {
            return Err(RawError::MalformedValue {
                message: "document length incorrect".into(),
            });
        }

        if data[data.len() - 1] != 0 {
            return Err(RawError::MalformedValue {
                message: "document not null-terminated".into(),
            });
        }

        Ok(RawDocumentRef::new_unchecked(data))
    }

    /// Creates a new Doc referencing the provided data slice.
    fn new_unchecked<D: AsRef<[u8]> + ?Sized>(data: &D) -> &RawDocumentRef {
        // SAFETY:
        //
        // Dereferencing a raw pointer requires unsafe due to the potential that the pointer is
        // null, dangling, or misaligned. We know the pointer is not null or dangling due to the
        // fact that it's created by a safe reference. Converting &[u8] to *const [u8] will be
        // properly aligned due to them being references to the same type, and converting *const
        // [u8] to *const RawDocumentRef is aligned due to the fact that the only field in a
        // RawDocumentRef is a [u8], meaning the structs are represented identically at the byte
        // level.
        unsafe { &*(data.as_ref() as *const [u8] as *const RawDocumentRef) }
    }

    /// Creates a new RawDocument with an owned copy of the BSON bytes.
    ///
    /// ```
    /// # use bson::raw::{Doc, RawError};
    /// use bson::raw::RawDocument;
    ///
    /// let data = b"\x05\0\0\0\0";
    /// let doc_ref = RawDocumentRef::new(data)?;
    /// let doc: RawDocument = doc_ref.to_raw_document();
    /// # Ok::<(), RawError>(())
    pub fn to_raw_document(&self) -> RawDocument {
        RawDocument {
            data: self.data.to_owned().into_boxed_slice(),
        }
    }

    /// Gets a reference to the value corresponding to the given key by iterating until the key is
    /// found.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem::Element, RawError};
    /// #
    /// use bson::{doc, oid::ObjectId};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "_id": ObjectId::new(),
    ///     "f64": 2.5,
    /// });
    ///
    /// let element = doc.get("f64")?.expect("finding key f64");
    /// assert_eq!(element.as_f64(), Ok(2.5));
    /// assert!(doc.get("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get<'a>(&'a self, key: &str) -> RawResult<Option<RawBson<'a>>> {
        for result in self.into_iter() {
            let (k, v) = result?;
            if key == k {
                return Ok(Some(v));
            }
        }
        Ok(None)
    }

    fn get_with<'a, T>(
        &'a self,
        key: &str,
        f: impl FnOnce(elem::RawBson<'a>) -> RawResult<T>,
    ) -> RawResult<Option<T>> {
        self.get(key)?.map(f).transpose()
    }

    /// Gets a reference to the BSON double value corresponding to a given key or returns an error
    /// if the key corresponds to a value which isn't a double.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem::Element, RawError};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "bool": true,
    ///     "f64": 2.5,
    /// });
    ///
    /// assert_eq!(doc.get_f64("f64"), Ok(Some(2.5)));
    /// assert_eq!(doc.get_f64("bool"), Err(RawError::UnexpectedType));
    /// assert_eq!(doc.get_f64("unknown"), Ok(None));
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_f64(&self, key: &str) -> RawResult<Option<f64>> {
        self.get_with(key, elem::RawBson::as_f64)
    }

    /// Gets a reference to the string value corresponding to a given key or returns an error if the
    /// key corresponds to a value which isn't a string.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem::Element, RawError};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "string": "hello",
    ///     "bool": true,
    /// });
    ///
    /// assert_eq!(doc.get_str("string"), Ok(Some("hello")));
    /// assert_eq!(doc.get_str("bool"), Err(RawError::UnexpectedType));
    /// assert_eq!(doc.get_str("unknown"), Ok(None));
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_str<'a>(&'a self, key: &str) -> RawResult<Option<&'a str>> {
        self.get_with(key, elem::RawBson::as_str)
    }

    /// Gets a reference to the document value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a document.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem::Element, RawError};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "doc": { "key": "value"},
    ///     "bool": true,
    /// });
    ///
    /// assert_eq!(doc.get_document("doc")?.expect("finding key doc").get_str("key"), Ok(Some("value")));
    /// assert_eq!(doc.get_document("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(doc.get_document("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_document<'a>(&'a self, key: &str) -> RawResult<Option<&'a RawDocumentRef>> {
        self.get_with(key, elem::RawBson::as_document)
    }

    /// Gets a reference to the array value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't an array.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem::Element, RawError};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "array": [true, 3],
    ///     "bool": true,
    /// });
    ///
    /// let mut arr_iter = docbuf.get_array("array")?.expect("finding key array").into_iter();
    /// let _: bool = arriter.next().unwrap()?.as_bool()?;
    /// let _: i32 = arriter.next().unwrap()?.as_i32()?;
    ///
    /// assert!(arr_iter.next().is_none());
    /// assert!(doc.get_array("bool").is_err());
    /// assert!(doc.get_array("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_array<'a>(&'a self, key: &str) -> RawResult<Option<&'a RawArray>> {
        self.get_with(key, elem::RawBson::as_array)
    }

    /// Gets a reference to the BSON binary value corresponding to a given key or returns an error
    /// if the key corresponds to a value which isn't a binary value.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem, RawError};
    ///
    /// use bson::{
    ///     spec::BinarySubtype
    ///     doc, Binary,
    /// };
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1, 2, 3] },
    ///     "bool": true,
    /// });
    ///
    /// assert_eq!(doc.get_binary("binary")?.map(elem::RawBsonBinary::as_bytes), Some(&[1, 2, 3][..]));
    /// assert_eq!(doc.get_binary("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(doc.get_binary("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_binary<'a>(&'a self, key: &str) -> RawResult<Option<elem::RawBinary<'a>>> {
        self.get_with(key, elem::RawBson::as_binary)
    }

    /// Gets a reference to the ObjectId value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't an ObjectId.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, RawError};
    /// use bson::{doc, oid::ObjectId};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "_id": ObjectId::new(),
    ///     "bool": true,
    /// });
    ///
    /// let oid = doc.get_object_id("_id")?.unwrap();
    /// assert_eq!(doc.get_object_id("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(doc.get_object_id("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_object_id(&self, key: &str) -> RawResult<Option<ObjectId>> {
        self.get_with(key, elem::RawBson::as_object_id)
    }

    /// Gets a reference to the boolean value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a boolean.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, RawError};
    /// use bson::{doc, oid::ObjectId};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "_id": ObjectId::new(),
    ///     "bool": true,
    /// });
    ///
    /// assert!(doc.get_bool("bool")?.unwrap());
    /// assert_eq!(doc.get_bool("_id").unwrap_err(), RawError::UnexpectedType);
    /// assert!(doc.get_object_id("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_bool(&self, key: &str) -> RawResult<Option<bool>> {
        self.get_with(key, elem::RawBson::as_bool)
    }

    /// Gets a reference to the BSON DateTime value corresponding to a given key or returns an error
    /// if the key corresponds to a value which isn't a DateTime.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, RawError};
    /// use bson::doc;
    /// use chrono::{Utc, Datelike, TimeZone};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "created_at": Utc.ymd(2020, 3, 15).and_hms(17, 0, 0),
    ///     "bool": true,
    /// });
    /// assert_eq!(doc.get_datetime("created_at")?.unwrap().year(), 2020);
    /// assert_eq!(doc.get_datetime("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(doc.get_datetime("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_datetime(&self, key: &str) -> RawResult<Option<DateTime<Utc>>> {
        self.get_with(key, elem::RawBson::as_datetime)
    }
    /// Gets a reference to the BSON regex value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a regex.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, RawError, elem};
    /// use bson::{doc, Regex};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "regex": Regex {
    ///         pattern: r"end\s*$".into(),
    ///         options: "i".into(),
    ///     },
    ///     "bool": true,
    /// });
    ///
    /// assert_eq!(doc.get_regex("regex")?.unwrap().pattern(), r"end\s*$");
    /// assert_eq!(doc.get_regex("regex")?.unwrap().options(), "i");
    /// assert_eq!(doc.get_regex("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(doc.get_regex("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_regex<'a>(&'a self, key: &str) -> RawResult<Option<elem::RawRegex<'a>>> {
        self.get_with(key, elem::RawBson::as_regex)
    }

    /// Gets a reference to the BSON timestamp value corresponding to a given key or returns an
    /// error if the key corresponds to a value which isn't a timestamp.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem, RawError};
    /// use bson::{doc, Timestamp};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "bool": true,
    ///     "ts": Timestamp { time: 649876543, increment: 9 },
    /// });
    ///
    /// let timestamp = doc.get_timestamp("ts")?.unwrap();
    ///
    /// assert_eq!(timestamp.time(), 649876543);
    /// assert_eq!(timestamp.increment(), 9);
    /// assert_eq!(doc.get_timestamp("bool"), Err(RawError::UnexpectedType));
    /// assert_eq!(doc.get_timestamp("unknown"), Ok(None));
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_timestamp<'a>(&'a self, key: &str) -> RawResult<Option<elem::RawTimestamp<'a>>> {
        self.get_with(key, elem::RawBson::as_timestamp)
    }

    /// Gets a reference to the BSON int32 value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a 32-bit integer.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, RawError};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "bool": true,
    ///     "i32": 1_000_000,
    /// });
    ///
    /// assert_eq!(doc.get_i32("i32"), Ok(Some(1_000_000)));
    /// assert_eq!(doc.get_i32("bool"), Err(RawError::UnexpectedType));
    /// assert_eq!(doc.get_i32("unknown"), Ok(None));
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_i32(&self, key: &str) -> RawResult<Option<i32>> {
        self.get_with(key, elem::RawBson::as_i32)
    }

    /// Gets a reference to the BSON int64 value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a 64-bit integer.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem::Element, RawError};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "bool": true,
    ///     "i64": 9223372036854775807_i64,
    /// });
    ///
    /// assert_eq!(doc.get_i64("i64"), Ok(Some(9223372036854775807)));
    /// assert_eq!(doc.get_i64("bool"), Err(RawError::UnexpectedType));
    /// assert_eq!(doc.get_i64("unknown"), Ok(None));
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_i64(&self, key: &str) -> RawResult<Option<i64>> {
        self.get_with(key, elem::RawBson::as_i64)
    }

    /// Return a reference to the contained data as a `&[u8]`
    ///
    /// ```
    /// # use bson::raw::RawDocument;
    /// use bson::doc;
    /// let docbuf = RawDocument::from_document(&doc!{});
    /// assert_eq!(docbuf.as_bytes(), b"\x05\x00\x00\x00\x00");
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

impl AsRef<RawDocumentRef> for RawDocumentRef {
    fn as_ref(&self) -> &RawDocumentRef {
        self
    }
}

impl Deref for RawDocument {
    type Target = RawDocumentRef;

    fn deref(&self) -> &Self::Target {
        RawDocumentRef::new_unchecked(&self.data)
    }
}

impl TryFrom<&RawDocumentRef> for crate::Document {
    type Error = RawError;

    fn try_from(rawdoc: &RawDocumentRef) -> RawResult<Document> {
        rawdoc
            .into_iter()
            .map(|res| res.and_then(|(k, v)| Ok((k.to_owned(), v.try_into()?))))
            .collect()
    }
}

impl<'a> IntoIterator for &'a RawDocumentRef {
    type IntoIter = RawDocumentIter<'a>;
    type Item = RawResult<(&'a str, RawBson<'a>)>;

    fn into_iter(self) -> RawDocumentIter<'a> {
        RawDocumentIter {
            doc: self,
            offset: 4,
        }
    }
}

pub struct RawDocumentIter<'a> {
    doc: &'a RawDocumentRef,
    offset: usize,
}

impl<'a> Iterator for RawDocumentIter<'a> {
    type Item = RawResult<(&'a str, elem::RawBson<'a>)>;

    fn next(&mut self) -> Option<RawResult<(&'a str, elem::RawBson<'a>)>> {
        if self.offset == self.doc.data.len() - 1 {
            if self.doc.data[self.offset] == 0 {
                // end of document marker
                return None;
            } else {
                return Some(Err(RawError::MalformedValue {
                    message: "document not null terminated".into(),
                }));
            }
        }

        let key = match read_nullterminated(&self.doc.data[self.offset + 1..]) {
            Ok(key) => key,
            Err(err) => return Some(Err(err)),
        };

        let valueoffset = self.offset + 1 + key.len() + 1; // type specifier + key + \0

        let element_type = match ElementType::from(self.doc.data[self.offset]) {
            Some(et) => et,
            None => {
                return Some(Err(RawError::MalformedValue {
                    message: format!("invalid tag: {}", self.doc.data[self.offset]),
                }))
            }
        };

        let element_size = match element_type {
            ElementType::Double => 8,
            ElementType::String => {
                let size =
                    4 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;

                if self.doc.data[valueoffset + size - 1] != 0 {
                    return Some(Err(RawError::MalformedValue {
                        message: "string not null terminated".into(),
                    }));
                }

                size
            }
            ElementType::EmbeddedDocument => {
                let size = i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;

                if self.doc.data[valueoffset + size - 1] != 0 {
                    return Some(Err(RawError::MalformedValue {
                        message: "document not null terminated".into(),
                    }));
                }

                size
            }
            ElementType::Array => {
                let size = i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;

                if self.doc.data[valueoffset + size - 1] != 0 {
                    return Some(Err(RawError::MalformedValue {
                        message: "array not null terminated".into(),
                    }));
                }

                size
            }
            ElementType::Binary => {
                5 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize
            }
            ElementType::Undefined => 0,
            ElementType::ObjectId => 12,
            ElementType::Boolean => 1,
            ElementType::DateTime => 8,
            ElementType::Null => 0,
            ElementType::RegularExpression => {
                let regex = match read_nullterminated(&self.doc.data[valueoffset..]) {
                    Ok(regex) => regex,
                    Err(err) => return Some(Err(err)),
                };

                let options =
                    match read_nullterminated(&self.doc.data[valueoffset + regex.len() + 1..]) {
                        Ok(options) => options,
                        Err(err) => return Some(Err(err)),
                    };

                regex.len() + options.len() + 2
            }
            ElementType::DbPointer => {
                let string_size =
                    4 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;

                let id_size = 12;

                if self.doc.data[valueoffset + string_size - 1] != 0 {
                    return Some(Err(RawError::MalformedValue {
                        message: "DBPointer string not null-terminated".into(),
                    }));
                }

                string_size + id_size
            }
            ElementType::JavaScriptCode => {
                let size =
                    4 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;

                if self.doc.data[valueoffset + size - 1] != 0 {
                    return Some(Err(RawError::MalformedValue {
                        message: "javascript code not null-terminated".into(),
                    }));
                }

                size
            }
            ElementType::Symbol => {
                4 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize
            }
            ElementType::JavaScriptCodeWithScope => {
                let size = i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;

                if self.doc.data[valueoffset + size - 1] != 0 {
                    return Some(Err(RawError::MalformedValue {
                        message: "javascript with scope not null-terminated".into(),
                    }));
                }

                size
            }
            ElementType::Int32 => 4,
            ElementType::Timestamp => 8,
            ElementType::Int64 => 8,
            ElementType::Decimal128 => 16,
            ElementType::MaxKey => 0,
            ElementType::MinKey => 0,
        };

        let nextoffset = valueoffset + element_size;
        self.offset = nextoffset;

        Some(Ok((
            key,
            elem::RawBson::new(element_type, &self.doc.data[valueoffset..nextoffset]),
        )))
    }
}

/// A BSON array referencing raw bytes stored elsewhere.
pub struct RawArray {
    doc: RawDocumentRef,
}

impl RawArray {
    fn new(data: &[u8]) -> RawResult<&RawArray> {
        Ok(RawArray::from_doc(RawDocumentRef::new(data)?))
    }

    fn from_doc(doc: &RawDocumentRef) -> &RawArray {
        // SAFETY:
        //
        // Dereferencing a raw pointer requires unsafe due to the potential that the pointer is
        // null, dangling, or misaligned. We know the pointer is not null or dangling due to the
        // fact that it's created by a safe reference. Converting &RawDocumentRef to *const
        // RawDocumentRef will be properly aligned due to them being references to the same type,
        // and converting *const RawDocumentRef to *const RawArray is aligned due to the fact that
        // the only field in a RawArray is a RawDocumentRef, meaning the structs are represented
        // identically at the byte level.
        unsafe { &*(doc as *const RawDocumentRef as *const RawArray) }
    }

    /// Gets a reference to the value at the given index.
    pub fn get(&self, index: usize) -> RawResult<Option<RawBson<'_>>> {
        self.into_iter().nth(index).transpose()
    }

    fn get_with<'a, T>(
        &'a self,
        index: usize,
        f: impl FnOnce(elem::RawBson<'a>) -> RawResult<T>,
    ) -> RawResult<Option<T>> {
        self.get(index)?.map(f).transpose()
    }

    /// Gets the BSON double at the given index or returns an error if the value at that index isn't
    /// a double.
    pub fn get_f64(&self, index: usize) -> RawResult<Option<f64>> {
        self.get_with(index, elem::RawBson::as_f64)
    }

    /// Gets a reference to the string at the given index or returns an error if the
    /// value at that index isn't a string.
    pub fn get_str(&self, index: usize) -> RawResult<Option<&str>> {
        self.get_with(index, elem::RawBson::as_str)
    }

    /// Gets a reference to the document at the given index or returns an error if the
    /// value at that index isn't a document.
    pub fn get_document(&self, index: usize) -> RawResult<Option<&RawDocumentRef>> {
        self.get_with(index, elem::RawBson::as_document)
    }

    /// Gets a reference to the array at the given index or returns an error if the
    /// value at that index isn't a array.
    pub fn get_array(&self, index: usize) -> RawResult<Option<&RawArray>> {
        self.get_with(index, elem::RawBson::as_array)
    }

    /// Gets a reference to the BSON binary value at the given index or returns an error if the
    /// value at that index isn't a binary.
    pub fn get_binary(&self, index: usize) -> RawResult<Option<RawBinary<'_>>> {
        self.get_with(index, elem::RawBson::as_binary)
    }

    /// Gets the ObjectId at the given index or returns an error if the value at that index isn't an
    /// ObjectId.
    pub fn get_object_id(&self, index: usize) -> RawResult<Option<ObjectId>> {
        self.get_with(index, elem::RawBson::as_object_id)
    }

    /// Gets the boolean at the given index or returns an error if the value at that index isn't a
    /// boolean.
    pub fn get_bool(&self, index: usize) -> RawResult<Option<bool>> {
        self.get_with(index, elem::RawBson::as_bool)
    }

    /// Gets the DateTime at the given index or returns an error if the value at that index isn't a
    /// DateTime.
    pub fn get_datetime(&self, index: usize) -> RawResult<Option<DateTime<Utc>>> {
        self.get_with(index, elem::RawBson::as_datetime)
    }

    /// Gets a reference to the BSON regex at the given index or returns an error if the
    /// value at that index isn't a regex.
    pub fn get_regex(&self, index: usize) -> RawResult<Option<RawRegex<'_>>> {
        self.get_with(index, elem::RawBson::as_regex)
    }

    /// Gets a reference to the BSON timestamp at the given index or returns an error if the
    /// value at that index isn't a timestamp.
    pub fn get_timestamp(&self, index: usize) -> RawResult<Option<RawTimestamp<'_>>> {
        self.get_with(index, elem::RawBson::as_timestamp)
    }

    /// Gets the BSON int32 at the given index or returns an error if the value at that index isn't
    /// a 32-bit integer.
    pub fn get_i32(&self, index: usize) -> RawResult<Option<i32>> {
        self.get_with(index, elem::RawBson::as_i32)
    }

    /// Gets BSON int64 at the given index or returns an error if the value at that index isn't a
    /// 64-bit integer.
    pub fn get_i64(&self, index: usize) -> RawResult<Option<i64>> {
        self.get_with(index, elem::RawBson::as_i64)
    }

    /// Gets a reference to the raw bytes of the RawArray.
    pub fn as_bytes(&self) -> &[u8] {
        self.doc.as_bytes()
    }
}

impl TryFrom<&RawArray> for Vec<Bson> {
    type Error = RawError;

    fn try_from(arr: &RawArray) -> RawResult<Vec<Bson>> {
        arr.into_iter()
            .map(|result| {
                let rawbson = result?;
                Bson::try_from(rawbson)
            })
            .collect()
    }
}

impl<'a> IntoIterator for &'a RawArray {
    type IntoIter = RawArrayIter<'a>;
    type Item = RawResult<elem::RawBson<'a>>;

    fn into_iter(self) -> RawArrayIter<'a> {
        RawArrayIter {
            inner: self.doc.into_iter(),
        }
    }
}

pub struct RawArrayIter<'a> {
    inner: RawDocumentIter<'a>,
}

impl<'a> Iterator for RawArrayIter<'a> {
    type Item = RawResult<elem::RawBson<'a>>;

    fn next(&mut self) -> Option<RawResult<RawBson<'a>>> {
        match self.inner.next() {
            Some(Ok((_, v))) => Some(Ok(v)),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}
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

fn read_nullterminated(buf: &[u8]) -> RawResult<&str> {
    let mut splits = buf.splitn(2, |x| *x == 0);
    let value = splits.next().ok_or_else(|| RawError::MalformedValue {
        message: "no value".into(),
    })?;
    if splits.next().is_some() {
        Ok(try_to_str(value)?)
    } else {
        Err(RawError::MalformedValue {
            message: "expected null terminator".into(),
        })
    }
}

fn read_lenencoded(buf: &[u8]) -> RawResult<&str> {
    let length = i32_from_slice(&buf[..4]);
    assert!(buf.len() as i32 >= length + 4);
    try_to_str(&buf[4..4 + length as usize - 1])
}

fn try_to_str(data: &[u8]) -> RawResult<&str> {
    match std::str::from_utf8(data) {
        Ok(s) => Ok(s),
        Err(_) => Err(RawError::Utf8EncodingError(data.into())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        doc,
        spec::BinarySubtype,
        Binary,
        Bson,
        JavaScriptCodeWithScope,
        Regex,
        Timestamp,
    };
    use chrono::TimeZone;

    fn to_bytes(doc: &crate::Document) -> Vec<u8> {
        let mut docbytes = Vec::new();
        doc.to_writer(&mut docbytes).unwrap();
        docbytes
    }

    #[test]
    fn string_from_document() {
        let docbytes = to_bytes(&doc! {
            "this": "first",
            "that": "second",
            "something": "else",
        });
        let rawdoc = RawDocumentRef::new(&docbytes).unwrap();
        assert_eq!(
            rawdoc.get("that").unwrap().unwrap().as_str().unwrap(),
            "second",
        );
    }

    #[test]
    fn nested_document() {
        let docbytes = to_bytes(&doc! {
            "outer": {
                "inner": "surprise",
            },
        });
        let rawdoc = RawDocumentRef::new(&docbytes).unwrap();
        assert_eq!(
            rawdoc
                .get("outer")
                .expect("get doc result")
                .expect("get doc option")
                .as_document()
                .expect("as doc")
                .get("inner")
                .expect("get str result")
                .expect("get str option")
                .as_str()
                .expect("as str"),
            "surprise",
        );
    }

    #[test]
    fn iterate() {
        let docbytes = to_bytes(&doc! {
            "apples": "oranges",
            "peanut butter": "chocolate",
            "easy as": {"do": 1, "re": 2, "mi": 3},
        });
        let rawdoc = RawDocumentRef::new(&docbytes).expect("malformed bson document");
        let mut dociter = rawdoc.into_iter();
        let next = dociter.next().expect("no result").expect("invalid bson");
        assert_eq!(next.0, "apples");
        assert_eq!(next.1.as_str().expect("result was not a str"), "oranges");
        let next = dociter.next().expect("no result").expect("invalid bson");
        assert_eq!(next.0, "peanut butter");
        assert_eq!(next.1.as_str().expect("result was not a str"), "chocolate");
        let next = dociter.next().expect("no result").expect("invalid bson");
        assert_eq!(next.0, "easy as");
        let _doc = next.1.as_document().expect("result was a not a document");
        let next = dociter.next();
        assert!(next.is_none());
    }

    #[test]
    fn rawdoc_to_doc() {
        let docbytes = to_bytes(&doc! {
            "f64": 2.5,
            "string": "hello",
            "document": {},
            "array": ["binary", "serialized", "object", "notation"],
            "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1, 2, 3] },
            "object_id": ObjectId::with_bytes([1, 2, 3, 4, 5,6,7,8,9,10, 11,12]),
            "boolean": true,
            "datetime": Utc::now(),
            "null": Bson::Null,
            "regex": Bson::RegularExpression(Regex { pattern: String::from(r"end\s*$"), options: String::from("i")}),
            "javascript": Bson::JavaScriptCode(String::from("console.log(console);")),
            "symbol": Bson::Symbol(String::from("artist-formerly-known-as")),
            "javascript_with_scope": Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope{ code: String::from("console.log(msg);"), scope: doc!{"ok": true}}),
            "int32": 23i32,
            "timestamp": Bson::Timestamp(Timestamp { time: 3542578, increment: 0 }),
            "int64": 46i64,
            "end": "END",
        });

        let rawdoc = RawDocumentRef::new(&docbytes).expect("invalid document");
        let _doc: crate::Document = rawdoc.try_into().expect("invalid bson");
    }

    #[test]
    fn f64() {
        #![allow(clippy::float_cmp)]

        let rawdoc = RawDocument::from_document(&doc! {"f64": 2.5});
        assert_eq!(
            rawdoc
                .get("f64")
                .expect("error finding key f64")
                .expect("no key f64")
                .as_f64()
                .expect("result was not a f64"),
            2.5,
        );
    }

    #[test]
    fn string() {
        let rawdoc = RawDocument::from_document(&doc! {"string": "hello"});

        assert_eq!(
            rawdoc
                .get("string")
                .expect("error finding key string")
                .expect("no key string")
                .as_str()
                .expect("result was not a string"),
            "hello",
        );
    }
    #[test]
    fn document() {
        let rawdoc = RawDocument::from_document(&doc! {"document": {}});

        let doc = rawdoc
            .get("document")
            .expect("error finding key document")
            .expect("no key document")
            .as_document()
            .expect("result was not a document");
        assert_eq!(&doc.data, [5, 0, 0, 0, 0].as_ref()); // Empty document
    }

    #[test]
    fn array() {
        let rawdoc = RawDocument::from_document(
            &doc! { "array": ["binary", "serialized", "object", "notation"]},
        );

        let array = rawdoc
            .get("array")
            .expect("error finding key array")
            .expect("no key array")
            .as_array()
            .expect("result was not an array");
        assert_eq!(array.get_str(0), Ok(Some("binary")));
        assert_eq!(array.get_str(3), Ok(Some("notation")));
        assert_eq!(array.get_str(4), Ok(None));
    }

    #[test]
    fn binary() {
        let rawdoc = RawDocument::from_document(&doc! {
            "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] }
        });
        let binary: elem::RawBinary<'_> = rawdoc
            .get("binary")
            .expect("error finding key binary")
            .expect("no key binary")
            .as_binary()
            .expect("result was not a binary object");
        assert_eq!(binary.subtype, BinarySubtype::Generic);
        assert_eq!(binary.data, &[1, 2, 3]);
    }

    #[test]
    fn object_id() {
        let rawdoc = RawDocument::from_document(&doc! {
            "object_id": ObjectId::with_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
        });
        let oid = rawdoc
            .get("object_id")
            .expect("error finding key object_id")
            .expect("no key object_id")
            .as_object_id()
            .expect("result was not an object id");
        assert_eq!(oid.to_hex(), "0102030405060708090a0b0c");
    }

    #[test]
    fn boolean() {
        let rawdoc = RawDocument::from_document(&doc! {
            "boolean": true,
        });

        let boolean = rawdoc
            .get("boolean")
            .expect("error finding key boolean")
            .expect("no key boolean")
            .as_bool()
            .expect("result was not boolean");

        assert_eq!(boolean, true);
    }

    #[test]
    fn datetime() {
        let rawdoc = RawDocument::from_document(&doc! {
            "boolean": true,
            "datetime": Utc.ymd(2000,10,31).and_hms(12, 30, 45),
        });
        let datetime = rawdoc
            .get("datetime")
            .expect("error finding key datetime")
            .expect("no key datetime")
            .as_datetime()
            .expect("result was not datetime");
        assert_eq!(datetime.to_rfc3339(), "2000-10-31T12:30:45+00:00");
    }

    #[test]
    fn null() {
        let rawdoc = RawDocument::from_document(&doc! {
            "null": null,
        });
        let () = rawdoc
            .get("null")
            .expect("error finding key null")
            .expect("no key null")
            .as_null()
            .expect("was not null");
    }

    #[test]
    fn regex() {
        let rawdoc = RawDocument::from_document(&doc! {
            "regex": Bson::RegularExpression(Regex { pattern: String::from(r"end\s*$"), options: String::from("i")}),
        });
        let regex = rawdoc
            .get("regex")
            .expect("error finding key regex")
            .expect("no key regex")
            .as_regex()
            .expect("was not regex");
        assert_eq!(regex.pattern, r"end\s*$");
        assert_eq!(regex.options, "i");
    }
    #[test]
    fn javascript() {
        let rawdoc = RawDocument::from_document(&doc! {
            "javascript": Bson::JavaScriptCode(String::from("console.log(console);")),
        });
        let js = rawdoc
            .get("javascript")
            .expect("error finding key javascript")
            .expect("no key javascript")
            .as_javascript()
            .expect("was not javascript");
        assert_eq!(js, "console.log(console);");
    }

    #[test]
    fn symbol() {
        let rawdoc = RawDocument::from_document(&doc! {
            "symbol": Bson::Symbol(String::from("artist-formerly-known-as")),
        });

        let symbol = rawdoc
            .get("symbol")
            .expect("error finding key symbol")
            .expect("no key symbol")
            .as_symbol()
            .expect("was not symbol");
        assert_eq!(symbol, "artist-formerly-known-as");
    }

    #[test]
    fn javascript_with_scope() {
        let rawdoc = RawDocument::from_document(&doc! {
            "javascript_with_scope": Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope{ code: String::from("console.log(msg);"), scope: doc!{"ok": true}}),
        });
        let (js, scopedoc) = rawdoc
            .get("javascript_with_scope")
            .expect("error finding key javascript_with_scope")
            .expect("no key javascript_with_scope")
            .as_javascript_with_scope()
            .expect("was not javascript with scope");
        assert_eq!(js, "console.log(msg);");
        let (scope_key, scope_value_bson) = scopedoc
            .into_iter()
            .next()
            .expect("no next value in scope")
            .expect("invalid element");
        assert_eq!(scope_key, "ok");
        let scope_value = scope_value_bson.as_bool().expect("not a boolean");
        assert_eq!(scope_value, true);
    }

    #[test]
    fn int32() {
        let rawdoc = RawDocument::from_document(&doc! {
            "int32": 23i32,
        });
        let int32 = rawdoc
            .get("int32")
            .expect("error finding key int32")
            .expect("no key int32")
            .as_i32()
            .expect("was not int32");
        assert_eq!(int32, 23i32);
    }

    #[test]
    fn timestamp() {
        let rawdoc = RawDocument::from_document(&doc! {
            "timestamp": Bson::Timestamp(Timestamp { time: 3542578, increment: 7 }),
        });
        let ts = rawdoc
            .get("timestamp")
            .expect("error finding key timestamp")
            .expect("no key timestamp")
            .as_timestamp()
            .expect("was not a timestamp");

        assert_eq!(ts.increment(), 7);
        assert_eq!(ts.time(), 3542578);
    }

    #[test]
    fn int64() {
        let rawdoc = RawDocument::from_document(&doc! {
            "int64": 46i64,
        });
        let int64 = rawdoc
            .get("int64")
            .expect("error finding key int64")
            .expect("no key int64")
            .as_i64()
            .expect("was not int64");
        assert_eq!(int64, 46i64);
    }
    #[test]
    fn document_iteration() {
        let docbytes = to_bytes(&doc! {
            "f64": 2.5,
            "string": "hello",
            "document": {},
            "array": ["binary", "serialized", "object", "notation"],
            "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] },
            "object_id": ObjectId::with_bytes([1, 2, 3, 4, 5,6,7,8,9,10, 11,12]),
            "boolean": true,
            "datetime": Utc::now(),
            "null": Bson::Null,
            "regex": Bson::RegularExpression(Regex { pattern: String::from(r"end\s*$"), options: String::from("i")}),
            "javascript": Bson::JavaScriptCode(String::from("console.log(console);")),
            "symbol": Bson::Symbol(String::from("artist-formerly-known-as")),
            "javascript_with_scope": Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope{ code: String::from("console.log(msg);"), scope: doc!{"ok": true}}),
            "int32": 23i32,
            "timestamp": Bson::Timestamp(Timestamp { time: 3542578, increment: 0 }),
            "int64": 46i64,
            "end": "END",
        });
        let rawdoc = unsafe { RawDocumentRef::new_unchecked(&docbytes) };

        assert_eq!(
            rawdoc
                .into_iter()
                .collect::<Result<Vec<(&str, _)>, RawError>>()
                .expect("collecting iterated doc")
                .len(),
            17
        );
        let end = rawdoc
            .get("end")
            .expect("error finding key end")
            .expect("no key end")
            .as_str()
            .expect("was not str");
        assert_eq!(end, "END");
    }

    #[test]
    fn into_bson_conversion() {
        let docbytes = to_bytes(&doc! {
            "f64": 2.5,
            "string": "hello",
            "document": {},
            "array": ["binary", "serialized", "object", "notation"],
            "object_id": ObjectId::with_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
            "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] },
            "boolean": false,
        });
        let rawbson = elem::RawBson::new(ElementType::EmbeddedDocument, &docbytes);
        let b: Bson = rawbson.try_into().expect("invalid bson");
        let doc = b.as_document().expect("not a document");
        assert_eq!(*doc.get("f64").expect("f64 not found"), Bson::Double(2.5));
        assert_eq!(
            *doc.get("string").expect("string not found"),
            Bson::String(String::from("hello"))
        );
        assert_eq!(
            *doc.get("document").expect("document not found"),
            Bson::Document(doc! {})
        );
        assert_eq!(
            *doc.get("array").expect("array not found"),
            Bson::Array(
                vec!["binary", "serialized", "object", "notation"]
                    .into_iter()
                    .map(|s| Bson::String(String::from(s)))
                    .collect()
            )
        );
        assert_eq!(
            *doc.get("object_id").expect("object_id not found"),
            Bson::ObjectId(ObjectId::with_bytes([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12
            ]))
        );
        assert_eq!(
            *doc.get("binary").expect("binary not found"),
            Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                bytes: vec![1, 2, 3]
            })
        );
        assert_eq!(
            *doc.get("boolean").expect("boolean not found"),
            Bson::Boolean(false)
        );
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;
    use std::convert::TryInto;

    use super::{props::arbitrary_bson, RawDocument};
    use crate::doc;

    fn to_bytes(doc: &crate::Document) -> Vec<u8> {
        let mut docbytes = Vec::new();
        doc.to_writer(&mut docbytes).unwrap();
        docbytes
    }

    proptest! {
        #[test]
        fn no_crashes(s: Vec<u8>) {
            let _ = RawDocument::new(s);
        }

        #[test]
        fn roundtrip_bson(bson in arbitrary_bson()) {
            println!("{:?}", bson);
            let doc = doc!{"bson": bson};
            let raw = to_bytes(&doc);
            let raw = RawDocument::new(raw);
            prop_assert!(raw.is_ok());
            let raw = raw.unwrap();
            let roundtrip: Result<crate::Document, _> = raw.try_into();
            prop_assert!(roundtrip.is_ok());
            let roundtrip = roundtrip.unwrap();
            prop_assert_eq!(doc, roundtrip);
        }
    }
}
