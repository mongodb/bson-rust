use std::{
    borrow::Cow,
    convert::{TryFrom, TryInto},
};

use crate::{raw::error::ErrorKind, DateTime, Timestamp};

use super::{
    error::{ValueAccessError, ValueAccessErrorKind, ValueAccessResult},
    i32_from_slice,
    Error,
    Iter,
    RawArr,
    RawBinary,
    RawBson,
    RawDocument,
    RawRegex,
    Result,
};
use crate::{oid::ObjectId, spec::ElementType, Document};

/// A slice of a BSON document (akin to [`std::str`]). This can be created from a
/// [`RawDocument`] or any type that contains valid BSON data, including static binary literals,
/// [Vec<u8>](std::vec::Vec), or arrays.
///
/// This is an _unsized_ type, meaning that it must always be used behind a pointer like `&`. For an
/// owned version of this type, see [`RawDocument`].
///
/// Accessing elements within a [`RawDoc`] is similar to element access in [`crate::Document`],
/// but because the contents are parsed during iteration instead of at creation time, format errors
/// can happen at any time during use.
///
/// Iterating over a [`RawDoc`] yields either an error or a key-value pair that borrows from the
/// original document without making any additional allocations.
/// ```
/// # use bson::raw::{Error};
/// use bson::raw::RawDoc;
///
/// let doc = RawDoc::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00")?;
/// let mut iter = doc.into_iter();
/// let (key, value) = iter.next().unwrap()?;
/// assert_eq!(key, "hi");
/// assert_eq!(value.as_str(), Some("y'all"));
/// assert!(iter.next().is_none());
/// # Ok::<(), Error>(())
/// ```
///
/// Individual elements can be accessed using [`RawDoc::get`] or any of
/// the type-specific getters, such as [`RawDoc::get_object_id`] or
/// [`RawDoc::get_str`]. Note that accessing elements is an O(N) operation, as it
/// requires iterating through the document from the beginning to find the requested key.
///
/// ```
/// use bson::raw::RawDoc;
///
/// let doc = RawDoc::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00")?;
/// assert_eq!(doc.get_str("hi")?, "y'all");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(PartialEq)]
#[repr(transparent)]
pub struct RawDoc {
    data: [u8],
}

impl RawDoc {
    /// Constructs a new [`RawDoc`], validating _only_ the
    /// following invariants:
    ///   * `data` is at least five bytes long (the minimum for a valid BSON document)
    ///   * the initial four bytes of `data` accurately represent the length of the bytes as
    ///     required by the BSON spec.
    ///   * the last byte of `data` is a 0
    ///
    /// Note that the internal structure of the bytes representing the
    /// BSON elements is _not_ validated at all by this method. If the
    /// bytes do not conform to the BSON spec, then method calls on
    /// the [`RawDoc`] will return Errors where appropriate.
    ///
    /// ```
    /// use bson::raw::RawDoc;
    ///
    /// let doc = RawDoc::new(b"\x05\0\0\0\0")?;
    /// # Ok::<(), bson::raw::Error>(())
    /// ```
    pub fn new<D: AsRef<[u8]> + ?Sized>(data: &D) -> Result<&RawDoc> {
        let data = data.as_ref();

        if data.len() < 5 {
            return Err(Error {
                key: None,
                kind: ErrorKind::MalformedValue {
                    message: "document too short".into(),
                },
            });
        }

        let length = i32_from_slice(data)?;

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

        Ok(RawDoc::new_unchecked(data))
    }

    /// Creates a new Doc referencing the provided data slice.
    pub(crate) fn new_unchecked<D: AsRef<[u8]> + ?Sized>(data: &D) -> &RawDoc {
        // SAFETY:
        //
        // Dereferencing a raw pointer requires unsafe due to the potential that the pointer is
        // null, dangling, or misaligned. We know the pointer is not null or dangling due to the
        // fact that it's created by a safe reference. Converting &[u8] to *const [u8] will be
        // properly aligned due to them being references to the same type, and converting *const
        // [u8] to *const RawDoc is aligned due to the fact that the only field in a
        // RawDoc is a [u8] and it is #[repr(transparent), meaning the structs are represented
        // identically at the byte level.
        unsafe { &*(data.as_ref() as *const [u8] as *const RawDoc) }
    }

