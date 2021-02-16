use std::{
    borrow::Borrow,
    convert::{TryFrom, TryInto},
    ops::Deref,
};

use chrono::{DateTime, Utc};

use super::{
    i32_from_slice,
    read_nullterminated,
    Error,
    RawArray,
    RawBinary,
    RawBson,
    RawRegex,
    RawTimestamp,
    Result,
};
#[cfg(feature = "decimal128")]
use crate::decimal128::Decimal128;
use crate::{oid::ObjectId, spec::ElementType, Document};

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
/// Individual elements can be accessed using [`RawDocument::get`](RawDocument::get) or any of the
/// type-specific getters, such as [`RawDocument::get_object_id`](RawDocument::get_object_id) or
/// [`RawDocument::get_str`](RawDocument::get_str). Note that accessing elements is an O(N)
/// operation, as it requires iterating through the document from the beginning to find the
/// requested key.
///
/// ```
/// # use bson::raw::{RawDocument, Error};
/// let doc = RawDocument::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
/// assert_eq!(doc.get_str("hi")?, Some("y'all"));
/// # Ok::<(), Error>(())
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
    /// the RawDocument will return Errors where appropriate.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, Error};
    /// let doc = RawDocument::new(b"\x05\0\0\0\0".to_vec())?;
    /// # Ok::<(), Error>(())
    /// ```
    pub fn new(data: Vec<u8>) -> Result<RawDocument> {
        if data.len() < 5 {
            return Err(Error::MalformedValue {
                message: "document too short".into(),
            });
        }

        let length = i32_from_slice(&data[..4]);

        if data.len() as i32 != length {
            return Err(Error::MalformedValue {
                message: "document length incorrect".into(),
            });
        }

        if data[data.len() - 1] != 0 {
            return Err(Error::MalformedValue {
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
    /// # use bson::raw::{RawDocument, Error};
    /// use bson::{doc, oid::ObjectId};
    ///
    /// let document = doc! {
    ///     "_id": ObjectId::new(),
    ///     "name": "Herman Melville",
    ///     "title": "Moby-Dick",
    /// };
    /// let doc = RawDocument::from_document(&document);
    /// # Ok::<(), Error>(())
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
    /// # use bson::raw::{elem, RawDocument, Error};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! { "ferris": true });
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
    type Error = Error;

    fn try_from(raw: RawDocument) -> Result<Document> {
        Document::try_from(raw.as_ref())
    }
}

impl<'a> IntoIterator for &'a RawDocument {
    type IntoIter = RawDocumentIter<'a>;
    type Item = Result<(&'a str, RawBson<'a>)>;

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
/// # use bson::raw::{Doc, Error};
/// let doc = RawDocumentRef::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00")?;
/// let mut iter = doc.into_iter();
/// let (key, value) = iter.next().unwrap()?;
/// assert_eq!(key, "hi");
/// assert_eq!(value.as_str(), Ok("y'all"));
/// assert!(iter.next().is_none());
/// # Ok::<(), Error>(())
/// ```
///
/// Individual elements can be accessed using [`RawDocumentRef::get`](RawDocumentRef::get) or any of
/// the type-specific getters, such as
/// [`RawDocumentRef::get_object_id`](RawDocumentRef::get_object_id) or [`RawDocumentRef::
/// get_str`](RawDocumentRef::get_str). Note that accessing elements is an O(N) operation, as it
/// requires iterating through the document from the beginning to find the requested key.
///
/// ```
/// # use bson::raw::{RawDocument, Error};
/// let doc = RawDocument::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
/// assert_eq!(doc.get_str("hi")?, Some("y'all"));
/// # Ok::<(), Error>(())
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
    /// the RawDocument will return Errors where appropriate.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, Error};
    /// let doc = RawDocumentRef::new(b"\x05\0\0\0\0")?;
    /// # Ok::<(), Error>(())
    /// ```
    pub fn new<D: AsRef<[u8]> + ?Sized>(data: &D) -> Result<&RawDocumentRef> {
        let data = data.as_ref();

        if data.len() < 5 {
            return Err(Error::MalformedValue {
                message: "document too short".into(),
            });
        }

        let length = i32_from_slice(&data[..4]);

        if data.len() as i32 != length {
            return Err(Error::MalformedValue {
                message: "document length incorrect".into(),
            });
        }

        if data[data.len() - 1] != 0 {
            return Err(Error::MalformedValue {
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
    /// # use bson::raw::{Doc, Error};
    /// use bson::raw::RawDocument;
    ///
    /// let data = b"\x05\0\0\0\0";
    /// let doc_ref = RawDocumentRef::new(data)?;
    /// let doc: RawDocument = doc_ref.to_raw_document();
    /// # Ok::<(), Error>(())
    pub fn to_raw_document(&self) -> RawDocument {
        RawDocument {
            data: self.data.to_owned().into_boxed_slice(),
        }
    }

    /// Gets a reference to the value corresponding to the given key by iterating until the key is
    /// found.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem::Element, Error};
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
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get<'a>(&'a self, key: &str) -> Result<Option<RawBson<'a>>> {
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
        f: impl FnOnce(RawBson<'a>) -> Result<T>,
    ) -> Result<Option<T>> {
        self.get(key)?.map(f).transpose()
    }

    /// Gets a reference to the BSON double value corresponding to a given key or returns an error
    /// if the key corresponds to a value which isn't a double.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem::Element, Error};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "bool": true,
    ///     "f64": 2.5,
    /// });
    ///
    /// assert_eq!(doc.get_f64("f64"), Ok(Some(2.5)));
    /// assert_eq!(doc.get_f64("bool"), Err(Error::UnexpectedType));
    /// assert_eq!(doc.get_f64("unknown"), Ok(None));
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get_f64(&self, key: &str) -> Result<Option<f64>> {
        self.get_with(key, RawBson::as_f64)
    }

    /// Gets a reference to the string value corresponding to a given key or returns an error if the
    /// key corresponds to a value which isn't a string.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem::Element, Error};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "string": "hello",
    ///     "bool": true,
    /// });
    ///
    /// assert_eq!(doc.get_str("string"), Ok(Some("hello")));
    /// assert_eq!(doc.get_str("bool"), Err(Error::UnexpectedType));
    /// assert_eq!(doc.get_str("unknown"), Ok(None));
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get_str<'a>(&'a self, key: &str) -> Result<Option<&'a str>> {
        self.get_with(key, RawBson::as_str)
    }

    /// Gets a reference to the document value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a document.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem::Element, Error};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "doc": { "key": "value"},
    ///     "bool": true,
    /// });
    ///
    /// assert_eq!(doc.get_document("doc")?.expect("finding key doc").get_str("key"), Ok(Some("value")));
    /// assert_eq!(doc.get_document("bool").unwrap_err(), Error::UnexpectedType);
    /// assert!(doc.get_document("unknown")?.is_none());
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get_document<'a>(&'a self, key: &str) -> Result<Option<&'a RawDocumentRef>> {
        self.get_with(key, RawBson::as_document)
    }

    /// Gets a reference to the array value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't an array.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem::Element, Error};
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
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get_array<'a>(&'a self, key: &str) -> Result<Option<&'a RawArray>> {
        self.get_with(key, RawBson::as_array)
    }

    /// Gets a reference to the BSON binary value corresponding to a given key or returns an error
    /// if the key corresponds to a value which isn't a binary value.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem, Error};
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
    /// assert_eq!(doc.get_binary("bool").unwrap_err(), Error::UnexpectedType);
    /// assert!(doc.get_binary("unknown")?.is_none());
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get_binary<'a>(&'a self, key: &str) -> Result<Option<RawBinary<'a>>> {
        self.get_with(key, RawBson::as_binary)
    }

    /// Gets a reference to the ObjectId value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't an ObjectId.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, Error};
    /// use bson::{doc, oid::ObjectId};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "_id": ObjectId::new(),
    ///     "bool": true,
    /// });
    ///
    /// let oid = doc.get_object_id("_id")?.unwrap();
    /// assert_eq!(doc.get_object_id("bool").unwrap_err(), Error::UnexpectedType);
    /// assert!(doc.get_object_id("unknown")?.is_none());
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get_object_id(&self, key: &str) -> Result<Option<ObjectId>> {
        self.get_with(key, RawBson::as_object_id)
    }

    /// Gets a reference to the boolean value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a boolean.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, Error};
    /// use bson::{doc, oid::ObjectId};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "_id": ObjectId::new(),
    ///     "bool": true,
    /// });
    ///
    /// assert!(doc.get_bool("bool")?.unwrap());
    /// assert_eq!(doc.get_bool("_id").unwrap_err(), Error::UnexpectedType);
    /// assert!(doc.get_object_id("unknown")?.is_none());
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get_bool(&self, key: &str) -> Result<Option<bool>> {
        self.get_with(key, RawBson::as_bool)
    }

    /// Gets a reference to the BSON DateTime value corresponding to a given key or returns an error
    /// if the key corresponds to a value which isn't a DateTime.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, Error};
    /// use bson::doc;
    /// use chrono::{Utc, Datelike, TimeZone};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "created_at": Utc.ymd(2020, 3, 15).and_hms(17, 0, 0),
    ///     "bool": true,
    /// });
    /// assert_eq!(doc.get_datetime("created_at")?.unwrap().year(), 2020);
    /// assert_eq!(doc.get_datetime("bool").unwrap_err(), Error::UnexpectedType);
    /// assert!(doc.get_datetime("unknown")?.is_none());
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get_datetime(&self, key: &str) -> Result<Option<DateTime<Utc>>> {
        self.get_with(key, RawBson::as_datetime)
    }
    /// Gets a reference to the BSON regex value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a regex.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, Error, elem};
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
    /// assert_eq!(doc.get_regex("bool").unwrap_err(), Error::UnexpectedType);
    /// assert!(doc.get_regex("unknown")?.is_none());
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get_regex<'a>(&'a self, key: &str) -> Result<Option<RawRegex<'a>>> {
        self.get_with(key, RawBson::as_regex)
    }

    /// Gets a reference to the BSON timestamp value corresponding to a given key or returns an
    /// error if the key corresponds to a value which isn't a timestamp.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem, Error};
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
    /// assert_eq!(doc.get_timestamp("bool"), Err(Error::UnexpectedType));
    /// assert_eq!(doc.get_timestamp("unknown"), Ok(None));
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get_timestamp<'a>(&'a self, key: &str) -> Result<Option<RawTimestamp<'a>>> {
        self.get_with(key, RawBson::as_timestamp)
    }

    /// Gets a reference to the BSON int32 value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a 32-bit integer.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, Error};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "bool": true,
    ///     "i32": 1_000_000,
    /// });
    ///
    /// assert_eq!(doc.get_i32("i32"), Ok(Some(1_000_000)));
    /// assert_eq!(doc.get_i32("bool"), Err(Error::UnexpectedType));
    /// assert_eq!(doc.get_i32("unknown"), Ok(None));
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get_i32(&self, key: &str) -> Result<Option<i32>> {
        self.get_with(key, RawBson::as_i32)
    }

    /// Gets a reference to the BSON int64 value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a 64-bit integer.
    ///
    /// ```
    /// # use bson::raw::{RawDocument, elem::Element, Error};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "bool": true,
    ///     "i64": 9223372036854775807_i64,
    /// });
    ///
    /// assert_eq!(doc.get_i64("i64"), Ok(Some(9223372036854775807)));
    /// assert_eq!(doc.get_i64("bool"), Err(Error::UnexpectedType));
    /// assert_eq!(doc.get_i64("unknown"), Ok(None));
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get_i64(&self, key: &str) -> Result<Option<i64>> {
        self.get_with(key, RawBson::as_i64)
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
    type Error = Error;

    fn try_from(rawdoc: &RawDocumentRef) -> Result<Document> {
        rawdoc
            .into_iter()
            .map(|res| res.and_then(|(k, v)| Ok((k.to_owned(), v.try_into()?))))
            .collect()
    }
}

