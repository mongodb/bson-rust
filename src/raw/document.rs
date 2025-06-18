use std::{
    borrow::Cow,
    convert::{TryFrom, TryInto},
};

use crate::{
    error::{Error, Result},
    Bson,
    DateTime,
    JavaScriptCodeWithScope,
    RawBson,
    RawJavaScriptCodeWithScope,
    Timestamp,
};

use super::{
    i32_from_slice,
    iter::Iter,
    try_to_str,
    Error as RawError,
    RawArray,
    RawBinaryRef,
    RawBsonRef,
    RawDocumentBuf,
    RawIter,
    RawRegexRef,
    Result as RawResult,
    MIN_BSON_DOCUMENT_SIZE,
};
use crate::{oid::ObjectId, spec::ElementType, Document};

/// A slice of a BSON document (akin to [`std::str`]). This can be created from a
/// [`RawDocumentBuf`] or any type that contains valid BSON data, including static binary literals,
/// [`Vec<u8>`](std::vec::Vec), or arrays.
///
/// This is an _unsized_ type, meaning that it must always be used behind a pointer like `&`. For an
/// owned version of this type, see [`RawDocumentBuf`].
///
/// Accessing elements within a [`RawDocument`] is similar to element access in [`crate::Document`],
/// but because the contents are parsed during iteration instead of at creation time, format errors
/// can happen at any time during use.
///
/// Iterating over a [`RawDocument`] yields either an error or a key-value pair that borrows from
/// the original document without making any additional allocations.
/// ```
/// # use bson::error::Error;
/// use bson::raw::RawDocument;
///
/// let doc = RawDocument::decode_from_bytes(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00")?;
/// let mut iter = doc.into_iter();
/// let (key, value) = iter.next().unwrap()?;
/// assert_eq!(key, "hi");
/// assert_eq!(value.as_str(), Some("y'all"));
/// assert!(iter.next().is_none());
/// # Ok::<(), Error>(())
/// ```
///
/// Individual elements can be accessed using [`RawDocument::get`] or any of
/// the type-specific getters, such as [`RawDocument::get_object_id`] or
/// [`RawDocument::get_str`]. Note that accessing elements is an O(N) operation, as it
/// requires iterating through the document from the beginning to find the requested key.
///
/// ```
/// use bson::raw::RawDocument;
///
/// let doc = RawDocument::decode_from_bytes(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00")?;
/// assert_eq!(doc.get_str("hi")?, "y'all");
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(PartialEq)]
#[repr(transparent)]
pub struct RawDocument {
    data: [u8],
}

impl RawDocument {
    /// Constructs a new [`RawDocument`], validating _only_ the
    /// following invariants:
    ///   * `data` is at least five bytes long (the minimum for a valid BSON document)
    ///   * the initial four bytes of `data` accurately represent the length of the bytes as
    ///     required by the BSON spec.
    ///   * the last byte of `data` is a 0
    ///
    /// Note that the internal structure of the bytes representing the
    /// BSON elements is _not_ validated at all by this method. If the
    /// bytes do not conform to the BSON spec, then method calls on
    /// the [`RawDocument`] will return Errors where appropriate.
    ///
    /// ```
    /// use bson::raw::RawDocument;
    ///
    /// let doc = RawDocument::decode_from_bytes(b"\x05\0\0\0\0")?;
    /// # Ok::<(), bson::error::Error>(())
    /// ```
    pub fn decode_from_bytes<D: AsRef<[u8]> + ?Sized>(data: &D) -> RawResult<&RawDocument> {
        let data = data.as_ref();

        if data.len() < 5 {
            return Err(Error::malformed_bytes("document too short"));
        }

        let length = i32_from_slice(data)?;

        if data.len() as i32 != length {
            return Err(Error::malformed_bytes("document length incorrect"));
        }

        if data[data.len() - 1] != 0 {
            return Err(Error::malformed_bytes("document not null-terminated"));
        }

        Ok(RawDocument::new_unchecked(data))
    }

    /// Creates a new [`RawDocument`] referencing the provided data slice.
    pub(crate) fn new_unchecked<D: AsRef<[u8]> + ?Sized>(data: &D) -> &RawDocument {
        // SAFETY:
        //
        // Dereferencing a raw pointer requires unsafe due to the potential that the pointer is
        // null, dangling, or misaligned. We know the pointer is not null or dangling due to the
        // fact that it's created by a safe reference. Converting &[u8] to *const [u8] will be
        // properly aligned due to them being references to the same type, and converting *const
        // [u8] to *const RawDocument is aligned due to the fact that the only field in a
        // RawDocument is a [u8] and it is #[repr(transparent), meaning the structs are represented
        // identically at the byte level.
        unsafe { &*(data.as_ref() as *const [u8] as *const RawDocument) }
    }

