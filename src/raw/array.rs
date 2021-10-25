use std::convert::TryFrom;

use serde::Deserialize;

use super::{
    error::{ValueAccessError, ValueAccessErrorKind, ValueAccessResult},
    Error,
    Iter,
    RawBinary,
    RawBson,
    RawDocument,
    RawRegex,
    Result,
};
use crate::{oid::ObjectId, spec::ElementType, Bson, DateTime, Timestamp};

/// A slice of a BSON document containing a BSON array value (akin to [`std::str`]). This can be
/// retrieved from a [`RawDocument`] via [`RawDocument::get`].
///
/// This is an _unsized_ type, meaning that it must always be used behind a pointer like `&`.
///
/// Accessing elements within a [`RawArray`] is similar to element access in [`crate::Document`],
/// but because the contents are parsed during iteration instead of at creation time, format errors
/// can happen at any time during use.
///
/// Iterating over a [`RawArray`] yields either an error or a value that borrows from the
/// original document without making any additional allocations.
///
/// ```
/// use bson::{doc, raw::RawDocument};
///
/// let doc = doc! {
///     "x": [1, true, "two", 5.5]
/// };
/// let bytes = bson::to_vec(&doc)?;
///
/// let rawdoc = RawDocument::new(bytes.as_slice())?;
/// let rawarray = rawdoc.get_array("x")?;
///
/// for v in rawarray {
///     println!("{:?}", v?);
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// Individual elements can be accessed using [`RawArray::get`] or any of
/// the type-specific getters, such as [`RawArray::get_object_id`] or
/// [`RawArray::get_str`]. Note that accessing elements is an O(N) operation, as it
/// requires iterating through the array from the beginning to find the requested index.
///
/// ```
/// # use bson::raw::{ValueAccessError};
/// use bson::{doc, raw::RawDocument};
///
/// let doc = doc! {
///     "x": [1, true, "two", 5.5]
/// };
/// let bytes = bson::to_vec(&doc)?;
///
/// let rawdoc = RawDocument::new(bytes.as_slice())?;
/// let rawarray = rawdoc.get_array("x")?;
///
/// assert_eq!(rawarray.get_bool(1)?, true);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(PartialEq)]
#[repr(transparent)]
pub struct RawArray {
    pub(crate) doc: RawDocument,
}

impl RawArray {
    pub(crate) fn from_doc(doc: &RawDocument) -> &RawArray {
        // SAFETY:
        //
        // Dereferencing a raw pointer requires unsafe due to the potential that the pointer is
        // null, dangling, or misaligned. We know the pointer is not null or dangling due to the
        // fact that it's created by a safe reference. Converting &RawDocument to *const
        // RawDocument will be properly aligned due to them being references to the same type,
        // and converting *const RawDocument to *const RawArray is aligned due to the fact that
        // the only field in a RawArray is a RawDocument, meaning the structs are represented
        // identically at the byte level.
        unsafe { &*(doc as *const RawDocument as *const RawArray) }
    }