    /// Creates a new [`RawDocument`] with an owned copy of the BSON bytes.
    ///
    /// ```
    /// use bson::raw::{RawDoc, RawDocument, Error};
    ///
    /// let data = b"\x05\0\0\0\0";
    /// let doc_ref = RawDoc::new(data)?;
    /// let doc: RawDocument = doc_ref.to_raw_document();
    /// # Ok::<(), Error>(())
    pub fn to_raw_document(&self) -> RawDocument {
        // unwrap is ok here because we already verified the bytes in `RawDocumentRef::new`
        RawDocument::new(self.data.to_owned()).unwrap()
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
    /// assert_eq!(element.as_f64(), Some(2.5));
    /// assert!(doc.get("unknown")?.is_none());
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get(&self, key: impl AsRef<str>) -> Result<Option<RawBson<'_>>> {
        for result in self.into_iter() {
            let (k, v) = result?;
            if key.as_ref() == k {
                return Ok(Some(v));
            }
        }
        Ok(None)
    }

    fn get_with<'a, T>(
        &'a self,
        key: impl AsRef<str>,
        expected_type: ElementType,
        f: impl FnOnce(RawBson<'a>) -> Option<T>,
    ) -> ValueAccessResult<T> {
        let key = key.as_ref();

        let bson = self
            .get(key)
            .map_err(|e| ValueAccessError {
                key: key.to_string(),
                kind: ValueAccessErrorKind::InvalidBson(e),
            })?
            .ok_or(ValueAccessError {
                key: key.to_string(),
                kind: ValueAccessErrorKind::NotPresent,
            })?;
        match f(bson) {
            Some(t) => Ok(t),
            None => Err(ValueAccessError {
                key: key.to_string(),
                kind: ValueAccessErrorKind::UnexpectedType {
                    expected: expected_type,
                    actual: bson.element_type(),
                },
            }),
        }
    }