    /// Creates a new [`RawDocumentBuf`] with an owned copy of the BSON bytes.
    ///
    /// ```
    /// use bson::raw::{RawDocument, RawDocumentBuf};
    ///
    /// let data = b"\x05\0\0\0\0";
    /// let doc_ref = RawDocument::decode_from_bytes(data)?;
    /// let doc: RawDocumentBuf = doc_ref.to_raw_document_buf();
    /// # Ok::<(), bson::error::Error>(())
    pub fn to_raw_document_buf(&self) -> RawDocumentBuf {
        // unwrap is ok here because we already verified the bytes in `RawDocumentRef::new`
        RawDocumentBuf::decode_from_bytes(self.data.to_owned()).unwrap()
    }

    /// Gets a reference to the value corresponding to the given key by iterating until the key is
    /// found.
    ///
    /// ```
    /// # use bson::error::Error;
    /// use bson::{rawdoc, oid::ObjectId};
    ///
    /// let doc = rawdoc! {
    ///     "_id": ObjectId::new(),
    ///     "f64": 2.5,
    /// };
    ///
    /// let element = doc.get("f64")?.expect("finding key f64");
    /// assert_eq!(element.as_f64(), Some(2.5));
    /// assert!(doc.get("unknown")?.is_none());
    /// # Ok::<(), Error>(())
    /// ```
    pub fn get(&self, key: impl AsRef<str>) -> RawResult<Option<RawBsonRef<'_>>> {
        for elem in RawIter::new(self) {
            let elem = elem?;
            if key.as_ref() == elem.key() {
                return Ok(Some(elem.try_into()?));
            }
        }
        Ok(None)
    }

    /// Gets an iterator over the elements in the [`RawDocument`] that yields
    /// `Result<(&str, RawBson<'_>)>`.
    pub fn iter(&self) -> Iter<'_> {
        Iter::new(self)
    }

    /// Gets an iterator over the elements in the [`RawDocument`],
    /// which yields `Result<RawElement<'_>>` values. These hold a
    /// reference to the underlying document but do not explicitly
    /// resolve the values.
    ///
    /// This iterator, which underpins the implementation of the
    /// default iterator, produces `RawElement` objects that hold a
    /// view onto the document but do not parse out or construct
    /// values until the `.value()` or `.try_into()` methods are
    /// called.
    pub fn iter_elements(&self) -> RawIter<'_> {
        RawIter::new(self)
    }

    fn get_with<'a, T>(
        &'a self,
        key: impl AsRef<str>,
        expected_type: ElementType,
        f: impl FnOnce(RawBsonRef<'a>) -> Option<T>,
    ) -> Result<T> {
        let key = key.as_ref();

        let bson = self
            .get(key)
            .map_err(|e| Error::value_access_invalid_bson(format!("{:?}", e)))?
            .ok_or_else(Error::value_access_not_present)
            .map_err(|e| e.with_key(key))?;
        match f(bson) {
            Some(t) => Ok(t),
            None => Err(
                Error::value_access_unexpected_type(bson.element_type(), expected_type)
                    .with_key(key),
            ),
        }
    }

    /// Gets a reference to the BSON double value corresponding to a given key or returns an error
    /// if the key corresponds to a value which isn't a double.
    ///
    /// ```
    /// # use bson::error::Error;
    /// use bson::rawdoc;
    ///
    /// let doc = rawdoc! {
    ///     "bool": true,
    ///     "f64": 2.5,
    /// };
    ///
    /// assert_eq!(doc.get_f64("f64")?, 2.5);
    /// assert!(doc.get_f64("bool").is_err());
    /// assert!(doc.get_f64("unknown").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_f64(&self, key: impl AsRef<str>) -> Result<f64> {
        self.get_with(key, ElementType::Double, RawBsonRef::as_f64)
    }

    /// Gets a reference to the string value corresponding to a given key or returns an error if the
    /// key corresponds to a value which isn't a string.
    ///
    /// ```
    /// use bson::rawdoc;
    ///
    /// let doc = rawdoc! {
    ///     "string": "hello",
    ///     "bool": true,
    /// };
    ///
    /// assert_eq!(doc.get_str("string")?, "hello");
    /// assert!(doc.get_str("bool").is_err());
    /// assert!(doc.get_str("unknown").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_str(&self, key: impl AsRef<str>) -> Result<&'_ str> {
        self.get_with(key, ElementType::String, RawBsonRef::as_str)
    }

    /// Gets a reference to the document value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a document.
    ///
    /// ```
    /// # use bson::error::Error;
    /// use bson::rawdoc;
    ///
    /// let doc = rawdoc! {
    ///     "doc": { "key": "value"},
    ///     "bool": true,
    /// };
    ///
    /// assert_eq!(doc.get_document("doc")?.get_str("key")?, "value");
    /// assert!(doc.get_document("bool").is_err());
    /// assert!(doc.get_document("unknown").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_document(&self, key: impl AsRef<str>) -> Result<&'_ RawDocument> {
        self.get_with(key, ElementType::EmbeddedDocument, RawBsonRef::as_document)
    }

    /// Gets a reference to the array value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't an array.
    ///
    /// ```
    /// use bson::rawdoc;
    ///
    /// let doc = rawdoc! {
    ///     "array": [true, 3],
    ///     "bool": true,
    /// };
    ///
    /// let mut arr_iter = doc.get_array("array")?.into_iter();
    /// let _: bool = arr_iter.next().unwrap()?.as_bool().unwrap();
    /// let _: i32 = arr_iter.next().unwrap()?.as_i32().unwrap();
    ///
    /// assert!(arr_iter.next().is_none());
    /// assert!(doc.get_array("bool").is_err());
    /// assert!(doc.get_array("unknown").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_array(&self, key: impl AsRef<str>) -> Result<&'_ RawArray> {
        self.get_with(key, ElementType::Array, RawBsonRef::as_array)
    }

    /// Gets a reference to the BSON binary value corresponding to a given key or returns an error
    /// if the key corresponds to a value which isn't a binary value.
    ///
    /// ```
    /// use bson::{
    ///     rawdoc,
    ///     spec::BinarySubtype,
    ///     Binary,
    /// };
    ///
    /// let doc = rawdoc! {
    ///     "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1, 2, 3] },
    ///     "bool": true,
    /// };
    ///
    /// assert_eq!(&doc.get_binary("binary")?.bytes, &[1, 2, 3]);
    /// assert!(doc.get_binary("bool").is_err());
    /// assert!(doc.get_binary("unknown").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_binary(&self, key: impl AsRef<str>) -> Result<RawBinaryRef<'_>> {
        self.get_with(key, ElementType::Binary, RawBsonRef::as_binary)
    }

    /// Gets a reference to the ObjectId value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't an ObjectId.
    ///
    /// ```
    /// # use bson::error::Error;
    /// use bson::{rawdoc, oid::ObjectId};
    ///
    /// let doc = rawdoc! {
    ///     "_id": ObjectId::new(),
    ///     "bool": true,
    /// };
    ///
    /// let oid = doc.get_object_id("_id")?;
    /// assert!(doc.get_object_id("bool").is_err());
    /// assert!(doc.get_object_id("unknown").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_object_id(&self, key: impl AsRef<str>) -> Result<ObjectId> {
        self.get_with(key, ElementType::ObjectId, RawBsonRef::as_object_id)
    }

    /// Gets a reference to the boolean value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a boolean.
    ///
    /// ```
    /// # use bson::error::Error;
    /// use bson::{rawdoc, oid::ObjectId};
    ///
    /// let doc = rawdoc! {
    ///     "_id": ObjectId::new(),
    ///     "bool": true,
    /// };
    ///
    /// assert!(doc.get_bool("bool")?);
    /// assert!(doc.get_bool("_id").is_err());
    /// assert!(doc.get_bool("unknown").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_bool(&self, key: impl AsRef<str>) -> Result<bool> {
        self.get_with(key, ElementType::Boolean, RawBsonRef::as_bool)
    }

    /// Gets a reference to the BSON DateTime value corresponding to a given key or returns an
    /// error if the key corresponds to a value which isn't a DateTime.
    ///
    /// ```
    /// # use bson::error::Error;
    /// use bson::{rawdoc, DateTime};
    ///
    /// let dt = DateTime::now();
    /// let doc = rawdoc! {
    ///     "created_at": dt,
    ///     "bool": true,
    /// };
    ///
    /// assert_eq!(doc.get_datetime("created_at")?, dt);
    /// assert!(doc.get_datetime("bool").is_err());
    /// assert!(doc.get_datetime("unknown").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_datetime(&self, key: impl AsRef<str>) -> Result<DateTime> {
        self.get_with(key, ElementType::DateTime, RawBsonRef::as_datetime)
    }

    /// Gets a reference to the BSON regex value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a regex.
    ///
    /// ```
    /// use bson::{rawdoc, Regex};
    ///
    /// let doc = rawdoc! {
    ///     "regex": Regex {
    ///         pattern: r"end\s*$".into(),
    ///         options: "i".into(),
    ///     },
    ///     "bool": true,
    /// };
    ///
    /// assert_eq!(doc.get_regex("regex")?.pattern, r"end\s*$");
    /// assert_eq!(doc.get_regex("regex")?.options, "i");
    /// assert!(doc.get_regex("bool").is_err());
    /// assert!(doc.get_regex("unknown").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_regex(&self, key: impl AsRef<str>) -> Result<RawRegexRef<'_>> {
        self.get_with(key, ElementType::RegularExpression, RawBsonRef::as_regex)
    }

    /// Gets a reference to the BSON timestamp value corresponding to a given key or returns an
    /// error if the key corresponds to a value which isn't a timestamp.
    ///
    /// ```
    /// # use bson::error::Error;
    /// use bson::{rawdoc, Timestamp};
    ///
    /// let doc = rawdoc! {
    ///     "bool": true,
    ///     "ts": Timestamp { time: 649876543, increment: 9 },
    /// };
    ///
    /// let timestamp = doc.get_timestamp("ts")?;
    ///
    /// assert_eq!(timestamp.time, 649876543);
    /// assert_eq!(timestamp.increment, 9);
    /// assert!(doc.get_timestamp("bool").is_err());
    /// assert!(doc.get_timestamp("unknown").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_timestamp(&self, key: impl AsRef<str>) -> Result<Timestamp> {
        self.get_with(key, ElementType::Timestamp, RawBsonRef::as_timestamp)
    }

    /// Gets a reference to the BSON int32 value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a 32-bit integer.
    ///
    /// ```
    /// # use bson::error::Error;
    /// use bson::rawdoc;
    ///
    /// let doc = rawdoc! {
    ///     "bool": true,
    ///     "i32": 1_000_000,
    /// };
    ///
    /// assert_eq!(doc.get_i32("i32")?, 1_000_000);
    /// assert!(doc.get_i32("bool").is_err());
    /// assert!(doc.get_i32("unknown").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_i32(&self, key: impl AsRef<str>) -> Result<i32> {
        self.get_with(key, ElementType::Int32, RawBsonRef::as_i32)
    }

    /// Gets a reference to the BSON int64 value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a 64-bit integer.
    ///
    /// ```
    /// # use bson::error::Error;
    /// use bson::rawdoc;
    ///
    /// let doc = rawdoc! {
    ///     "bool": true,
    ///     "i64": 9223372036854775807_i64,
    /// };
    ///
    /// assert_eq!(doc.get_i64("i64")?, 9223372036854775807);
    /// assert!(doc.get_i64("bool").is_err());
    /// assert!(doc.get_i64("unknown").is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn get_i64(&self, key: impl AsRef<str>) -> Result<i64> {
        self.get_with(key, ElementType::Int64, RawBsonRef::as_i64)
    }

    /// Return a reference to the contained data as a `&[u8]`
    ///
    /// ```
    /// # use bson::error::Error;
    /// use bson::rawdoc;
    /// let docbuf = rawdoc! {};
    /// assert_eq!(docbuf.as_bytes(), b"\x05\x00\x00\x00\x00");
    /// # Ok::<(), Error>(())
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Returns whether this document contains any elements or not.
    pub fn is_empty(&self) -> bool {
        self.as_bytes().len() == MIN_BSON_DOCUMENT_SIZE as usize
    }

    pub(crate) fn cstring_bytes_at(&self, start_at: usize) -> RawResult<&[u8]> {
        let buf = &self.as_bytes()[start_at..];

        let mut splits = buf.splitn(2, |x| *x == 0);
        let value = splits
            .next()
            .ok_or_else(|| RawError::malformed_bytes("no value"))?;
        if splits.next().is_some() {
            Ok(value)
        } else {
            Err(RawError::malformed_bytes("expected null terminator"))
        }
    }

    pub(crate) fn read_cstring_at(&self, start_at: usize) -> RawResult<&str> {
        let bytes = self.cstring_bytes_at(start_at)?;
        try_to_str(bytes)
    }

    /// Copy this into a [`Document`], returning an error if invalid BSON is encountered.
    pub fn to_document(&self) -> RawResult<Document> {
        self.try_into()
    }

    /// Copy this into a [`Document`], returning an error if invalid BSON is encountered.  Any
    /// invalid UTF-8 sequences will be replaced with the Unicode replacement character.
    pub fn to_document_utf8_lossy(&self) -> RawResult<Document> {
        let mut out = Document::new();
        for elem in self.iter_elements() {
            let elem = elem?;
            let value = deep_utf8_lossy(elem.value_utf8_lossy()?)?;
            out.insert(elem.key(), value);
        }
        Ok(out)
    }
}