    /// Gets a reference to the value at the given index.
    pub fn get(&self, index: usize) -> Result<Option<RawBson<'_>>> {
        self.into_iter().nth(index).transpose()
    }

    fn get_with<'a, T>(
        &'a self,
        index: usize,
        expected_type: ElementType,
        f: impl FnOnce(RawBson<'a>) -> Option<T>,
    ) -> ValueAccessResult<T> {
        let bson = self
            .get(index)
            .map_err(|e| ValueAccessError {
                key: index.to_string(),
                kind: ValueAccessErrorKind::InvalidBson(e),
            })?
            .ok_or(ValueAccessError {
                key: index.to_string(),
                kind: ValueAccessErrorKind::NotPresent,
            })?;
        match f(bson) {
            Some(t) => Ok(t),
            None => Err(ValueAccessError {
                key: index.to_string(),
                kind: ValueAccessErrorKind::UnexpectedType {
                    expected: expected_type,
                    actual: bson.element_type(),
                },
            }),
        }
    }

    /// Gets the BSON double at the given index or returns an error if the value at that index isn't
    /// a double.
    pub fn get_f64(&self, index: usize) -> ValueAccessResult<f64> {
        self.get_with(index, ElementType::Double, RawBson::as_f64)
    }

    /// Gets a reference to the string at the given index or returns an error if the
    /// value at that index isn't a string.
    pub fn get_str(&self, index: usize) -> ValueAccessResult<&str> {
        self.get_with(index, ElementType::String, RawBson::as_str)
    }

    /// Gets a reference to the document at the given index or returns an error if the
    /// value at that index isn't a document.
    pub fn get_document(&self, index: usize) -> ValueAccessResult<&RawDocument> {
        self.get_with(index, ElementType::EmbeddedDocument, RawBson::as_document)
    }

    /// Gets a reference to the array at the given index or returns an error if the
    /// value at that index isn't a array.
    pub fn get_array(&self, index: usize) -> ValueAccessResult<&RawArray> {
        self.get_with(index, ElementType::Array, RawBson::as_array)
    }

    /// Gets a reference to the BSON binary value at the given index or returns an error if the
    /// value at that index isn't a binary.
    pub fn get_binary(&self, index: usize) -> ValueAccessResult<RawBinary<'_>> {
        self.get_with(index, ElementType::Binary, RawBson::as_binary)
    }

    /// Gets the ObjectId at the given index or returns an error if the value at that index isn't an
    /// ObjectId.
    pub fn get_object_id(&self, index: usize) -> ValueAccessResult<ObjectId> {
        self.get_with(index, ElementType::ObjectId, RawBson::as_object_id)
    }

    /// Gets the boolean at the given index or returns an error if the value at that index isn't a
    /// boolean.
    pub fn get_bool(&self, index: usize) -> ValueAccessResult<bool> {
        self.get_with(index, ElementType::Boolean, RawBson::as_bool)
    }

    /// Gets the DateTime at the given index or returns an error if the value at that index isn't a
    /// DateTime.
    pub fn get_datetime(&self, index: usize) -> ValueAccessResult<DateTime> {
        self.get_with(index, ElementType::DateTime, RawBson::as_datetime)
    }

    /// Gets a reference to the BSON regex at the given index or returns an error if the
    /// value at that index isn't a regex.
    pub fn get_regex(&self, index: usize) -> ValueAccessResult<RawRegex<'_>> {
        self.get_with(index, ElementType::RegularExpression, RawBson::as_regex)
    }

    /// Gets a reference to the BSON timestamp at the given index or returns an error if the
    /// value at that index isn't a timestamp.
    pub fn get_timestamp(&self, index: usize) -> ValueAccessResult<Timestamp> {
        self.get_with(index, ElementType::Timestamp, RawBson::as_timestamp)
    }

    /// Gets the BSON int32 at the given index or returns an error if the value at that index isn't
    /// a 32-bit integer.
    pub fn get_i32(&self, index: usize) -> ValueAccessResult<i32> {
        self.get_with(index, ElementType::Int32, RawBson::as_i32)
    }

    /// Gets BSON int64 at the given index or returns an error if the value at that index isn't a
    /// 64-bit integer.
    pub fn get_i64(&self, index: usize) -> ValueAccessResult<i64> {
        self.get_with(index, ElementType::Int64, RawBson::as_i64)
    }

    /// Gets a reference to the raw bytes of the [`RawArray`].
    pub fn as_bytes(&self) -> &[u8] {
        self.doc.as_bytes()
    }
}

impl std::fmt::Debug for RawArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawArray")
            .field("data", &hex::encode(self.doc.as_bytes()))
            .finish()
    }
}

impl TryFrom<&RawArray> for Vec<Bson> {
    type Error = Error;

    fn try_from(arr: &RawArray) -> Result<Vec<Bson>> {
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
    type Item = Result<RawBson<'a>>;

    fn into_iter(self) -> RawArrayIter<'a> {
        RawArrayIter {
            inner: self.doc.into_iter(),
        }
    }
}

/// An iterator over borrowed raw BSON array values.
pub struct RawArrayIter<'a> {
    inner: Iter<'a>,
}

impl<'a> Iterator for RawArrayIter<'a> {
    type Item = Result<RawBson<'a>>;

    fn next(&mut self) -> Option<Result<RawBson<'a>>> {
        match self.inner.next() {
            Some(Ok((_, v))) => Some(Ok(v)),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for &'a RawArray {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawBson::deserialize(deserializer)? {
            RawBson::Array(d) => Ok(d),
            b => Err(serde::de::Error::custom(format!(
                "expected raw array reference, instead got {:?}",
                b
            ))),
        }
    }
}
