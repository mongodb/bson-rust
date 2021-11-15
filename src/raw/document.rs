use std::{
    borrow::Cow,
    convert::{TryFrom, TryInto},
};

use serde::{ser::SerializeMap, Deserialize, Serialize};

use crate::{
    raw::{error::ErrorKind, RawBsonVisitor, RAW_DOCUMENT_NEWTYPE},
    spec::BinarySubtype,
    DateTime,
    Timestamp,
};

use super::{
    error::{ValueAccessError, ValueAccessErrorKind, ValueAccessResult},
    i32_from_slice,
    Error,
    Iter,
    RawArray,
    RawBinary,
    RawBson,
    RawDocumentBuf,
    RawRegex,
    Result,
};
use crate::{oid::ObjectId, spec::ElementType, Document};

/// A slice of a BSON document (akin to [`std::str`]). This can be created from a
/// [`RawDocumentBuf`] or any type that contains valid BSON data, including static binary literals,
/// [Vec<u8>](std::vec::Vec), or arrays.
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
///
/// Individual elements can be accessed using [`RawDocument::get`] or any of
/// the type-specific getters, such as [`RawDocument::get_object_id`] or
/// [`RawDocument::get_str`]. Note that accessing elements is an O(N) operation, as it
/// requires iterating through the document from the beginning to find the requested key.
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
    pub fn new<D: AsRef<[u8]> + ?Sized>(data: &D) -> Result<&RawDocument> {
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

        Ok(RawDocument::new_unchecked(data))
    }

    /// Creates a new `RawDocument` referencing the provided data slice.
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

    /// Creates a new [`RawDocument`] with an owned copy of the BSON bytes.
    pub fn to_raw_document_buf(&self) -> RawDocumentBuf {
        // unwrap is ok here because we already verified the bytes in `RawDocumentRef::new`
        RawDocumentBuf::new(self.data.to_owned()).unwrap()
    }

    /// Gets a reference to the value corresponding to the given key by iterating until the key is
    /// found.
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
    pub fn get_f64(&self, key: impl AsRef<str>) -> ValueAccessResult<f64> {
        self.get_with(key, ElementType::Double, RawBson::as_f64)
    }

    /// Gets a reference to the string value corresponding to a given key or returns an error if the
    /// key corresponds to a value which isn't a string.
    pub fn get_str(&self, key: impl AsRef<str>) -> ValueAccessResult<&'_ str> {
        self.get_with(key, ElementType::String, RawBson::as_str)
    }

    /// Gets a reference to the document value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a document.
    pub fn get_document(&self, key: impl AsRef<str>) -> ValueAccessResult<&'_ RawDocument> {
        self.get_with(key, ElementType::EmbeddedDocument, RawBson::as_document)
    }

    /// Gets a reference to the array value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't an array.
    pub fn get_array(&self, key: impl AsRef<str>) -> ValueAccessResult<&'_ RawArray> {
        self.get_with(key, ElementType::Array, RawBson::as_array)
    }

    /// Gets a reference to the BSON binary value corresponding to a given key or returns an error
    /// if the key corresponds to a value which isn't a binary value.
    pub fn get_binary(&self, key: impl AsRef<str>) -> ValueAccessResult<RawBinary<'_>> {
        self.get_with(key, ElementType::Binary, RawBson::as_binary)
    }

    /// Gets a reference to the ObjectId value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't an ObjectId.
    pub fn get_object_id(&self, key: impl AsRef<str>) -> ValueAccessResult<ObjectId> {
        self.get_with(key, ElementType::ObjectId, RawBson::as_object_id)
    }

    /// Gets a reference to the boolean value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a boolean.
    pub fn get_bool(&self, key: impl AsRef<str>) -> ValueAccessResult<bool> {
        self.get_with(key, ElementType::Boolean, RawBson::as_bool)
    }

    /// Gets a reference to the BSON DateTime value corresponding to a given key or returns an
    /// error if the key corresponds to a value which isn't a DateTime.
    pub fn get_datetime(&self, key: impl AsRef<str>) -> ValueAccessResult<DateTime> {
        self.get_with(key, ElementType::DateTime, RawBson::as_datetime)
    }

    /// Gets a reference to the BSON regex value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a regex.
    pub fn get_regex(&self, key: impl AsRef<str>) -> ValueAccessResult<RawRegex<'_>> {
        self.get_with(key, ElementType::RegularExpression, RawBson::as_regex)
    }

    /// Gets a reference to the BSON timestamp value corresponding to a given key or returns an
    /// error if the key corresponds to a value which isn't a timestamp.
    pub fn get_timestamp(&self, key: impl AsRef<str>) -> ValueAccessResult<Timestamp> {
        self.get_with(key, ElementType::Timestamp, RawBson::as_timestamp)
    }

    /// Gets a reference to the BSON int32 value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a 32-bit integer.
    pub fn get_i32(&self, key: impl AsRef<str>) -> ValueAccessResult<i32> {
        self.get_with(key, ElementType::Int32, RawBson::as_i32)
    }

    /// Gets a reference to the BSON int64 value corresponding to a given key or returns an error if
    /// the key corresponds to a value which isn't a 64-bit integer.
    pub fn get_i64(&self, key: impl AsRef<str>) -> ValueAccessResult<i64> {
        self.get_with(key, ElementType::Int64, RawBson::as_i64)
    }

    /// Return a reference to the contained data as a `&[u8]`
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for &'a RawDocument {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match deserializer.deserialize_newtype_struct(RAW_DOCUMENT_NEWTYPE, RawBsonVisitor)? {
            RawBson::Document(d) => Ok(d),

            // For non-BSON formats, RawDocument gets serialized as bytes, so we need to deserialize
            // from them here too. For BSON, the deserializier will return an error if it
            // sees the RAW_DOCUMENT_NEWTYPE but the next type isn't a document.
            RawBson::Binary(b) if b.subtype == BinarySubtype::Generic => {
                RawDocument::new(b.bytes).map_err(serde::de::Error::custom)
            }

            o => Err(serde::de::Error::custom(format!(
                "expected raw document reference, instead got {:?}",
                o
            ))),
        }
    }
}

impl<'a> Serialize for &'a RawDocument {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        struct KvpSerializer<'a>(&'a RawDocument);

        impl<'a> Serialize for KvpSerializer<'a> {
            fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
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
        serializer.serialize_newtype_struct(RAW_DOCUMENT_NEWTYPE, &KvpSerializer(self))
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
    type Error = Error;

    fn try_from(rawdoc: &RawDocument) -> Result<Document> {
        rawdoc
            .into_iter()
            .map(|res| res.and_then(|(k, v)| Ok((k.to_owned(), v.try_into()?))))
            .collect()
    }
}

impl<'a> IntoIterator for &'a RawDocument {
    type IntoIter = Iter<'a>;
    type Item = Result<(&'a str, RawBson<'a>)>;

    fn into_iter(self) -> Iter<'a> {
        Iter::new(self)
    }
}