fn deep_utf8_lossy(src: RawBson) -> RawResult<Bson> {
    match src {
        RawBson::Array(arr) => {
            let mut tmp = vec![];
            for elem in arr.iter_elements() {
                tmp.push(deep_utf8_lossy(elem?.value_utf8_lossy()?)?);
            }
            Ok(Bson::Array(tmp))
        }
        RawBson::Document(doc) => {
            let mut tmp = doc! {};
            for elem in doc.iter_elements() {
                let elem = elem?;
                tmp.insert(elem.key(), deep_utf8_lossy(elem.value_utf8_lossy()?)?);
            }
            Ok(Bson::Document(tmp))
        }
        RawBson::JavaScriptCodeWithScope(RawJavaScriptCodeWithScope { code, scope }) => {
            let mut tmp = doc! {};
            for elem in scope.iter_elements() {
                let elem = elem?;
                tmp.insert(elem.key(), deep_utf8_lossy(elem.value_utf8_lossy()?)?);
            }
            Ok(Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope {
                code,
                scope: tmp,
            }))
        }
        v => v.try_into(),
    }
}

#[cfg(feature = "serde")]
impl<'de: 'a, 'a> serde::Deserialize<'de> for &'a RawDocument {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use super::serde::OwnedOrBorrowedRawDocument;
        match OwnedOrBorrowedRawDocument::deserialize(deserializer)? {
            OwnedOrBorrowedRawDocument::Borrowed(b) => Ok(b),
            OwnedOrBorrowedRawDocument::Owned(d) => Err(serde::de::Error::custom(format!(
                "expected borrowed raw document, instead got owned {:?}",
                d
            ))),
        }
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for &RawDocument {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        struct KvpSerializer<'a>(&'a RawDocument);

        impl serde::Serialize for KvpSerializer<'_> {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                use serde::ser::SerializeMap as _;
                if serializer.is_human_readable() {
                    let mut map = serializer.serialize_map(None)?;
                    for kvp in self.0 {
                        let (k, v) = kvp.map_err(serde::ser::Error::custom)?;
                        map.serialize_entry(k, &v)?;
                    }
                    map.end()
                } else {
                    serializer.serialize_bytes(self.0.as_bytes())
                }
            }
        }
        serializer.serialize_newtype_struct(super::RAW_DOCUMENT_NEWTYPE, &KvpSerializer(self))
    }
}

