//! A BSON document represented as an associative HashMap with insertion ordering.

use std::{
    error,
    fmt::{self, Debug, Display, Formatter},
    io::{Read, Write},
    iter::{Extend, FromIterator, IntoIterator, Map},
    marker::PhantomData,
    mem,
};

use byteorder::{ReadBytesExt, WriteBytesExt};

use chrono::{DateTime, Utc};

use linked_hash_map::{self, LinkedHashMap};

use serde::de::{self, MapAccess, Visitor};

#[cfg(feature = "decimal128")]
use crate::decimal128::Decimal128;
use crate::{
    bson::{Array, Binary, Bson, TimeStamp},
    decoder::{decode_bson_kvp, read_i32, DecoderResult},
    encoder::{encode_bson, write_i32, EncoderResult},
    oid::ObjectId,
    spec::BinarySubtype,
};

/// Error to indicate that either a value was empty or it contained an unexpected
/// type, for use with the direct getters.
#[derive(PartialEq)]
#[non_exhaustive]
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
            ValueAccessError::UnexpectedType => {
                write!(f, "ValueAccessError: field does not have the expected type")
            }
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

impl error::Error for ValueAccessError {}

/// A BSON document represented as an associative HashMap with insertion ordering.
#[derive(Clone, PartialEq)]
pub struct Document {
    inner: LinkedHashMap<String, Bson>,
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
        write!(f, "Document({:?})", self.inner)
    }
}

/// An iterator over Document entries.
pub struct DocumentIntoIterator {
    inner: LinkedHashMap<String, Bson>,
}

/// An owning iterator over Document entries.
pub struct DocumentIterator<'a> {
    inner: linked_hash_map::Iter<'a, String, Bson>,
}

type DocumentMap<'a, T> = Map<DocumentIterator<'a>, fn((&'a String, &'a Bson)) -> T>;

/// An iterator over an Document's keys.
pub struct Keys<'a> {
    inner: DocumentMap<'a, &'a String>,
}

/// An iterator over an Document's values.
pub struct Values<'a> {
    inner: DocumentMap<'a, &'a Bson>,
}

impl<'a> Iterator for Keys<'a> {
    type Item = &'a String;

    fn next(&mut self) -> Option<&'a String> {
        self.inner.next()
    }
}

impl<'a> Iterator for Values<'a> {
    type Item = &'a Bson;

    fn next(&mut self) -> Option<&'a Bson> {
        self.inner.next()
    }
}

impl IntoIterator for Document {
    type Item = (String, Bson);
    type IntoIter = DocumentIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        DocumentIntoIterator { inner: self.inner }
    }
}

impl<'a> IntoIterator for &'a Document {
    type Item = (&'a String, &'a Bson);
    type IntoIter = DocumentIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        DocumentIterator {
            inner: self.inner.iter(),
        }
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

impl<'a> Iterator for DocumentIntoIterator {
    type Item = (String, Bson);

    fn next(&mut self) -> Option<(String, Bson)> {
        self.inner.pop_front()
    }
}

impl<'a> Iterator for DocumentIterator<'a> {
    type Item = (&'a String, &'a Bson);

    fn next(&mut self) -> Option<(&'a String, &'a Bson)> {
        self.inner.next()
    }
}

impl Document {
    /// Creates a new empty Document.
    pub fn new() -> Document {
        Document {
            inner: LinkedHashMap::new(),
        }
    }

    /// Gets an iterator over the entries of the map.
    pub fn iter(&self) -> DocumentIterator {
        self.into_iter()
    }

    /// Clears the document, removing all values.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Returns a reference to the Bson corresponding to the key.
    pub fn get(&self, key: &str) -> Option<&Bson> {
        self.inner.get(key)
    }

    /// Gets a mutable reference to the Bson corresponding to the key
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Bson> {
        self.inner.get_mut(key)
    }