    /// Gets a reference to the BSON double value corresponding to a given key or returns an error
    /// if the key corresponds to a value which isn't a double.
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::raw::{ValueAccessErrorKind, RawDocument};
    /// use bson::doc;
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "bool": true,
    ///     "f64": 2.5,
    /// })?;
    ///
    /// assert_eq!(doc.get_f64("f64")?, 2.5);
    /// assert!(matches!(doc.get_f64("bool").unwrap_err().kind, ValueAccessErrorKind::UnexpectedType { .. }));
    /// assert!(matches!(doc.get_f64("unknown").unwrap_err().kind, ValueAccessErrorKind::NotPresent));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_f64(&self, key: impl AsRef<str>) -> ValueAccessResult<f64> {
        self.get_with(key, ElementType::Double, RawBson::as_f64)
    }

    /// Gets a reference to the string value corresponding to a given key or returns an error if the
    /// key corresponds to a value which isn't a string.
    ///
    /// ```
    /// use bson::{doc, raw::{RawDocument, ValueAccessErrorKind}};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "string": "hello",
    ///     "bool": true,
    /// })?;
    ///
    /// assert_eq!(doc.get_str("string")?, "hello");
    /// assert!(matches!(doc.get_str("bool").unwrap_err().kind, ValueAccessErrorKind::UnexpectedType { .. }));
    /// assert!(matches!(doc.get_str("unknown").unwrap_err().kind, ValueAccessErrorKind::NotPresent));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_str(&self, key: impl AsRef<str>) -> ValueAccessResult<&'_ str> {
        self.get_with(key, ElementType::String, RawBson::as_str)
    }

    /// Gets a reference to the document value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a document.
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, raw::{ValueAccessErrorKind, RawDocument}};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "doc": { "key": "value"},
    ///     "bool": true,
    /// })?;
    ///
    /// assert_eq!(doc.get_document("doc")?.get_str("key")?, "value");
    /// assert!(matches!(doc.get_document("bool").unwrap_err().kind, ValueAccessErrorKind::UnexpectedType { .. }));
    /// assert!(matches!(doc.get_document("unknown").unwrap_err().kind, ValueAccessErrorKind::NotPresent));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_document(&self, key: impl AsRef<str>) -> ValueAccessResult<&'_ RawDoc> {
        self.get_with(key, ElementType::EmbeddedDocument, RawBson::as_document)
    }

    /// Gets a reference to the array value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't an array.
    ///
    /// ```
    /// use bson::{doc, raw::{RawDocument, ValueAccessErrorKind}};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "array": [true, 3],
    ///     "bool": true,
    /// })?;
    ///
    /// let mut arr_iter = doc.get_array("array")?.into_iter();
    /// let _: bool = arr_iter.next().unwrap()?.as_bool().unwrap();
    /// let _: i32 = arr_iter.next().unwrap()?.as_i32().unwrap();
    ///
    /// assert!(arr_iter.next().is_none());
    /// assert!(doc.get_array("bool").is_err());
    /// assert!(matches!(doc.get_array("unknown").unwrap_err().kind, ValueAccessErrorKind::NotPresent));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_array(&self, key: impl AsRef<str>) -> ValueAccessResult<&'_ RawArr> {
        self.get_with(key, ElementType::Array, RawBson::as_array)
    }

    /// Gets a reference to the BSON binary value corresponding to a given key or returns an error
    /// if the key corresponds to a value which isn't a binary value.
    ///
    /// ```
    /// use bson::{
    ///     doc,
    ///     raw::{ValueAccessErrorKind, RawDocument, RawBinary},
    ///     spec::BinarySubtype,
    ///     Binary,
    /// };
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1, 2, 3] },
    ///     "bool": true,
    /// })?;
    ///
    /// assert_eq!(doc.get_binary("binary")?.as_bytes(), &[1, 2, 3][..]);
    /// assert!(matches!(doc.get_binary("bool").unwrap_err().kind, ValueAccessErrorKind::UnexpectedType { .. }));
    /// assert!(matches!(doc.get_binary("unknown").unwrap_err().kind, ValueAccessErrorKind::NotPresent));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_binary(&self, key: impl AsRef<str>) -> ValueAccessResult<RawBinary<'_>> {
        self.get_with(key, ElementType::Binary, RawBson::as_binary)
    }

    /// Gets a reference to the ObjectId value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't an ObjectId.
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, oid::ObjectId, raw::{ValueAccessErrorKind, RawDocument}};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "_id": ObjectId::new(),
    ///     "bool": true,
    /// })?;
    ///
    /// let oid = doc.get_object_id("_id")?;
    /// assert!(matches!(doc.get_object_id("bool").unwrap_err().kind, ValueAccessErrorKind::UnexpectedType { .. }));
    /// assert!(matches!(doc.get_object_id("unknown").unwrap_err().kind, ValueAccessErrorKind::NotPresent));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_object_id(&self, key: impl AsRef<str>) -> ValueAccessResult<ObjectId> {
        self.get_with(key, ElementType::ObjectId, RawBson::as_object_id)
    }

    /// Gets a reference to the boolean value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a boolean.
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, oid::ObjectId, raw::{RawDocument, ValueAccessErrorKind}};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "_id": ObjectId::new(),
    ///     "bool": true,
    /// })?;
    ///
    /// assert!(doc.get_bool("bool")?);
    /// assert!(matches!(doc.get_bool("_id").unwrap_err().kind, ValueAccessErrorKind::UnexpectedType { .. }));
    /// assert!(matches!(doc.get_bool("unknown").unwrap_err().kind, ValueAccessErrorKind::NotPresent));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_bool(&self, key: impl AsRef<str>) -> ValueAccessResult<bool> {
        self.get_with(key, ElementType::Boolean, RawBson::as_bool)
    }

    /// Gets a reference to the BSON DateTime value corresponding to a given key or returns an
    /// error if the key corresponds to a value which isn't a DateTime.
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, raw::{ValueAccessErrorKind, RawDocument}, DateTime};
    ///
    /// let dt = DateTime::now();
    /// let doc = RawDocument::from_document(&doc! {
    ///     "created_at": dt,
    ///     "bool": true,
    /// })?;
    ///
    /// assert_eq!(doc.get_datetime("created_at")?, dt);
    /// assert!(matches!(doc.get_datetime("bool").unwrap_err().kind, ValueAccessErrorKind::UnexpectedType { .. }));
    /// assert!(matches!(doc.get_datetime("unknown").unwrap_err().kind, ValueAccessErrorKind::NotPresent));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_datetime(&self, key: impl AsRef<str>) -> ValueAccessResult<DateTime> {
        self.get_with(key, ElementType::DateTime, RawBson::as_datetime)
    }

    /// Gets a reference to the BSON regex value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a regex.
    ///
    /// ```
    /// use bson::{doc, Regex, raw::{RawDocument, ValueAccessErrorKind}};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "regex": Regex {
    ///         pattern: r"end\s*$".into(),
    ///         options: "i".into(),
    ///     },
    ///     "bool": true,
    /// })?;
    ///
    /// assert_eq!(doc.get_regex("regex")?.pattern(), r"end\s*$");
    /// assert_eq!(doc.get_regex("regex")?.options(), "i");
    /// assert!(matches!(doc.get_regex("bool").unwrap_err().kind, ValueAccessErrorKind::UnexpectedType { .. }));
    /// assert!(matches!(doc.get_regex("unknown").unwrap_err().kind, ValueAccessErrorKind::NotPresent));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_regex(&self, key: impl AsRef<str>) -> ValueAccessResult<RawRegex<'_>> {
        self.get_with(key, ElementType::RegularExpression, RawBson::as_regex)
    }

    /// Gets a reference to the BSON timestamp value corresponding to a given key or returns an
    /// error if the key corresponds to a value which isn't a timestamp.
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, Timestamp, raw::{RawDocument, ValueAccessErrorKind}};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "bool": true,
    ///     "ts": Timestamp { time: 649876543, increment: 9 },
    /// })?;
    ///
    /// let timestamp = doc.get_timestamp("ts")?;
    ///
    /// assert_eq!(timestamp.time, 649876543);
    /// assert_eq!(timestamp.increment, 9);
    /// assert!(matches!(doc.get_timestamp("bool").unwrap_err().kind, ValueAccessErrorKind::UnexpectedType { .. }));
    /// assert!(matches!(doc.get_timestamp("unknown").unwrap_err().kind, ValueAccessErrorKind::NotPresent));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_timestamp(&self, key: impl AsRef<str>) -> ValueAccessResult<Timestamp> {
        self.get_with(key, ElementType::Timestamp, RawBson::as_timestamp)
    }

    /// Gets a reference to the BSON int32 value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a 32-bit integer.
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, raw::{RawDocument, ValueAccessErrorKind}};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "bool": true,
    ///     "i32": 1_000_000,
    /// })?;
    ///
    /// assert_eq!(doc.get_i32("i32")?, 1_000_000);
    /// assert!(matches!(doc.get_i32("bool").unwrap_err().kind, ValueAccessErrorKind::UnexpectedType { ..}));
    /// assert!(matches!(doc.get_i32("unknown").unwrap_err().kind, ValueAccessErrorKind::NotPresent));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_i32(&self, key: impl AsRef<str>) -> ValueAccessResult<i32> {
        self.get_with(key, ElementType::Int32, RawBson::as_i32)
    }

    /// Gets a reference to the BSON int64 value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a 64-bit integer.
    ///
    /// ```
    /// # use bson::raw::Error;
    /// use bson::{doc, raw::{ValueAccessErrorKind, RawDocument}};
    ///
    /// let doc = RawDocument::from_document(&doc! {
    ///     "bool": true,
    ///     "i64": 9223372036854775807_i64,
    /// })?;
    ///
    /// assert_eq!(doc.get_i64("i64")?, 9223372036854775807);
    /// assert!(matches!(doc.get_i64("bool").unwrap_err().kind, ValueAccessErrorKind::UnexpectedType { .. }));
    /// assert!(matches!(doc.get_i64("unknown").unwrap_err().kind, ValueAccessErrorKind::NotPresent));
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_i64(&self, key: impl AsRef<str>) -> ValueAccessResult<i64> {
        self.get_with(key, ElementType::Int64, RawBson::as_i64)
    }

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

impl std::fmt::Debug for RawDoc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawDoc")
            .field("data", &hex::encode(&self.data))
            .finish()
    }
}

impl AsRef<RawDoc> for RawDoc {
    fn as_ref(&self) -> &RawDoc {
        self
    }
}

impl ToOwned for RawDoc {
    type Owned = RawDocument;

    fn to_owned(&self) -> Self::Owned {
        self.to_raw_document()
    }
}

impl<'a> From<&'a RawDoc> for Cow<'a, RawDoc> {
    fn from(rdr: &'a RawDoc) -> Self {
        Cow::Borrowed(rdr)
    }
}

impl TryFrom<&RawDoc> for crate::Document {
    type Error = Error;

    fn try_from(rawdoc: &RawDoc) -> Result<Document> {
        rawdoc
            .into_iter()
            .map(|res| res.and_then(|(k, v)| Ok((k.to_owned(), v.try_into()?))))
            .collect()
    }
}

impl<'a> IntoIterator for &'a RawDoc {
    type IntoIter = Iter<'a>;
    type Item = Result<(&'a str, RawBson<'a>)>;

    fn into_iter(self) -> Iter<'a> {
        Iter::new(self)
    }
}
