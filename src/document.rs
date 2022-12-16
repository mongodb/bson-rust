//! A BSON document represented as an associative HashMap with insertion ordering.

use std::{
    error,
    fmt::{self, Debug, Display, Formatter},
    io::{Read, Write},
    iter::{Extend, FromIterator, IntoIterator},
    mem,
};

use ahash::RandomState;
use indexmap::IndexMap;
use serde::de::Error;

use crate::{
    bson::{Array, Bson, Timestamp},
    de::{deserialize_bson_kvp, ensure_read_exactly, read_i32, MIN_BSON_DOCUMENT_SIZE},
    oid::ObjectId,
    ser::{serialize_bson, write_i32},
    spec::BinarySubtype,
    Binary,
    Decimal128,
};

/// Error to indicate that either a value was empty or it contained an unexpected
/// type, for use with the direct getters.
#[derive(PartialEq, Clone)]
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
    inner: IndexMap<String, Bson, RandomState>,
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

            write!(fmt, "\"{}\": {}", k, v)?;
        }

        write!(fmt, "{}}}", if !first { " " } else { "" })
    }
}

impl Debug for Document {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "Document(")?;
        Debug::fmt(&self.inner, fmt)?;
        write!(fmt, ")")
    }
}

/// An iterator over Document entries.
pub struct IntoIter {
    inner: indexmap::map::IntoIter<String, Bson>,
}

/// An owning iterator over Document entries.
pub struct Iter<'a> {
    inner: indexmap::map::Iter<'a, String, Bson>,
}

/// An iterator over an Document's keys.
pub struct Keys<'a> {
    inner: indexmap::map::Keys<'a, String, Bson>,
}

/// An iterator over an Document's values.
pub struct Values<'a> {
    inner: indexmap::map::Values<'a, String, Bson>,
}

/// An iterator over a [`Document`]'s keys and mutable values.
pub struct IterMut<'a> {
    inner: indexmap::map::IterMut<'a, String, Bson>,
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
    type IntoIter = IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        IntoIter {
            inner: self.inner.into_iter(),
        }
    }
}

