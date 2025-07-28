//! A BSON document represented as an associative HashMap with insertion ordering.

use std::{
    fmt::{self, Debug, Display, Formatter},
    hash::Hash,
    io::{Read, Write},
    iter::{Extend, FromIterator, IntoIterator},
    ops::Index,
};

use ahash::RandomState;
use indexmap::IndexMap;

use crate::{
    bson::{Array, Bson, Timestamp},
    error::{Error, Result},
    oid::ObjectId,
    spec::{BinarySubtype, ElementType},
    Binary,
    Decimal128,
};

/// A BSON document represented as an associative HashMap with insertion ordering.
#[derive(Clone, PartialEq, Eq)]
pub struct Document {
    inner: IndexMap<String, Bson, RandomState>,
}

impl Default for Document {
    fn default() -> Self {
        Document::new()
    }
}

impl Hash for Document {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let mut entries = Vec::from_iter(&self.inner);
        entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
        entries.hash(state);
    }
}

impl Display for Document {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        let indent = fmt.width().unwrap_or(2);
        let indent_str = " ".repeat(indent);

        write!(fmt, "{{")?;
        if fmt.alternate() && !self.inner.is_empty() {
            fmt.write_str("\n")?;
        }

        let mut first = true;
        for (k, v) in self {
            if !fmt.alternate() {
                if first {
                    first = false;
                    write!(fmt, " ")?;
                } else {
                    fmt.write_str(", ")?;
                }
                write!(fmt, "\"{}\": {}", k, v)?;
            }

            if fmt.alternate() {
                if first {
                    first = false;
                } else {
                    fmt.write_str(",\n")?;
                }
                match v {
                    Bson::Document(ref doc) => {
                        let new_indent = indent + 2;
                        write!(fmt, "{indent_str}\"{}\": {doc:#new_indent$}", k)?;
                    }
                    Bson::Array(_arr) => {
                        let new_indent = indent + 2;
                        write!(fmt, "{indent_str}\"{}\": {v:#new_indent$}", k)?;
                    }
                    _ => {
                        write!(fmt, "{indent_str}\"{}\": {}", k, v)?;
                    }
                }
            }
        }

        let closing_bracket_indent_str = " ".repeat(indent - 2);
        if fmt.alternate() && !self.inner.is_empty() {
            write!(fmt, "\n{closing_bracket_indent_str}}}")
        } else {
            write!(fmt, "{}}}", if !first { " " } else { "" })
        }
    }
}

impl Debug for Document {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        write!(fmt, "Document(")?;
        Debug::fmt(&self.inner, fmt)?;
        write!(fmt, ")")
    }
}

impl Index<&str> for Document {
    type Output = Bson;

