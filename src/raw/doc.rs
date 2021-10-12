use std::{
    borrow::{Borrow, Cow},
    convert::{TryFrom, TryInto},
    ops::{Deref, Range},
};

use crate::{
    de::read_bool,
    raw::{
        error::{try_with_key, ErrorKind},
        f64_from_slice,
        i64_from_slice,
        RawJavaScriptCodeWithScope,
    },
    spec::BinarySubtype,
    DateTime,
    Decimal128,
    Timestamp,
};

use super::{
    i32_from_slice,
    read_lenencoded,
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
        if data.len() < 5 {
            return Err(Error::new_without_key(ErrorKind::MalformedValue {
                message: "document too short".into(),
            }));
        }

        let length = i32_from_slice(&data)?;

        if data.len() as i32 != length {
            return Err(Error::new_without_key(ErrorKind::MalformedValue {
                message: "document length incorrect".into(),
            }));
        }

        if data[data.len() - 1] != 0 {
            return Err(Error::new_without_key(ErrorKind::MalformedValue {
                message: "document not null-terminated".into(),
            }));
        }

        Ok(Self { data })
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
    /// There is no owning iterator for RawDocument.  If you need ownership over
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
        self.data.to_vec()
    }
}

impl<'a> From<RawDocument> for Cow<'a, RawDocumentRef> {
    fn from(rd: RawDocument) -> Self {
        Cow::Owned(rd)
    }
}