impl<'a> IntoIterator for &'a Document {
    type Item = (&'a String, &'a Bson);
    type IntoIter = Iter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        Iter {
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

impl Iterator for IntoIter {
    type Item = (String, Bson);

    fn next(&mut self) -> Option<(String, Bson)> {
        self.inner.next()
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a String, &'a Bson);

    fn next(&mut self) -> Option<(&'a String, &'a Bson)> {
        self.inner.next()
    }
}

impl<'a> Iterator for IterMut<'a> {
    type Item = (&'a String, &'a mut Bson);

    fn next(&mut self) -> Option<(&'a String, &'a mut Bson)> {
        self.inner.next()
    }
}

impl Document {
    /// Creates a new empty Document.
    pub fn new() -> Document {
        Document {
            inner: IndexMap::default(),
        }
    }

    /// Gets an iterator over the entries of the map.
    pub fn iter(&self) -> Iter {
        self.into_iter()
    }

    /// Gets an iterator over pairs of keys and mutable values.
    pub fn iter_mut(&mut self) -> IterMut {
        IterMut {
            inner: self.inner.iter_mut(),
        }
    }

    /// Clears the document, removing all values.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Returns a reference to the Bson corresponding to the key.
    pub fn get(&self, key: impl AsRef<str>) -> Option<&Bson> {
        self.inner.get(key.as_ref())
    }

    /// Gets a mutable reference to the Bson corresponding to the key
    pub fn get_mut(&mut self, key: impl AsRef<str>) -> Option<&mut Bson> {
        self.inner.get_mut(key.as_ref())
    }

    /// Get a floating point value for this key if it exists and has
    /// the correct type.
    pub fn get_f64(&self, key: impl AsRef<str>) -> ValueAccessResult<f64> {
        match self.get(key) {
            Some(&Bson::Double(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a floating point value for this key if it exists and has
    /// the correct type.
    pub fn get_f64_mut(&mut self, key: impl AsRef<str>) -> ValueAccessResult<&mut f64> {
        match self.get_mut(key) {
            Some(&mut Bson::Double(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a reference to a Decimal128 value for key, if it exists.
    pub fn get_decimal128(&self, key: impl AsRef<str>) -> ValueAccessResult<&Decimal128> {
        match self.get(key) {
            Some(&Bson::Decimal128(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a Decimal128 value for key, if it exists.
    pub fn get_decimal128_mut(
        &mut self,
        key: impl AsRef<str>,
    ) -> ValueAccessResult<&mut Decimal128> {
        match self.get_mut(key) {
            Some(&mut Bson::Decimal128(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a string slice this key if it exists and has the correct type.
    pub fn get_str(&self, key: impl AsRef<str>) -> ValueAccessResult<&str> {
        match self.get(key) {
            Some(&Bson::String(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable string slice this key if it exists and has the correct type.
    pub fn get_str_mut(&mut self, key: impl AsRef<str>) -> ValueAccessResult<&mut str> {
        match self.get_mut(key) {
            Some(&mut Bson::String(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a reference to an array for this key if it exists and has
    /// the correct type.
    pub fn get_array(&self, key: impl AsRef<str>) -> ValueAccessResult<&Array> {
        match self.get(key) {
            Some(&Bson::Array(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to an array for this key if it exists and has
    /// the correct type.
    pub fn get_array_mut(&mut self, key: impl AsRef<str>) -> ValueAccessResult<&mut Array> {
        match self.get_mut(key) {
            Some(&mut Bson::Array(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a reference to a document for this key if it exists and has
    /// the correct type.
    pub fn get_document(&self, key: impl AsRef<str>) -> ValueAccessResult<&Document> {
        match self.get(key) {
            Some(&Bson::Document(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a document for this key if it exists and has
    /// the correct type.
    pub fn get_document_mut(&mut self, key: impl AsRef<str>) -> ValueAccessResult<&mut Document> {
        match self.get_mut(key) {
            Some(&mut Bson::Document(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a bool value for this key if it exists and has the correct type.
    pub fn get_bool(&self, key: impl AsRef<str>) -> ValueAccessResult<bool> {
        match self.get(key) {
            Some(&Bson::Boolean(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a bool value for this key if it exists and has the correct type.
    pub fn get_bool_mut(&mut self, key: impl AsRef<str>) -> ValueAccessResult<&mut bool> {
        match self.get_mut(key) {
            Some(&mut Bson::Boolean(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Returns wether this key has a null value
    pub fn is_null(&self, key: impl AsRef<str>) -> bool {
        self.get(key) == Some(&Bson::Null)
    }

    /// Get an i32 value for this key if it exists and has the correct type.
    pub fn get_i32(&self, key: impl AsRef<str>) -> ValueAccessResult<i32> {
        match self.get(key) {
            Some(&Bson::Int32(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to an i32 value for this key if it exists and has the correct type.
    pub fn get_i32_mut(&mut self, key: impl AsRef<str>) -> ValueAccessResult<&mut i32> {
        match self.get_mut(key) {
            Some(&mut Bson::Int32(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get an i64 value for this key if it exists and has the correct type.
    pub fn get_i64(&self, key: impl AsRef<str>) -> ValueAccessResult<i64> {
        match self.get(key) {
            Some(&Bson::Int64(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to an i64 value for this key if it exists and has the correct type.
    pub fn get_i64_mut(&mut self, key: impl AsRef<str>) -> ValueAccessResult<&mut i64> {
        match self.get_mut(key) {
            Some(&mut Bson::Int64(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a time stamp value for this key if it exists and has the correct type.
    pub fn get_timestamp(&self, key: impl AsRef<str>) -> ValueAccessResult<Timestamp> {
        match self.get(key) {
            Some(&Bson::Timestamp(timestamp)) => Ok(timestamp),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a time stamp value for this key if it exists and has the correct
    /// type.
    pub fn get_timestamp_mut(&mut self, key: impl AsRef<str>) -> ValueAccessResult<&mut Timestamp> {
        match self.get_mut(key) {
            Some(&mut Bson::Timestamp(ref mut timestamp)) => Ok(timestamp),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a reference to a generic binary value for this key if it exists and has the correct
    /// type.
    pub fn get_binary_generic(&self, key: impl AsRef<str>) -> ValueAccessResult<&Vec<u8>> {
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
    pub fn get_binary_generic_mut(
        &mut self,
        key: impl AsRef<str>,
    ) -> ValueAccessResult<&mut Vec<u8>> {
        match self.get_mut(key) {
            Some(&mut Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                ref mut bytes,
            })) => Ok(bytes),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get an object id value for this key if it exists and has the correct type.
    pub fn get_object_id(&self, key: impl AsRef<str>) -> ValueAccessResult<ObjectId> {
        match self.get(key) {
            Some(&Bson::ObjectId(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to an object id value for this key if it exists and has the correct
    /// type.
    pub fn get_object_id_mut(&mut self, key: impl AsRef<str>) -> ValueAccessResult<&mut ObjectId> {
        match self.get_mut(key) {
            Some(&mut Bson::ObjectId(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a reference to a UTC datetime value for this key if it exists and has the correct type.
    pub fn get_datetime(&self, key: impl AsRef<str>) -> ValueAccessResult<&crate::DateTime> {
        match self.get(key) {
            Some(&Bson::DateTime(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Get a mutable reference to a UTC datetime value for this key if it exists and has the
    /// correct type.
    pub fn get_datetime_mut(
        &mut self,
        key: impl AsRef<str>,
    ) -> ValueAccessResult<&mut crate::DateTime> {
        match self.get_mut(key) {
            Some(&mut Bson::DateTime(ref mut v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent),
        }
    }

    /// Returns true if the map contains a value for the specified key.
    pub fn contains_key(&self, key: impl AsRef<str>) -> bool {
        self.inner.contains_key(key.as_ref())
    }

    /// Gets a collection of all keys in the document.
    pub fn keys(&self) -> Keys {
        Keys {
            inner: self.inner.keys(),
        }
    }

    /// Gets a collection of all values in the document.
    pub fn values(&self) -> Values {
        Values {
            inner: self.inner.values(),
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
    /// Computes in **O(n)** time (average).
    pub fn remove(&mut self, key: impl AsRef<str>) -> Option<Bson> {
        self.inner.shift_remove(key.as_ref())
    }

    pub fn entry(&mut self, k: String) -> Entry {
        match self.inner.entry(k) {
            indexmap::map::Entry::Occupied(o) => Entry::Occupied(OccupiedEntry { inner: o }),
            indexmap::map::Entry::Vacant(v) => Entry::Vacant(VacantEntry { inner: v }),
        }
    }

    /// Attempts to serialize the [`Document`] into a byte stream.
    ///
    /// While the method signature indicates an owned writer must be passed in, a mutable reference
    /// may also be passed in due to blanket implementations of [`Write`] provided in the standard
    /// library.
    ///
    /// ```
    /// # fn main() -> bson::ser::Result<()> {
    /// use bson::doc;
    ///
    /// let mut v: Vec<u8> = Vec::new();
    /// let doc = doc! { "x" : 1 };
    /// doc.to_writer(&mut v)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_writer<W: Write>(&self, mut writer: W) -> crate::ser::Result<()> {
        let mut buf = Vec::new();
        for (key, val) in self.into_iter() {
            serialize_bson(&mut buf, key.as_ref(), val)?;
        }

        write_i32(
            &mut writer,
            (buf.len() + mem::size_of::<i32>() + mem::size_of::<u8>()) as i32,
        )?;
        writer.write_all(&buf)?;
        writer.write_all(&[0])?;
        Ok(())
    }

    fn decode<R: Read + ?Sized>(reader: &mut R, utf_lossy: bool) -> crate::de::Result<Document> {
        let mut doc = Document::new();

        let length = read_i32(reader)?;
        if length < MIN_BSON_DOCUMENT_SIZE {
            return Err(crate::de::Error::invalid_length(
                length as usize,
                &"document length must be at least 5",
            ));
        }

        ensure_read_exactly(
            reader,
            (length as usize) - 4,
            "document length longer than contents",
            |cursor| {
                loop {
                    let mut tag_byte = [0];
                    cursor.read_exact(&mut tag_byte)?;
                    let tag = tag_byte[0];

                    if tag == 0 {
                        break;
                    }

                    let (key, val) = deserialize_bson_kvp(cursor, tag, utf_lossy)?;
                    doc.insert(key, val);
                }
                Ok(())
            },
        )?;

        Ok(doc)
    }

    /// Attempts to deserialize a [`Document`] from a byte stream.
    ///
    /// While the method signature indicates an owned reader must be passed in, a mutable reference
    /// may also be passed in due to blanket implementations of [`Read`] provided in the standard
    /// library.
    ///
    /// ```
    /// # use std::error::Error;
    /// # fn main() -> std::result::Result<(), Box<dyn Error>> {
    /// use bson::{doc, Document};
    /// use std::io::Cursor;
    ///
    /// let mut v: Vec<u8> = Vec::new();
    /// let doc = doc! { "x" : 1 };
    /// doc.to_writer(&mut v)?;
    ///
    /// // read from mutable reference
    /// let mut reader = Cursor::new(v.clone());
    /// let doc1 = Document::from_reader(&mut reader)?;
    ///
    /// // read from owned value
    /// let doc2 = Document::from_reader(Cursor::new(v))?;
    ///
    /// assert_eq!(doc, doc1);
    /// assert_eq!(doc, doc2);
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_reader<R: Read>(mut reader: R) -> crate::de::Result<Document> {
        Self::decode(&mut reader, false)
    }

    /// Attempt to deserialize a [`Document`] that may contain invalid UTF-8 strings from a byte
    /// stream.
    ///
    /// This is mainly useful when reading raw BSON returned from a MongoDB server, which
    /// in rare cases can contain invalidly truncated strings (<https://jira.mongodb.org/browse/SERVER-24007>).
    /// For most use cases, `Document::from_reader` can be used instead.
    pub fn from_reader_utf8_lossy<R: Read>(mut reader: R) -> crate::de::Result<Document> {
        Self::decode(&mut reader, true)
    }
}

/// A view into a single entry in a map, which may either be vacant or occupied.
///
/// This enum is constructed from the entry method on HashMap.
pub enum Entry<'a> {
    /// An occupied entry.
    Occupied(OccupiedEntry<'a>),

    /// A vacant entry.
    Vacant(VacantEntry<'a>),
}

impl<'a> Entry<'a> {
    /// Returns a reference to this entry's key.
    pub fn key(&self) -> &str {
        match self {
            Self::Vacant(v) => v.key(),
            Self::Occupied(o) => o.key(),
        }
    }

    fn into_indexmap_entry(self) -> indexmap::map::Entry<'a, String, Bson> {
        match self {
            Self::Occupied(o) => indexmap::map::Entry::Occupied(o.inner),
            Self::Vacant(v) => indexmap::map::Entry::Vacant(v.inner),
        }
    }

    /// Inserts the given default value in the entry if it is vacant and returns a mutable reference
    /// to it. Otherwise a mutable reference to an already existent value is returned.
    pub fn or_insert(self, default: Bson) -> &'a mut Bson {
        self.into_indexmap_entry().or_insert(default)
    }

    /// Inserts the result of the `default` function in the entry if it is vacant and returns a
    /// mutable reference to it. Otherwise a mutable reference to an already existent value is
    /// returned.
    pub fn or_insert_with<F: FnOnce() -> Bson>(self, default: F) -> &'a mut Bson {
        self.into_indexmap_entry().or_insert_with(default)
    }
}

/// A view into a vacant entry in a [Document]. It is part of the [Entry] enum.
pub struct VacantEntry<'a> {
    inner: indexmap::map::VacantEntry<'a, String, Bson>,
}

impl<'a> VacantEntry<'a> {
    /// Gets a reference to the key that would be used when inserting a value through the
    /// [VacantEntry].
    fn key(&self) -> &str {
        self.inner.key()
    }
}

/// A view into an occupied entry in a [Document]. It is part of the [Entry] enum.
pub struct OccupiedEntry<'a> {
    inner: indexmap::map::OccupiedEntry<'a, String, Bson>,
}

impl<'a> OccupiedEntry<'a> {
    /// Gets a reference to the key in the entry.
    pub fn key(&self) -> &str {
        self.inner.key()
    }
}

impl Extend<(String, Bson)> for Document {
    fn extend<T: IntoIterator<Item = (String, Bson)>>(&mut self, iter: T) {
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}