    /// Get a floating point value for this key if it exists and has
    /// the correct type.
    pub fn get_f64(&self, key: &str) -> ValueAccessResult<f64> {
        match self.get(key) {
            Some(&Bson::Double(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a floating point value for this key if it exists and has
    /// the correct type.
    pub fn get_f64_mut(&mut self, key: &str) -> ValueAccessResult<&mut f64> {
        match self.get_mut(key) {
            Some(&mut Bson::Double(ref mut v)) => Ok(v),
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
            Some(&Bson::Int32(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to an i32 value for this key if it exists and has the correct type.
    pub fn get_i32_mut(&mut self, key: &str) -> ValueAccessResult<&mut i32> {
        match self.get_mut(key) {
            Some(&mut Bson::Int32(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get an i64 value for this key if it exists and has the correct type.
    pub fn get_i64(&self, key: &str) -> ValueAccessResult<i64> {
        match self.get(key) {
            Some(&Bson::Int64(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to an i64 value for this key if it exists and has the correct type.
    pub fn get_i64_mut(&mut self, key: &str) -> ValueAccessResult<&mut i64> {
        match self.get_mut(key) {
            Some(&mut Bson::Int64(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a time stamp value for this key if it exists and has the correct type.
    pub fn get_timestamp(&self, key: &str) -> ValueAccessResult<Timestamp> {
        match self.get(key) {
            Some(&Bson::Timestamp(timestamp)) => Ok(timestamp),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a time stamp value for this key if it exists and has the correct
    /// type.
    pub fn get_timestamp_mut(&mut self, key: &str) -> ValueAccessResult<&mut Timestamp> {
        match self.get_mut(key) {
            Some(&mut Bson::Timestamp(ref mut timestamp)) => Ok(timestamp),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a reference to a generic binary value for this key if it exists and has the correct
    /// type.
    pub fn get_binary_generic(&self, key: &str) -> ValueAccessResult<&Vec<u8>> {
        match self.get(key) {
            Some(&Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                ref bytes,
            })) => Ok(bytes),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference generic binary value for this key if it exists and has the correct
    /// type.
    pub fn get_binary_generic_mut(&mut self, key: &str) -> ValueAccessResult<&mut Vec<u8>> {
        match self.get_mut(key) {
            Some(&mut Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                ref mut bytes,
            })) => Ok(bytes),
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

    /// Get a mutable reference to an object id value for this key if it exists and has the correct
    /// type.
    pub fn get_object_id_mut(&mut self, key: &str) -> ValueAccessResult<&mut ObjectId> {
        match self.get_mut(key) {
            Some(&mut Bson::ObjectId(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a reference to a UTC datetime value for this key if it exists and has the correct type.
    pub fn get_datetime(&self, key: &str) -> ValueAccessResult<&DateTime<Utc>> {
        match self.get(key) {
            Some(&Bson::DateTime(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a UTC datetime value for this key if it exists and has the
    /// correct type.
    pub fn get_datetime_mut(&mut self, key: &str) -> ValueAccessResult<&mut DateTime<Utc>> {
        match self.get_mut(key) {
            Some(&mut Bson::DateTime(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Returns true if the map contains a value for the specified key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    /// Gets a collection of all keys in the document.
    pub fn keys<'a>(&'a self) -> Keys<'a> {
        fn first<A, B>((a, _): (A, B)) -> A {
            a
        }
        let first: fn((&'a String, &'a Bson)) -> &'a String = first;

        Keys {
            inner: self.iter().map(first),
        }
    }

    /// Gets a collection of all values in the document.
    pub fn values<'a>(&'a self) -> Values<'a> {
        fn second<A, B>((_, b): (A, B)) -> B {
            b
        }
        let second: fn((&'a String, &'a Bson)) -> &'a Bson = second;

        Values {
            inner: self.iter().map(second),
        }
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
        self.inner.insert(key.into(), val.into())
    }

    /// Takes the value of the entry out of the document, and returns it.
    pub fn remove(&mut self, key: &str) -> Option<Bson> {
        self.inner.remove(key)
    }

    pub fn entry(&mut self, k: String) -> Entry {
        Entry {
            inner: self.inner.entry(k),
        }
    }

    /// Attempts to encode the `Document` into a byte stream.
    pub fn encode<W: Write + ?Sized>(&self, writer: &mut W) -> EncoderResult<()> {
        let mut buf = Vec::new();
        for (key, val) in self.into_iter() {
            encode_bson(&mut buf, key.as_ref(), val)?;
        }

        write_i32(
            writer,
            (buf.len() + mem::size_of::<i32>() + mem::size_of::<u8>()) as i32,
        )?;
        writer.write_all(&buf)?;
        writer.write_u8(0)?;
        Ok(())
    }

    /// Attempts to decode a `Document` from a byte stream.
    pub fn decode<R: Read + ?Sized>(reader: &mut R) -> DecoderResult<Document> {
        let mut doc = Document::new();

        // disregard the length: using Read::take causes infinite type recursion
        read_i32(reader)?;

        loop {
            let tag = reader.read_u8()?;

            if tag == 0 {
                break;
            }

            let (key, val) = decode_bson_kvp(reader, tag, false)?;
            doc.insert(key, val);
        }

        Ok(doc)
    }
}

pub struct Entry<'a> {
    inner: linked_hash_map::Entry<'a, String, Bson>,
}

impl<'a> Entry<'a> {
    pub fn key(&self) -> &str {
        self.inner.key()
    }

    pub fn or_insert(self, default: Bson) -> &'a mut Bson {
        self.inner.or_insert(default)
    }

    pub fn or_insert_with<F: FnOnce() -> Bson>(self, default: F) -> &'a mut Bson {
        self.inner.or_insert_with(default)
    }
}

impl From<LinkedHashMap<String, Bson>> for Document {
    fn from(tree: LinkedHashMap<String, Bson>) -> Document {
        Document { inner: tree }
    }
}

pub struct DocumentVisitor {
    marker: PhantomData<Document>,
}

impl DocumentVisitor {
    #[allow(clippy::new_without_default)]
    pub fn new() -> DocumentVisitor {
        DocumentVisitor {
            marker: PhantomData,
        }
    }
}

impl<'de> Visitor<'de> for DocumentVisitor {
    type Value = Document;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "expecting ordered document")
    }

    #[inline]
    fn visit_unit<E>(self) -> Result<Document, E>
    where
        E: de::Error,
    {
        Ok(Document::new())
    }

    #[inline]
    fn visit_map<V>(self, mut visitor: V) -> Result<Document, V::Error>
    where
        V: MapAccess<'de>,
    {
        let mut inner = match visitor.size_hint() {
            Some(size) => LinkedHashMap::with_capacity(size),
            None => LinkedHashMap::new(),
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