    fn index(&self, index: &str) -> &Self::Output {
        match self.get(index) {
            Some(v) => v,
            None => &Bson::Null,
        }
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
    /// Returns a new empty document.
    pub fn new() -> Document {
        Document {
            inner: IndexMap::default(),
        }
    }

    /// Returns an iterator over the contents of the document.
    pub fn iter(&self) -> Iter {
        self.into_iter()
    }

    /// Returns an iterator over mutable references to the contents of the document.
    pub fn iter_mut(&mut self) -> IterMut {
        IterMut {
            inner: self.inner.iter_mut(),
        }
    }

    /// Removes all values from the document.
    pub fn clear(&mut self) {
        self.inner.clear();
    }

    /// Returns a reference to the [`Bson`] value that corresponds to the given key, if present.
    pub fn get(&self, key: impl AsRef<str>) -> Option<&Bson> {
        self.inner.get(key.as_ref())
    }

    /// Returns a mutable reference to the [`Bson`] value that corresponds to the given key, if
    /// present.
    pub fn get_mut(&mut self, key: impl AsRef<str>) -> Option<&mut Bson> {
        self.inner.get_mut(key.as_ref())
    }

    /// Returns the value for the given key if one is present and is of type
    /// [`ElementType::Double`].
    pub fn get_f64(&self, key: impl AsRef<str>) -> Result<f64> {
        let key = key.as_ref();
        match self.get(key) {
            Some(&Bson::Double(v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Double,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a mutable reference to the value for the given key if one is present and is of type
    /// [`ElementType::Double`].
    pub fn get_f64_mut(&mut self, key: impl AsRef<str>) -> Result<&mut f64> {
        let key = key.as_ref();
        match self.get_mut(key) {
            Some(&mut Bson::Double(ref mut v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Double,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a reference to the value for the given key if one is present and is of type
    /// [`ElementType::Decimal128`].
    pub fn get_decimal128(&self, key: impl AsRef<str>) -> Result<&Decimal128> {
        let key = key.as_ref();
        match self.get(key) {
            Some(Bson::Decimal128(v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Decimal128,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a mutable reference to the value for the given key if one is present and is of type
    /// [`ElementType::Decimal128`].
    pub fn get_decimal128_mut(&mut self, key: impl AsRef<str>) -> Result<&mut Decimal128> {
        let key = key.as_ref();
        match self.get_mut(key) {
            Some(&mut Bson::Decimal128(ref mut v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Decimal128,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a reference to the value for the given key if one is present and is of type
    /// [`ElementType::String`].
    pub fn get_str(&self, key: impl AsRef<str>) -> Result<&str> {
        let key = key.as_ref();
        match self.get(key) {
            Some(Bson::String(v)) => Ok(v.as_str()),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::String,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a mutable reference to the value for the given key if one is present and is of type
    /// [`ElementType::String`].
    pub fn get_str_mut(&mut self, key: impl AsRef<str>) -> Result<&mut str> {
        let key = key.as_ref();
        match self.get_mut(key) {
            Some(&mut Bson::String(ref mut v)) => Ok(v.as_mut_str()),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::String,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a reference to the value for the given key if one is present and is of type
    /// [`ElementType::Array`].
    pub fn get_array(&self, key: impl AsRef<str>) -> Result<&Array> {
        let key = key.as_ref();
        match self.get(key) {
            Some(Bson::Array(v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Array,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a mutable reference to the value for the given key if one is present and is of type
    /// [`ElementType::Array`].
    pub fn get_array_mut(&mut self, key: impl AsRef<str>) -> Result<&mut Array> {
        let key = key.as_ref();
        match self.get_mut(key) {
            Some(&mut Bson::Array(ref mut v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Array,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a reference to the value for the given key if one is present and is of type
    /// [`ElementType::EmbeddedDocument`].
    pub fn get_document(&self, key: impl AsRef<str>) -> Result<&Document> {
        let key = key.as_ref();
        match self.get(key) {
            Some(Bson::Document(v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::EmbeddedDocument,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a mutable reference to the value for the given key if one is present and is of type
    /// [`ElementType::EmbeddedDocument`].
    pub fn get_document_mut(&mut self, key: impl AsRef<str>) -> Result<&mut Document> {
        let key = key.as_ref();
        match self.get_mut(key) {
            Some(&mut Bson::Document(ref mut v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::EmbeddedDocument,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a reference to the value for the given key if one is present and is of type
    /// [`ElementType::Boolean`].
    pub fn get_bool(&self, key: impl AsRef<str>) -> Result<bool> {
        let key = key.as_ref();
        match self.get(key) {
            Some(&Bson::Boolean(v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Boolean,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a mutable reference to the value for the given key if one is present and is of type
    /// [`ElementType::Boolean`].
    pub fn get_bool_mut(&mut self, key: impl AsRef<str>) -> Result<&mut bool> {
        let key = key.as_ref();
        match self.get_mut(key) {
            Some(&mut Bson::Boolean(ref mut v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Boolean,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns [`Bson::Null`] if the given key corresponds to a [`Bson::Null`] value.
    pub fn get_null(&self, key: impl AsRef<str>) -> Result<Bson> {
        let key = key.as_ref();
        match self.get(key) {
            Some(&Bson::Null) => Ok(Bson::Null),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Null,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns the value for the given key if one is present and is of type [`ElementType::Int32`].
    pub fn get_i32(&self, key: impl AsRef<str>) -> Result<i32> {
        let key = key.as_ref();
        match self.get(key) {
            Some(&Bson::Int32(v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Int32,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a mutable reference to the value for the given key if one is present and is of type
    /// [`ElementType::Int32`].
    pub fn get_i32_mut(&mut self, key: impl AsRef<str>) -> Result<&mut i32> {
        let key = key.as_ref();
        match self.get_mut(key) {
            Some(&mut Bson::Int32(ref mut v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Int32,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns the value for the given key if one is present and is of type [`ElementType::Int64`].
    pub fn get_i64(&self, key: impl AsRef<str>) -> Result<i64> {
        let key = key.as_ref();
        match self.get(key) {
            Some(&Bson::Int64(v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Int64,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a mutable reference to the value for the given key if one is present and is of type
    /// [`ElementType::Int64`].
    pub fn get_i64_mut(&mut self, key: impl AsRef<str>) -> Result<&mut i64> {
        let key = key.as_ref();
        match self.get_mut(key) {
            Some(&mut Bson::Int64(ref mut v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Int64,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns the value for the given key if one is present and is of type
    /// [`ElementType::Timestamp`].
    pub fn get_timestamp(&self, key: impl AsRef<str>) -> Result<Timestamp> {
        let key = key.as_ref();
        match self.get(key) {
            Some(&Bson::Timestamp(timestamp)) => Ok(timestamp),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Timestamp,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a mutable reference to the value for the given key if one is present and is of type
    /// [`ElementType::Timestamp`].
    pub fn get_timestamp_mut(&mut self, key: impl AsRef<str>) -> Result<&mut Timestamp> {
        let key = key.as_ref();
        match self.get_mut(key) {
            Some(&mut Bson::Timestamp(ref mut timestamp)) => Ok(timestamp),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Timestamp,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a reference to the value for the given key if one is present and is of type
    /// [`ElementType::Binary`] with binary subtype [`BinarySubtype::Generic`].
    pub fn get_binary_generic(&self, key: impl AsRef<str>) -> Result<&Vec<u8>> {
        let key = key.as_ref();
        match self.get(key) {
            Some(&Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                ref bytes,
            })) => Ok(bytes),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Binary,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a mutable reference to the value for the given key if one is present and is of type
    /// [`ElementType::Binary`] with binary subtype [`BinarySubtype::Generic`].
    pub fn get_binary_generic_mut(&mut self, key: impl AsRef<str>) -> Result<&mut Vec<u8>> {
        let key = key.as_ref();
        match self.get_mut(key) {
            Some(&mut Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                ref mut bytes,
            })) => Ok(bytes),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::Binary,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns the value for the given key if one is present and is of type
    /// [`ElementType::ObjectId`].
    pub fn get_object_id(&self, key: impl AsRef<str>) -> Result<ObjectId> {
        let key = key.as_ref();
        match self.get(key) {
            Some(&Bson::ObjectId(v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::ObjectId,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a mutable reference to the value for the given key if one is present and is of type
    /// [`ElementType::ObjectId`].
    pub fn get_object_id_mut(&mut self, key: impl AsRef<str>) -> Result<&mut ObjectId> {
        let key = key.as_ref();
        match self.get_mut(key) {
            Some(&mut Bson::ObjectId(ref mut v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::ObjectId,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a reference to the value for the given key if one is present and is of type
    /// [`ElementType::DateTime`].
    pub fn get_datetime(&self, key: impl AsRef<str>) -> Result<&crate::DateTime> {
        let key = key.as_ref();
        match self.get(key) {
            Some(Bson::DateTime(v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::DateTime,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns a mutable reference to the value for the given key if one is present and is of type
    /// [`ElementType::DateTime`].
    pub fn get_datetime_mut(&mut self, key: impl AsRef<str>) -> Result<&mut crate::DateTime> {
        let key = key.as_ref();
        match self.get_mut(key) {
            Some(&mut Bson::DateTime(ref mut v)) => Ok(v),
            Some(bson) => Err(Error::value_access_unexpected_type(
                bson.element_type(),
                ElementType::DateTime,
            )),
            None => Err(Error::value_access_not_present()),
        }
        .map_err(|e| e.with_key(key))
    }

    /// Returns whether the map contains a value for the specified key.
    pub fn contains_key(&self, key: impl AsRef<str>) -> bool {
        self.inner.contains_key(key.as_ref())
    }

    /// Returns an iterator over the keys in the document.
    pub fn keys(&self) -> Keys {
        Keys {
            inner: self.inner.keys(),
        }
    }

    /// Returns an iterator over the values in the document.
    pub fn values(&self) -> Values {
        Values {
            inner: self.inner.values(),
        }
    }

    /// Returns the number of elements in the document.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns whether the document is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Inserts the provided key-value pair into the document. Any type that implements `Into<Bson>`
    /// can be specified as a value. If a value is already present for the given key, it will be
    /// overridden and returned.
    pub fn insert<KT: Into<String>, BT: Into<Bson>>(&mut self, key: KT, val: BT) -> Option<Bson> {
        self.inner.insert(key.into(), val.into())
    }

    /// Removes and returns the value that corresponds to the given key if present. Computes in
    /// **O(n)** time (average).
    pub fn remove(&mut self, key: impl AsRef<str>) -> Option<Bson> {
        self.inner.shift_remove(key.as_ref())
    }

    /// Returns an [`Entry`] for the given key.
    pub fn entry(&mut self, k: impl Into<String>) -> Entry {
        match self.inner.entry(k.into()) {
            indexmap::map::Entry::Occupied(o) => Entry::Occupied(OccupiedEntry { inner: o }),
            indexmap::map::Entry::Vacant(v) => Entry::Vacant(VacantEntry { inner: v }),
        }
    }

    /// Attempt to encode the [`Document`] into a byte [`Vec`].
    pub fn to_vec(&self) -> Result<Vec<u8>> {
        Ok(crate::RawDocumentBuf::try_from(self)?.into_bytes())
    }

    /// Attempts to encode the [`Document`] into a byte stream.
    ///
    /// While the method signature indicates an owned writer must be passed in, a mutable reference
    /// may also be passed in due to blanket implementations of [`Write`] provided in the standard
    /// library.
    ///
    /// ```
    /// # fn main() -> bson::error::Result<()> {
    /// use bson::doc;
    ///
    /// let mut v: Vec<u8> = Vec::new();
    /// let doc = doc! { "x" : 1 };
    /// doc.to_writer(&mut v)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn to_writer<W: Write>(&self, mut writer: W) -> crate::error::Result<()> {
        let buf = crate::RawDocumentBuf::try_from(self)?;
        writer.write_all(buf.as_bytes())?;
        Ok(())
    }

    /// Attempts to decode a [`Document`] from a byte stream.
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
    pub fn from_reader<R: Read>(reader: R) -> crate::error::Result<Document> {
        let raw = crate::raw::RawDocumentBuf::from_reader(reader)?;
        raw.try_into()
    }
}

/// A view into a single entry in a document, which may either be vacant or occupied.
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

impl VacantEntry<'_> {
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

impl OccupiedEntry<'_> {
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