impl<'a> IntoIterator for &'a RawDocumentRef {
    type IntoIter = RawDocumentIter<'a>;
    type Item = Result<(&'a str, RawBson<'a>)>;

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
    type Item = Result<(&'a str, RawBson<'a>)>;

    fn next(&mut self) -> Option<Result<(&'a str, RawBson<'a>)>> {
        if self.offset == self.doc.data.len() - 1 {
            if self.doc.data[self.offset] == 0 {
                // end of document marker
                return None;
            } else {
                return Some(Err(Error::MalformedValue {
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
                return Some(Err(Error::MalformedValue {
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
                    return Some(Err(Error::MalformedValue {
                        message: "string not null terminated".into(),
                    }));
                }

                size
            }
            ElementType::EmbeddedDocument => {
                let size = i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;

                if self.doc.data[valueoffset + size - 1] != 0 {
                    return Some(Err(Error::MalformedValue {
                        message: "document not null terminated".into(),
                    }));
                }

                size
            }
            ElementType::Array => {
                let size = i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;

                if self.doc.data[valueoffset + size - 1] != 0 {
                    return Some(Err(Error::MalformedValue {
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
                    return Some(Err(Error::MalformedValue {
                        message: "DBPointer string not null-terminated".into(),
                    }));
                }

                string_size + id_size
            }
            ElementType::JavaScriptCode => {
                let size =
                    4 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;

                if self.doc.data[valueoffset + size - 1] != 0 {
                    return Some(Err(Error::MalformedValue {
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
                    return Some(Err(Error::MalformedValue {
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
            RawBson::new(element_type, &self.doc.data[valueoffset..nextoffset]),
        )))
    }
}