impl<'a> From<&'a RawDocument> for Cow<'a, RawDocumentRef> {
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
        Iter {
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
/// # use bson::raw::{Error};
/// use bson::raw::RawDocumentRef;
///
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
#[repr(transparent)]
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
    /// use bson::raw::RawDocumentRef;
    ///
    /// let doc = RawDocumentRef::new(b"\x05\0\0\0\0")?;
    /// # Ok::<(), bson::raw::Error>(())
    /// ```
    pub fn new<D: AsRef<[u8]> + ?Sized>(data: &D) -> Result<&RawDocumentRef> {
        let data = data.as_ref();

        if data.len() < 5 {
            return Err(Error {
                key: None,
                kind: ErrorKind::MalformedValue {
                    message: "document too short".into(),
                },
            });
        }

        let length = i32_from_slice(&data)?;
        println!("got length {}", length);

        if data.len() as i32 != length {
            return Err(Error {
                key: None,
                kind: ErrorKind::MalformedValue {
                    message: "document length incorrect".into(),
                },
            });
        }

        if data[data.len() - 1] != 0 {
            return Err(Error {
                key: None,
                kind: ErrorKind::MalformedValue {
                    message: "document not null-terminated".into(),
                },
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
    /// use bson::raw::{RawDocumentRef, RawDocument, Error};
    ///
    /// let data = b"\x05\0\0\0\0";
    /// let doc_ref = RawDocumentRef::new(data)?;
    /// let doc: RawDocument = doc_ref.to_raw_document();
    /// # Ok::<(), Error>(())
    pub fn to_raw_document(&self) -> RawDocument {
        RawDocument {
            data: self.data.to_owned(),
        }
    }

    /// Gets a reference to the value corresponding to the given key by iterating until the key is
    /// found.
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, oid::ObjectId, raw::{RawDocument, RawBson}};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "_id": ObjectId::new(),
    ///     "f64": 2.5,
    /// })?;
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

    // fn get_with<'a, T>(
    //     &'a self,
    //     key: &str,
    //     f: impl FnOnce(RawBson<'a>) -> Result<T>,
    // ) -> Result<Option<T>> {
    //     self.get(key)?.map(f).transpose()
    // }

    // /// Gets a reference to the BSON double value corresponding to a given key or returns an
    // error /// if the key corresponds to a value which isn't a double.
    // ///
    // /// ```
    // /// # use bson::raw::Error;
    // /// use bson::raw::{ErrorKind, RawDocument};
    // /// use bson::doc;
    // ///
    // /// let doc = RawDocument::from_document(&doc! {
    // ///     "bool": true,
    // ///     "f64": 2.5,
    // /// })?;
    // ///
    // /// assert_eq!(doc.get_f64("f64"), Ok(Some(2.5)));
    // /// assert!(matches!(doc.get_f64("bool").unwrap_err().kind, ErrorKind::UnexpectedType { ..
    // })); /// assert_eq!(doc.get_f64("unknown"), Ok(None));
    // /// # Ok::<(), Error>(())
    // /// ```
    // pub fn get_f64(&self, key: &str) -> Result<Option<f64>> {
    //     self.get_with(key, RawBson::as_f64)
    // }

    // /// Gets a reference to the string value corresponding to a given key or returns an error if
    // the /// key corresponds to a value which isn't a string.
    // ///
    // /// ```
    // /// # use bson::raw::Error;
    // /// use bson::{doc, raw::{RawDocument, ErrorKind}};
    // ///
    // /// let doc = RawDocument::from_document(&doc! {
    // ///     "string": "hello",
    // ///     "bool": true,
    // /// })?;
    // ///
    // /// assert_eq!(doc.get_str("string"), Ok(Some("hello")));
    // /// assert!(matches!(doc.get_str("bool").unwrap_err().kind, ErrorKind::UnexpectedType { ..
    // })); /// assert_eq!(doc.get_str("unknown"), Ok(None));
    // /// # Ok::<(), Error>(())
    // /// ```
    // pub fn get_str<'a>(&'a self, key: &str) -> Result<Option<&'a str>> {
    //     self.get_with(key, RawBson::as_str)
    // }

    // /// Gets a reference to the document value corresponding to a given key or returns an error
    // if /// the key corresponds to a value which isn't a document.
    // ///
    // /// ```
    // /// # use bson::raw::Error;
    // /// use bson::{doc, raw::{ErrorKind, RawDocument}};
    // ///
    // /// let doc = RawDocument::from_document(&doc! {
    // ///     "doc": { "key": "value"},
    // ///     "bool": true,
    // /// })?;
    // ///
    // /// assert_eq!(doc.get_document("doc")?.expect("finding key doc").get_str("key"),
    // Ok(Some("value"))); /// assert!(matches!(doc.get_document("bool").unwrap_err().kind,
    // ErrorKind::UnexpectedType { .. })); /// assert!(doc.get_document("unknown")?.is_none());
    // /// # Ok::<(), Error>(())
    // /// ```
    // pub fn get_document<'a>(&'a self, key: &str) -> Result<Option<&'a RawDocumentRef>> {
    //     self.get_with(key, RawBson::as_document)
    // }

    // /// Gets a reference to the array value corresponding to a given key or returns an error if
    // /// the key corresponds to a value which isn't an array.
    // ///
    // /// ```
    // /// # use bson::raw::Error;
    // /// use bson::{doc, raw::RawDocument};
    // ///
    // /// let doc = RawDocument::from_document(&doc! {
    // ///     "array": [true, 3],
    // ///     "bool": true,
    // /// })?;
    // ///
    // /// let mut arr_iter = doc.get_array("array")?.expect("finding key array").into_iter();
    // /// let _: bool = arr_iter.next().unwrap()?.as_bool()?;
    // /// let _: i32 = arr_iter.next().unwrap()?.as_i32()?;
    // ///
    // /// assert!(arr_iter.next().is_none());
    // /// assert!(doc.get_array("bool").is_err());
    // /// assert!(doc.get_array("unknown")?.is_none());
    // /// # Ok::<(), Error>(())
    // /// ```
    // pub fn get_array<'a>(&'a self, key: &str) -> Result<Option<&'a RawArray>> {
    //     self.get_with(key, RawBson::as_array)
    // }

    // /// Gets a reference to the BSON binary value corresponding to a given key or returns an
    // error /// if the key corresponds to a value which isn't a binary value.
    // ///
    // /// ```
    // /// # use bson::raw::Error;
    // /// use bson::{
    // ///     doc,
    // ///     raw::{ErrorKind, RawDocument, RawBinary},
    // ///     spec::BinarySubtype,
    // ///     Binary,
    // /// };
    // ///
    // /// let doc = RawDocument::from_document(&doc! {
    // ///     "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1, 2, 3] },
    // ///     "bool": true,
    // /// })?;
    // ///
    // /// assert_eq!(doc.get_binary("binary")?.map(RawBinary::as_bytes), Some(&[1, 2, 3][..]));
    // /// assert!(matches!(doc.get_binary("bool").unwrap_err().kind, ErrorKind::UnexpectedType { ..
    // })); /// assert!(doc.get_binary("unknown")?.is_none());
    // /// # Ok::<(), Error>(())
    // /// ```
    // pub fn get_binary<'a>(&'a self, key: &str) -> Result<Option<RawBinary<'a>>> {
    //     self.get_with(key, RawBson::as_binary)
    // }

    // /// Gets a reference to the ObjectId value corresponding to a given key or returns an error
    // if /// the key corresponds to a value which isn't an ObjectId.
    // ///
    // /// ```
    // /// # use bson::raw::Error;
    // /// use bson::{doc, oid::ObjectId, raw::{ErrorKind, RawDocument}};
    // ///
    // /// let doc = RawDocument::from_document(&doc! {
    // ///     "_id": ObjectId::new(),
    // ///     "bool": true,
    // /// })?;
    // ///
    // /// let oid = doc.get_object_id("_id")?.unwrap();
    // /// assert!(matches!(doc.get_object_id("bool").unwrap_err().kind, ErrorKind::UnexpectedType {
    // .. })); /// assert!(doc.get_object_id("unknown")?.is_none());
    // /// # Ok::<(), Error>(())
    // /// ```
    // pub fn get_object_id(&self, key: &str) -> Result<Option<ObjectId>> {
    //     self.get_with(key, RawBson::as_object_id)
    // }

    // /// Gets a reference to the boolean value corresponding to a given key or returns an error if
    // /// the key corresponds to a value which isn't a boolean.
    // ///
    // /// ```
    // /// # use bson::raw::Error;
    // /// use bson::{doc, oid::ObjectId, raw::{RawDocument, ErrorKind}};
    // ///
    // /// let doc = RawDocument::from_document(&doc! {
    // ///     "_id": ObjectId::new(),
    // ///     "bool": true,
    // /// })?;
    // ///
    // /// assert!(doc.get_bool("bool")?.unwrap());
    // /// assert!(matches!(doc.get_bool("_id").unwrap_err().kind, ErrorKind::UnexpectedType { ..
    // })); /// assert!(doc.get_object_id("unknown")?.is_none());
    // /// # Ok::<(), Error>(())
    // /// ```
    // pub fn get_bool(&self, key: &str) -> Result<Option<bool>> {
    //     self.get_with(key, RawBson::as_bool)
    // }

    // /// Gets a reference to the BSON DateTime value corresponding to a given key or returns an
    // error /// if the key corresponds to a value which isn't a DateTime.
    // ///
    // /// ```
    // /// # use bson::raw::Error;
    // /// use bson::{doc, raw::{ErrorKind, RawDocument}, DateTime};
    // ///
    // /// let dt = DateTime::now();
    // /// let doc = RawDocument::from_document(&doc! {
    // ///     "created_at": dt,
    // ///     "bool": true,
    // /// })?;
    // ///
    // /// assert_eq!(doc.get_datetime("created_at")?, Some(dt));
    // /// assert!(matches!(doc.get_datetime("bool").unwrap_err().kind, ErrorKind::UnexpectedType {
    // .. })); /// assert!(doc.get_datetime("unknown")?.is_none());
    // /// # Ok::<(), Error>(())
    // /// ```
    // pub fn get_datetime(&self, key: &str) -> Result<Option<DateTime>> {
    //     self.get_with(key, RawBson::as_datetime)
    // }

    // /// Gets a reference to the BSON regex value corresponding to a given key or returns an error
    // if /// the key corresponds to a value which isn't a regex.
    // ///
    // /// ```
    // /// # use bson::raw::Error;
    // /// use bson::{doc, Regex, raw::{RawDocument, ErrorKind}};
    // ///
    // /// let doc = RawDocument::from_document(&doc! {
    // ///     "regex": Regex {
    // ///         pattern: r"end\s*$".into(),
    // ///         options: "i".into(),
    // ///     },
    // ///     "bool": true,
    // /// })?;
    // ///
    // /// assert_eq!(doc.get_regex("regex")?.unwrap().pattern(), r"end\s*$");
    // /// assert_eq!(doc.get_regex("regex")?.unwrap().options(), "i");
    // /// assert!(matches!(doc.get_regex("bool").unwrap_err().kind, ErrorKind::UnexpectedType { ..
    // })); /// assert!(doc.get_regex("unknown")?.is_none());
    // /// # Ok::<(), Error>(())
    // /// ```
    // pub fn get_regex<'a>(&'a self, key: &str) -> Result<Option<RawRegex<'a>>> {
    //     self.get_with(key, RawBson::as_regex)
    // }

    // /// Gets a reference to the BSON timestamp value corresponding to a given key or returns an
    // /// error if the key corresponds to a value which isn't a timestamp.
    // ///
    // /// ```
    // /// # use bson::raw::Error;
    // /// use bson::{doc, Timestamp, raw::{RawDocument, ErrorKind}};
    // ///
    // /// let doc = RawDocument::from_document(&doc! {
    // ///     "bool": true,
    // ///     "ts": Timestamp { time: 649876543, increment: 9 },
    // /// })?;
    // ///
    // /// let timestamp = doc.get_timestamp("ts")?.unwrap();
    // ///
    // /// assert_eq!(timestamp.time(), 649876543);
    // /// assert_eq!(timestamp.increment(), 9);
    // /// assert!(matches!(doc.get_timestamp("bool").unwrap_err().kind, ErrorKind::UnexpectedType {
    // .. })); /// assert_eq!(doc.get_timestamp("unknown"), Ok(None));
    // /// # Ok::<(), Error>(())
    // /// ```
    // pub fn get_timestamp<'a>(&'a self, key: &str) -> Result<Option<RawTimestamp<'a>>> {
    //     self.get_with(key, RawBson::as_timestamp)
    // }

    // /// Gets a reference to the BSON int32 value corresponding to a given key or returns an error
    // if /// the key corresponds to a value which isn't a 32-bit integer.
    // ///
    // /// ```
    // /// # use bson::raw::Error;
    // /// use bson::{doc, raw::{RawDocument, ErrorKind}};
    // ///
    // /// let doc = RawDocument::from_document(&doc! {
    // ///     "bool": true,
    // ///     "i32": 1_000_000,
    // /// })?;
    // ///
    // /// assert_eq!(doc.get_i32("i32"), Ok(Some(1_000_000)));
    // /// assert!(matches!(doc.get_i32("bool").unwrap_err().kind, ErrorKind::UnexpectedType { ..
    // })); /// assert_eq!(doc.get_i32("unknown"), Ok(None));
    // /// # Ok::<(), Error>(())
    // /// ```
    // pub fn get_i32(&self, key: &str) -> Result<Option<i32>> {
    //     self.get_with(key, RawBson::as_i32)
    // }

    // /// Gets a reference to the BSON int64 value corresponding to a given key or returns an error
    // if /// the key corresponds to a value which isn't a 64-bit integer.
    // ///
    // /// ```
    // /// # use bson::raw::Error;
    // /// use bson::{doc, raw::{ErrorKind, RawDocument}};
    // ///
    // /// let doc = RawDocument::from_document(&doc! {
    // ///     "bool": true,
    // ///     "i64": 9223372036854775807_i64,
    // /// })?;
    // ///
    // /// assert_eq!(doc.get_i64("i64"), Ok(Some(9223372036854775807)));
    // /// assert!(matches!(doc.get_i64("bool").unwrap_err().kind, ErrorKind::UnexpectedType { ..
    // })); /// assert_eq!(doc.get_i64("unknown"), Ok(None));
    // /// # Ok::<(), Error>(())
    // /// ```
    // pub fn get_i64(&self, key: &str) -> Result<Option<i64>> {
    //     self.get_with(key, RawBson::as_i64)
    // }

    /// Return a reference to the contained data as a `&[u8]`
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, raw::RawDocument};
    /// let docbuf = RawDocument::from_document(&doc!{})?;
    /// assert_eq!(docbuf.as_bytes(), b"\x05\x00\x00\x00\x00");
    /// # Ok::<(), Error>(())
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

impl<'a> From<&'a RawDocumentRef> for Cow<'a, RawDocumentRef> {
    fn from(rdr: &'a RawDocumentRef) -> Self {
        Cow::Borrowed(rdr)
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
    type IntoIter = Iter<'a>;
    type Item = Result<(&'a str, RawBson<'a>)>;

    fn into_iter(self) -> Iter<'a> {
        Iter {
            doc: self,
            offset: 4,
        }
    }
}

/// An iterator over the document's entries.
pub struct Iter<'a> {
    doc: &'a RawDocumentRef,
    offset: usize,
}

impl<'a> Iter<'a> {
    fn verify_null_terminated(&self, range: Range<usize>) -> Result<()> {
        if range.is_empty() {
            return Err(Error::new_without_key(ErrorKind::MalformedValue {
                message: "value has empty range".to_string(),
            }));
        }

        self.verify_in_range(range.clone())?;
        if self.doc.data[range.end - 1] == 0 {
            return Ok(());
        } else {
            return Err(Error {
                key: None,
                kind: ErrorKind::MalformedValue {
                    message: "not null terminated".into(),
                },
            });
        }
    }

    fn verify_in_range(&self, range: Range<usize>) -> Result<()> {
        let start = range.start;
        let len = range.len();
        if self.doc.data.get(range).is_none() {
            return Err(Error::new_without_key(ErrorKind::MalformedValue {
                message: format!(
                    "length exceeds remaining length of buffer: {} vs {}",
                    len,
                    self.doc.data.len() - start
                ),
            }));
        }
        Ok(())
    }

    fn next_oid(&self, starting_at: usize) -> Result<ObjectId> {
        self.verify_in_range(starting_at..(starting_at + 12))?;
        let oid = ObjectId::from_bytes(
            self.doc.data[starting_at..(starting_at + 12)]
                .try_into()
                .unwrap(), // ok because we know slice is 12 bytes long
        );
        Ok(oid)
    }

    fn next_document(&self, starting_at: usize) -> Result<&'a RawDocumentRef> {
        let size = i32_from_slice(&self.doc.data[starting_at..])? as usize;
        let range = starting_at..(starting_at + size);
        self.verify_null_terminated(range.clone())?;
        RawDocumentRef::new(&self.doc.data[range])
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<(&'a str, RawBson<'a>)>;

    fn next(&mut self) -> Option<Result<(&'a str, RawBson<'a>)>> {
        if self.offset == self.doc.data.len() - 1 {
            if self.doc.data[self.offset] == 0 {
                // end of document marker
                return None;
            } else {
                return Some(Err(Error {
                    key: None,
                    kind: ErrorKind::MalformedValue {
                        message: "document not null terminated".into(),
                    },
                }));
            }
        } else if self.offset >= self.doc.data.len() {
            // return None on subsequent iterations after an error
            return None;
        }

        let key = match read_nullterminated(&self.doc.data[self.offset + 1..]) {
            Ok(k) => k,
            Err(e) => return Some(Err(e)),
        };

        println!("iterating {}", key);

        let kvp_result = try_with_key(key, || {
            let valueoffset = self.offset + 1 + key.len() + 1; // type specifier + key + \0

            let element_type = match ElementType::from(self.doc.data[self.offset]) {
                Some(et) => et,
                None => {
                    return Err(Error::new_with_key(
                        key,
                        ErrorKind::MalformedValue {
                            message: format!("invalid tag: {}", self.doc.data[self.offset]),
                        },
                    ))
                }
            };

            println!("et: {:?}", element_type);

            let (element, element_size) = match element_type {
                ElementType::Int32 => {
                    let i = i32_from_slice(&self.doc.data[valueoffset..])?;
                    (RawBson::Int32(i), 4)
                }
                ElementType::Int64 => {
                    let i = i64_from_slice(&self.doc.data[valueoffset..])?;
                    (RawBson::Int64(i), 8)
                }
                ElementType::Double => {
                    let f = f64_from_slice(&self.doc.data[valueoffset..])?;
                    (RawBson::Double(f), 8)
                }
                ElementType::String => {
                    let s = read_lenencoded(&self.doc.data[valueoffset..])?;
                    (RawBson::String(s), 4 + s.len() + 1)
                }
                ElementType::EmbeddedDocument => {
                    let doc = self.next_document(valueoffset)?;
                    (RawBson::Document(doc), doc.as_bytes().len())
                }
                ElementType::Array => {
                    let doc = self.next_document(valueoffset)?;
                    (
                        RawBson::Array(RawArray::from_doc(doc)),
                        doc.as_bytes().len(),
                    )
                }
                ElementType::Binary => {
                    let len = i32_from_slice(&self.doc.data[valueoffset..])? as usize;
                    let data_start = valueoffset + 4 + 1;
                    self.verify_in_range(valueoffset..(data_start + len))?;
                    let subtype = BinarySubtype::from(self.doc.data[valueoffset + 4]);
                    let data = match subtype {
                        BinarySubtype::BinaryOld => {
                            if len < 4 {
                                return Err(Error::new_without_key(ErrorKind::MalformedValue {
                                    message: "old binary subtype has no inner declared length"
                                        .into(),
                                }));
                            }
                            let oldlength = i32_from_slice(&self.doc.data[data_start..])? as usize;
                            if oldlength + 4 != len {
                                return Err(Error::new_without_key(ErrorKind::MalformedValue {
                                    message: "old binary subtype has wrong inner declared length"
                                        .into(),
                                }));
                            }
                            &self.doc.data[(data_start + 4)..(data_start + len)]
                        }
                        _ => &self.doc.data[data_start..(data_start + len)],
                    };
                    (RawBson::Binary(RawBinary { subtype, data }), 4 + 1 + len)
                }
                ElementType::ObjectId => {
                    let oid = self.next_oid(valueoffset)?;
                    (RawBson::ObjectId(oid), 12)
                }
                ElementType::Boolean => {
                    let b = read_bool(&self.doc.data[valueoffset..]).map_err(|e| {
                        Error::new_with_key(
                            key,
                            ErrorKind::MalformedValue {
                                message: e.to_string(),
                            },
                        )
                    })?;
                    (RawBson::Boolean(b), 1)
                }
                ElementType::DateTime => {
                    let ms = i64_from_slice(&self.doc.data[valueoffset..])?;
                    (RawBson::DateTime(DateTime::from_millis(ms)), 8)
                }
                ElementType::RegularExpression => {
                    let pattern = read_nullterminated(&self.doc.data[valueoffset..])?;
                    let options =
                        read_nullterminated(&self.doc.data[(valueoffset + pattern.len() + 1)..])?;
                    (
                        RawBson::RegularExpression(RawRegex { pattern, options }),
                        pattern.len() + 1 + options.len() + 1,
                    )
                }
                ElementType::Null => (RawBson::Null, 0),
                ElementType::Undefined => (RawBson::Undefined, 0),
                ElementType::Timestamp => {
                    let ts =
                        Timestamp::from_reader(&self.doc.data[valueoffset..]).map_err(|e| {
                            Error::new_without_key(ErrorKind::MalformedValue {
                                message: e.to_string(),
                            })
                        })?;
                    (RawBson::Timestamp(ts), 8)
                }
                ElementType::JavaScriptCode => {
                    let code = read_lenencoded(&self.doc.data[valueoffset..])?;
                    (RawBson::JavaScriptCode(code), 4 + code.len() + 1)
                }
                ElementType::JavaScriptCodeWithScope => {
                    let length = i32_from_slice(&self.doc.data[valueoffset..])? as usize;
                    self.verify_in_range(valueoffset..(valueoffset + length))?;
                    let code = read_lenencoded(&self.doc.data[valueoffset + 4..])?;

                    let scope_start = valueoffset + 4 + 4 + code.len() + 1;
                    let scope =
                        RawDocumentRef::new(&self.doc.data[scope_start..(valueoffset + length)])?;
                    (
                        RawBson::JavaScriptCodeWithScope(RawJavaScriptCodeWithScope {
                            code,
                            scope,
                        }),
                        length,
                    )
                }
                ElementType::DbPointer => {
                    let ns = read_lenencoded(&self.doc.data[valueoffset..])?;
                    let oid = self.next_oid(valueoffset + 4 + ns.len() + 1)?;
                    // TODO: fix this
                    (RawBson::DbPointer(0), 4 + ns.len() + 1 + 12)
                }
                ElementType::Symbol => {
                    let s = read_lenencoded(&self.doc.data[valueoffset..])?;
                    (RawBson::Symbol(s), 4 + s.len() + 1)
                }
                ElementType::Decimal128 => {
                    self.verify_in_range(valueoffset..(valueoffset + 16))?;
                    (
                        RawBson::Decimal128(Decimal128::from_bytes(
                            self.doc.data[valueoffset..(valueoffset + 16)]
                                .try_into()
                                .unwrap(),
                        )),
                        16,
                    )
                }
                ElementType::MinKey => (RawBson::MinKey, 0),
                ElementType::MaxKey => (RawBson::MaxKey, 0),
            };

            let nextoffset = valueoffset + element_size;
            self.offset = nextoffset;

            self.verify_in_range(valueoffset..nextoffset)?;

            Ok((key, element))
        });

        Some(kvp_result)
    }
}
