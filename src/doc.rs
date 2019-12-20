//! A BSON document represented as an associative HashMap with insertion ordering.
use std::error;
use std::fmt::{self, Debug, Display, Formatter};
use std::iter::{Extend, FromIterator};
use std::marker::PhantomData;
use std::cmp::Ordering;
use std::ops::RangeFull;
use std::io::{Write, Read};

use chrono::{DateTime, Utc};

use indexmap::IndexMap;
pub use indexmap::map::{Keys, Values, IntoIter, Iter, IterMut, ValuesMut, Drain, Entry};

use serde::de::{self, MapAccess, Visitor};

use crate::bson::{Array, Bson};
#[cfg(feature = "decimal128")]
use crate::decimal128::Decimal128;
use crate::oid::ObjectId;
use crate::spec::BinarySubtype;
use crate::encoder::{encode_document, EncoderResult};
use crate::decoder::{decode_document, decode_document_utf8_lossy, DecoderResult};

/// Error to indicate that either a value was empty or it contained an unexpected
/// type, for use with the direct getters.
#[derive(PartialEq)]
pub enum ValueAccessError {
    /// Cannot find the expected field with the specified key
    NotPresent,
    /// Found a Bson value with the specified key, but not with the expected type
    UnexpectedType,
}

/// Result of accessing Bson value
pub type ValueAccessResult<T> = Result<T, ValueAccessError>;

impl Debug for ValueAccessError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            ValueAccessError::NotPresent => write!(f, "ValueAccessError: field is not present"),
            ValueAccessError::UnexpectedType => write!(f, "ValueAccessError: field does not have the expected type"),
        }
    }
}

impl Display for ValueAccessError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            ValueAccessError::NotPresent => write!(f, "field is not present"),
            ValueAccessError::UnexpectedType => write!(f, "field does not have the expected type"),
        }
    }
}

impl error::Error for ValueAccessError {
    fn description(&self) -> &str {
        "Error to indicate that either a value was empty or it contained an unexpected type"
    }
}

#[deprecated(since = "0.15.0", note = "use Document instead")]
pub type OrderedDocument = Document;

/// A BSON document represented as an associative HashMap with insertion ordering.
#[derive(Clone, PartialEq)]
pub struct Document {
    inner: IndexMap<String, Bson>,
}

impl Default for Document {
    fn default() -> Self {
        Document::new()
    }
}

impl Display for Document {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        fmt.write_str("{")?;

        let mut first = true;
        for (k, v) in self {
            if first {
                first = false;
                fmt.write_str(" ")?;
            } else {
                fmt.write_str(", ")?;
            }

            write!(fmt, "{}: {}", k, v)?;
        }

        write!(fmt, "{}}}", if !first { " " } else { "" })
    }
}

impl Debug for Document {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "OrderedDocument({:?})", self.inner)
    }
}

#[deprecated(since = "0.15.0", note = "use IntoIter instead")]
pub type OrderedDocumentIntoIterator = IntoIter<String, Bson>;
#[deprecated(since = "0.15.0", note = "use Iter instead")]
pub type OrderedDocumentIterator<'a> = Iter<'a, String, Bson>;

impl IntoIterator for Document {
    type Item = (String, Bson);
    type IntoIter = IntoIter<String, Bson>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a Document {
    type Item = (&'a String, &'a Bson);
    type IntoIter = Iter<'a, String, Bson>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl<'a> IntoIterator for &'a mut Document {
    type Item = (&'a String, &'a mut Bson);
    type IntoIter = IterMut<'a, String, Bson>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter_mut()
    }
}

impl FromIterator<(String, Bson)> for Document {
    fn from_iter<T: IntoIterator<Item = (String, Bson)>>(iter: T) -> Self {
        let mut doc = Document::new();
        for (k, v) in iter {
            doc.insert(k, v);
        }
        doc
    }
}

impl Document {
    /// Creates a new empty Document.
    pub fn new() -> Document {
        Document { inner: IndexMap::new() }
    }

    /// Create a new Document with capacity. (Does not allocate if n is zero.)
    /// Computes in O(n) time.
    pub fn with_capacity(n: usize) -> Document {
        Document {
            inner: IndexMap::with_capacity(n)
        }
    }