impl std::fmt::Debug for RawDocument {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawDocument")
            .field("data", &hex::encode(&self.data))
            .finish()
    }
}

impl AsRef<RawDocument> for RawDocument {
    fn as_ref(&self) -> &RawDocument {
        self
    }
}

impl ToOwned for RawDocument {
    type Owned = RawDocumentBuf;

    fn to_owned(&self) -> Self::Owned {
        self.to_raw_document_buf()
    }
}

impl<'a> From<&'a RawDocument> for Cow<'a, RawDocument> {
    fn from(rdr: &'a RawDocument) -> Self {
        Cow::Borrowed(rdr)
    }
}

impl TryFrom<&RawDocument> for crate::Document {
    type Error = RawError;

    fn try_from(rawdoc: &RawDocument) -> RawResult<Document> {
        rawdoc
            .into_iter()
            .map(|res| res.and_then(|(k, v)| Ok((k.to_owned(), v.try_into()?))))
            .collect()
    }
}

impl TryFrom<RawDocumentBuf> for Document {
    type Error = crate::error::Error;

    fn try_from(raw: RawDocumentBuf) -> Result<Document> {
        Document::try_from(raw.as_ref())
    }
}

impl TryFrom<&RawDocumentBuf> for Document {
    type Error = crate::error::Error;

    fn try_from(raw: &RawDocumentBuf) -> Result<Document> {
        Document::try_from(raw.as_ref())
    }
}

impl<'a> IntoIterator for &'a RawDocument {
    type IntoIter = Iter<'a>;
    type Item = RawResult<(&'a str, RawBsonRef<'a>)>;

    fn into_iter(self) -> Iter<'a> {
        self.iter()
    }
}