    /// Computes in O(1) time.
    pub fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    /// Gets an iterator over the entries of the map.
    pub fn iter<'a>(&'a self) -> Iter<'a, String, Bson> {
        self.into_iter()
    }

    /// Gets an iterator over the entries of the map.
    pub fn iter_mut(&mut self) -> IterMut<'_, String, Bson> {
        self.into_iter()
    }

    /// Clears the document, removing all values.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Reserve capacity for `additional` more key-value pairs.
    ///
    /// FIXME Not implemented fully yet.
    pub fn reserve(&mut self, additional: usize) {
        self.inner.reserve(additional);
    }

    /// Returns a reference to the Bson corresponding to the key.
    pub fn get(&self, key: &str) -> Option<&Bson> {
        self.inner.get(key)
    }

    /// Returns a reference to the Bson corresponding to the key.
    pub fn get_full(&self, key: &str) -> Option<(usize, &String, &Bson)> {
        self.inner.get_full(key)
    }

    /// Gets a mutable reference to the Bson corresponding to the key
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Bson> {
        self.inner.get_mut(key)
    }

    /// Gets a mutable reference to the Bson corresponding to the key
    pub fn get_mut_full(&mut self, key: &str) -> Option<(usize, &String, &mut Bson)> {
        self.inner.get_full_mut(key)
    }

    /// Get a floating point value for this key if it exists and has
    /// the correct type.
    pub fn get_f64(&self, key: &str) -> ValueAccessResult<f64> {
        match self.get(key) {
            Some(&Bson::FloatingPoint(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a floating point value for this key if it exists and has
    /// the correct type.
    pub fn get_f64_mut(&mut self, key: &str) -> ValueAccessResult<&mut f64> {
        match self.get_mut(key) {
            Some(&mut Bson::FloatingPoint(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a reference to a Decimal128 value for key, if it exists.
    #[cfg(feature = "decimal128")]
    pub fn get_decimal128(&self, key: &str) -> ValueAccessResult<&Decimal128> {
        match self.get(key) {
            Some(&Bson::Decimal128(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a Decimal128 value for key, if it exists.
    #[cfg(feature = "decimal128")]
    pub fn get_decimal128_mut(&mut self, key: &str) -> ValueAccessResult<&mut Decimal128> {
        match self.get_mut(key) {
            Some(&mut Bson::Decimal128(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a string slice this key if it exists and has the correct type.
    pub fn get_str(&self, key: &str) -> ValueAccessResult<&str> {
        match self.get(key) {
            Some(&Bson::String(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable string slice this key if it exists and has the correct type.
    pub fn get_str_mut(&mut self, key: &str) -> ValueAccessResult<&mut str> {
        match self.get_mut(key) {
            Some(&mut Bson::String(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a reference to an array for this key if it exists and has
    /// the correct type.
    pub fn get_array(&self, key: &str) -> ValueAccessResult<&Array> {
        match self.get(key) {
            Some(&Bson::Array(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to an array for this key if it exists and has
    /// the correct type.
    pub fn get_array_mut(&mut self, key: &str) -> ValueAccessResult<&mut Array> {
        match self.get_mut(key) {
            Some(&mut Bson::Array(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a reference to a document for this key if it exists and has
    /// the correct type.
    pub fn get_document(&self, key: &str) -> ValueAccessResult<&Document> {
        match self.get(key) {
            Some(&Bson::Document(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a document for this key if it exists and has
    /// the correct type.
    pub fn get_document_mut(&mut self, key: &str) -> ValueAccessResult<&mut Document> {
        match self.get_mut(key) {
            Some(&mut Bson::Document(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a bool value for this key if it exists and has the correct type.
    pub fn get_bool(&self, key: &str) -> ValueAccessResult<bool> {
        match self.get(key) {
            Some(&Bson::Boolean(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a bool value for this key if it exists and has the correct type.
    pub fn get_bool_mut(&mut self, key: &str) -> ValueAccessResult<&mut bool> {
        match self.get_mut(key) {
            Some(&mut Bson::Boolean(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Returns wether this key has a null value
    pub fn is_null(&self, key: &str) -> bool {
        self.get(key) == Some(&Bson::Null)
    }

    /// Get an i32 value for this key if it exists and has the correct type.
    pub fn get_i32(&self, key: &str) -> ValueAccessResult<i32> {
        match self.get(key) {
            Some(&Bson::I32(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to an i32 value for this key if it exists and has the correct type.
    pub fn get_i32_mut(&mut self, key: &str) -> ValueAccessResult<&mut i32> {
        match self.get_mut(key) {
            Some(&mut Bson::I32(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get an i64 value for this key if it exists and has the correct type.
    pub fn get_i64(&self, key: &str) -> ValueAccessResult<i64> {
        match self.get(key) {
            Some(&Bson::I64(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to an i64 value for this key if it exists and has the correct type.
    pub fn get_i64_mut(&mut self, key: &str) -> ValueAccessResult<&mut i64> {
        match self.get_mut(key) {
            Some(&mut Bson::I64(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a time stamp value for this key if it exists and has the correct type.
    pub fn get_time_stamp(&self, key: &str) -> ValueAccessResult<i64> {
        match self.get(key) {
            Some(&Bson::TimeStamp(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a time stamp value for this key if it exists and has the correct type.
    pub fn get_time_stamp_mut(&mut self, key: &str) -> ValueAccessResult<&mut i64> {
        match self.get_mut(key) {
            Some(&mut Bson::TimeStamp(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a reference to a generic binary value for this key if it exists and has the correct type.
    pub fn get_binary_generic(&self, key: &str) -> ValueAccessResult<&Vec<u8>> {
        match self.get(key) {
            Some(&Bson::Binary(BinarySubtype::Generic, ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference generic binary value for this key if it exists and has the correct type.
    pub fn get_binary_generic_mut(&mut self, key: &str) -> ValueAccessResult<&mut Vec<u8>> {
        match self.get_mut(key) {
            Some(&mut Bson::Binary(BinarySubtype::Generic, ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a reference to an object id value for this key if it exists and has the correct type.
    pub fn get_object_id(&self, key: &str) -> ValueAccessResult<&ObjectId> {
        match self.get(key) {
            Some(&Bson::ObjectId(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to an object id value for this key if it exists and has the correct type.
    pub fn get_object_id_mut(&mut self, key: &str) -> ValueAccessResult<&mut ObjectId> {
        match self.get_mut(key) {
            Some(&mut Bson::ObjectId(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a reference to a UTC datetime value for this key if it exists and has the correct type.
    pub fn get_utc_datetime(&self, key: &str) -> ValueAccessResult<&DateTime<Utc>> {
        match self.get(key) {
            Some(&Bson::UtcDatetime(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a UTC datetime value for this key if it exists and has the correct type.
    pub fn get_utc_datetime_mut(&mut self, key: &str) -> ValueAccessResult<&mut DateTime<Utc>> {
        match self.get_mut(key) {
            Some(&mut Bson::UtcDatetime(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Returns true if the map contains a value for the specified key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    /// Gets a collection of all keys in the document.
    pub fn keys(&self) -> Keys<String, Bson> {
        self.inner.keys()
    }

    /// Gets a collection of all values in the document.
    pub fn value(&self) -> Values<String, Bson> {
        self.inner.values()
    }

    /// Returns the number of elements in the document.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns true if the document contains no elements
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Sets the value of the entry with the OccupiedEntry's key,
    /// and returns the entry's old value. Accepts any type that
    /// can be converted into Bson.
    pub fn insert<KT: Into<String>, BT: Into<Bson>>(&mut self, key: KT, val: BT) -> Option<Bson> {
        self.insert_bson(key.into(), val.into())
    }

    /// Sets the value of the entry with the OccupiedEntry's key,
    /// and returns the entry's old value.
    pub fn insert_bson(&mut self, key: String, val: Bson) -> Option<Bson> {
        self.inner.insert(key, val)
    }

    /// Sets the value of the entry with the OccupiedEntry's key,
    /// and returns the entry's old value.
    pub fn insert_bson_full(&mut self, key: String, value: Bson) -> (usize, Option<Bson>) {
        self.inner.insert_full(key, value)
    }

    /// Takes the value of the entry out of the document, and returns it.
    ///
    /// **NOTE:** This is equivalent to `.swap_remove(key)`, if you need to
    /// preserve the order of the keys in the map, use `.shift_remove(key)`
    /// instead.
    ///
    /// Computes in **O(1)** time (average).
    pub fn remove(&mut self, key: &str) -> Option<Bson> {
        self.inner.remove(key)
    }

    /// Takes the value of the entry out of the document, and returns it.
    ///
    /// Like `Vec::swap_remove`, the pair is removed by swapping it with the
    /// last element of the map and popping it off. **This perturbs
    /// the postion of what used to be the last element!**
    ///
    /// Return `None` if `key` is not in map.
    ///
    /// Computes in **O(1)** time (average).
    pub fn swap_remove(&mut self, key: &str) -> Option<Bson> {
        self.inner.swap_remove(key)
    }

    /// Takes the value of the entry out of the document, and returns it.
    ///
    /// Like `Vec::swap_remove`, the pair is removed by swapping it with the
    /// last element of the map and popping it off. **This perturbs
    /// the postion of what used to be the last element!**
    ///
    /// Return `None` if `key` is not in map.
    ///
    /// Computes in **O(1)** time (average).
    pub fn swap_remove_full(&mut self, key: &str) -> Option<(usize, String, Bson)> {
        self.inner.swap_remove_full(key)
    }

    /// Takes the value of the entry out of the document, and returns it.
    ///
    /// Like `Vec::remove`, the pair is removed by shifting all of the
    /// elements that follow it, preserving their relative order.
    /// **This perturbs the index of all of those elements!**
    ///
    /// Return `None` if `key` is not in map.
    ///
    /// Computes in **O(n)** time (average).
    pub fn shift_remove(&mut self, key: &str) -> Option<Bson> {
        self.inner.shift_remove(key)
    }

    /// Takes the value of the entry out of the document, and returns it.
    ///
    /// Like `Vec::remove`, the pair is removed by shifting all of the
    /// elements that follow it, preserving their relative order.
    /// **This perturbs the index of all of those elements!**
    ///
    /// Return `None` if `key` is not in map.
    ///
    /// Computes in **O(n)** time (average).
    pub fn shift_remove_full(&mut self, key: &str) -> Option<(usize, String, Bson)> {
        self.inner.shift_remove_full(key)
    }

    /// Remove the last key-value pair
    /// Computes in O(1) time (average).
    pub fn pop(&mut self) -> Option<(String, Bson)> {
        self.inner.pop()
    }

    /// Scan through each key-value pair in the map and keep those where the
    /// closure `keep` returns `true`.
    ///
    /// The elements are visited in order, and remaining elements keep their
    /// order.
    ///
    /// Computes in **O(n)** time (average).
    pub fn retain<F>(&mut self, keep: F)
        where F: FnMut(&String, &mut Bson) -> bool
    {
        self.inner.retain(keep)
    }

    /// Sort the map’s key-value pairs by the default ordering of the keys.
    ///
    /// See `sort_by` for details.
    pub fn sort_keys(&mut self) {
        self.inner.sort_keys()
    }

     /// Sort the map’s key-value pairs in place using the comparison
    /// function `compare`.
    ///
    /// The comparison function receives two key and value pairs to compare (you
    /// can sort by keys or values or their combination as needed).
    ///
    /// Computes in **O(n log n + c)** time and **O(n)** space where *n* is
    /// the length of the map and *c* the capacity. The sort is stable.
    pub fn sort_by<F>(&mut self, compare: F)
        where F: FnMut(&String, &Bson, &String, &Bson) -> Ordering
    {
        self.inner.sort_by(compare)
    }

    /// Sort the key-value pairs of the map and return a by value iterator of
    /// the key-value pairs with the result.
    ///
    /// The sort is stable.
    pub fn sorted_by<F>(self, compare: F) -> IntoIter<String, Bson>
        where F: FnMut(&String, &Bson, &String, &Bson) -> Ordering
    {
        self.inner.sorted_by(compare)
    }

    /// Clears the `IndexMap`, returning all key-value pairs as a drain iterator.
    /// Keeps the allocated memory for reuse.
    pub fn drain(&mut self, range: RangeFull) -> Drain<String, Bson> {
        self.inner.drain(range)
    }

    /// Get the given key’s corresponding entry in the map for insertion and/or
    /// in-place manipulation.
    ///
    /// Computes in **O(1)** time (amortized average).
    pub fn entry(&mut self, k: String) -> Entry<String, Bson> {
        self.inner.entry(k)
    }

    pub fn extend(&mut self, iter: impl Into<Document>) {
        self.inner.extend(iter.into());
    }

    /// Get a key-value pair by index
    ///
    /// Valid indices are *0 <= index < self.len()*
    ///
    /// Computes in **O(1)** time.
    pub fn get_index(&self, index: usize) -> Option<(&String, &Bson)> {
        self.inner.get_index(index)
    }

    /// Get a key-value pair by index
    ///
    /// Valid indices are *0 <= index < self.len()*
    ///
    /// Computes in **O(1)** time.
    pub fn get_index_mut(&mut self, index: usize) -> Option<(&mut String, &mut Bson)> {
        self.inner.get_index_mut(index)
    }

    /// Remove the key-value pair by index
    ///
    /// Valid indices are *0 <= index < self.len()*
    ///
    /// Like `Vec::swap_remove`, the pair is removed by swapping it with the
    /// last element of the map and popping it off. **This perturbs
    /// the postion of what used to be the last element!**
    ///
    /// Computes in **O(1)** time (average).
    pub fn swap_remove_index(&mut self, index: usize) -> Option<(String, Bson)> {
        self.inner.swap_remove_index(index)
    }

    /// Remove the key-value pair by index
    ///
    /// Valid indices are *0 <= index < self.len()*
    ///
    /// Like `Vec::remove`, the pair is removed by shifting all of the
    /// elements that follow it, preserving their relative order.
    /// **This perturbs the index of all of those elements!**
    ///
    /// Computes in **O(n)** time (average).
    pub fn shift_remove_index(&mut self, index: usize) -> Option<(String, Bson)> {
        self.inner.shift_remove_index(index)
    }

    /// Attempt to encode a Document into a byte stream.
    pub fn encode(&self, writer: &mut impl Write) -> EncoderResult<()> {
        encode_document(writer, self)
    }

    /// Attempt to decode a Document from a byte stream.
    pub fn decode(reader: &mut impl Read) -> DecoderResult<Document> {
        decode_document(reader)
    }

    /// Attempt to decode a Document that may contain invalid UTF-8 strings from a byte stream.
    pub fn decode_utf8_lossy(reader: &mut impl Read) -> DecoderResult<Document> {
        decode_document_utf8_lossy(reader)
    }
}

impl From<IndexMap<String, Bson>> for Document {
    fn from(tree: IndexMap<String, Bson>) -> Document {
        Document { inner: tree }
    }
}

#[deprecated(since = "0.15.0", note = "use DocumentVisitor instead")]
pub type OrderedDocumentVisitor = DocumentVisitor;

pub struct DocumentVisitor {
    marker: PhantomData<Document>,
}

impl DocumentVisitor {
    pub fn new() -> DocumentVisitor {
        DocumentVisitor { marker: PhantomData }
    }
}

impl<'de> Visitor<'de> for DocumentVisitor {
    type Value = Document;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "expecting ordered document")
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Document, E>
        where E: de::Error
    {
        Ok(Document::new())
    }

    #[inline]
    fn visit_map<V>(self, mut visitor: V) -> Result<Document, V::Error>
        where V: MapAccess<'de>
    {
        let mut inner = match visitor.size_hint() {
            Some(size) => IndexMap::with_capacity(size),
            None => IndexMap::new(),
        };

        while let Some((key, value)) = visitor.next_entry()? {
            inner.insert(key, value);
        }

        Ok(inner.into())
    }
}

impl Extend<(String, Bson)> for Document {
    fn extend<T: IntoIterator<Item = (String, Bson)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}
