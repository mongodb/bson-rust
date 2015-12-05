use chrono::{DateTime, UTC};
use std::collections::BTreeMap;
use std::{error, fmt, slice};
use std::fmt::{Debug, Display, Error, Formatter};
use std::iter::{FromIterator, Map};
use std::vec::IntoIter;

use bson::{Array,Bson,Document};
use oid::ObjectId;
use spec::BinarySubtype;

/// Error to indicate that either a value was empty or it contained an unexpected
/// type, for use with the direct getters.
#[derive(PartialEq)]
pub enum ValueAccessError {
    NotPresent,
    UnexpectedType
}

pub type ValueAccessResult<T> = Result<T, ValueAccessError>;

impl Debug for ValueAccessError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            ValueAccessError::NotPresent => write!(f, "ValueAccessError: field is not present"),
            ValueAccessError::UnexpectedType => write!(f, "ValueAccessError: field does not have the expected type")
        }
    }
}

impl Display for ValueAccessError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            ValueAccessError::NotPresent => write!(f, "field is not present"),
            ValueAccessError::UnexpectedType => write!(f, "field does not have the expected type")
        }
    }
}

impl error::Error for ValueAccessError {
    fn description(&self) -> &str {
        "Error to indicate that either a value was empty or it contained an unexpected type"
    }
}

/// A BSON document represented as an associative BTree Map with insertion ordering.
#[derive(Debug,Clone,PartialEq)]
pub struct OrderedDocument {
    pub keys: Vec<String>,
    document: BTreeMap<String, Bson>,
}

impl Display for OrderedDocument {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        let mut string = "{ ".to_owned();

        for (key, value) in self.iter() {
            if !string.eq("{ ") {
                string.push_str(", ");
            }

            string.push_str(&format!("{}: {}", key, value));
        }

        string.push_str(" }");
        fmt.write_str(&string)
    }
}

/// An iterator over OrderedDocument entries.
pub struct OrderedDocumentIntoIterator {
    vec_iter: IntoIter<String>,
    document: BTreeMap<String, Bson>,
}

/// An owning iterator over OrderedDocument entries.
pub struct OrderedDocumentIterator<'a> {
    vec_iter: slice::Iter<'a, String>,
    document: &'a BTreeMap<String, Bson>,
}

/// An iterator over an OrderedDocument's keys.
pub struct Keys<'a> {
    inner: Map<OrderedDocumentIterator<'a>, fn((&'a String, &'a Bson)) -> &'a String>
}

/// An iterator over an OrderedDocument's values.
pub struct Values<'a> {
    inner: Map<OrderedDocumentIterator<'a>, fn((&'a String, &'a Bson)) -> &'a Bson>
}

impl<'a> Iterator for Keys<'a> {
    type Item = &'a String;
    fn next(&mut self) -> Option<(&'a String)> { self.inner.next() }
}

impl<'a> Iterator for Values<'a> {
    type Item = &'a Bson;
    fn next(&mut self) -> Option<(&'a Bson)> { self.inner.next() }
}

impl IntoIterator for OrderedDocument {
    type Item = (String, Bson);
    type IntoIter = OrderedDocumentIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        OrderedDocumentIntoIterator {
            document: self.document,
            vec_iter: self.keys.into_iter()
        }
    }
}

impl<'a> IntoIterator for &'a OrderedDocument {
    type Item = (&'a String, &'a Bson);
    type IntoIter = OrderedDocumentIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        let ref keys = self.keys;
        OrderedDocumentIterator {
            vec_iter: keys.into_iter(),
            document: &self.document,
        }
    }
}

impl FromIterator<(String, Bson)> for OrderedDocument {
    fn from_iter<T: IntoIterator<Item=(String, Bson)>>(iter: T) -> Self {
        let mut doc = OrderedDocument::new();
        for (k, v) in iter {
            doc.insert(k, v.to_owned());
        }
        doc
    }
}

impl<'a> Iterator for OrderedDocumentIntoIterator {
    type Item = (String, Bson);
    fn next(&mut self) -> Option<(String, Bson)> {
        match self.vec_iter.next() {
            Some(key) => {
                let val = self.document.remove(&key[..]).unwrap();
                Some((key, val))
            },
            None => None,
        }
    }
}

impl<'a> Iterator for OrderedDocumentIterator<'a> {
    type Item = (&'a String, &'a Bson);
    fn next(&mut self) -> Option<(&'a String, &'a Bson)> {
        match self.vec_iter.next() {
            Some(key) => {
                let val = self.document.get(&key[..]).unwrap();
                Some((&key, val))
            },
            None => None,
        }
    }
}

impl OrderedDocument {
    /// Creates a new empty OrderedDocument.
    pub fn new() -> OrderedDocument {
        OrderedDocument {
            keys: Vec::new(),
            document: BTreeMap::new(),
        }
    }

    /// Gets an iterator over the entries of the map.
    pub fn iter<'a>(&'a self) -> OrderedDocumentIterator<'a> {
        self.into_iter()
    }

    /// Clears the document, removing all values.
    pub fn clear(&mut self) {
        self.keys.clear();
        self.document.clear();
    }

    /// Returns a reference to the Bson corresponding to the key.
    pub fn get(&self, key: &str) -> Option<&Bson> {
        self.document.get(key)
    }

    /// Gets a mutable reference to the Bson corresponding to the key
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Bson> {
        self.document.get_mut(key)
    }

    /// Get a floating point value for this key if it exists and has
    /// the correct type.
    pub fn get_f64(&self, key: &str) -> ValueAccessResult<f64> {
        match self.get(key) {
            Some(&Bson::FloatingPoint(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent)
        }
    }

    /// Get a string slice this key if it exists and has the correct type.
    pub fn get_str(&self, key: &str) -> ValueAccessResult<&str> {
        match self.get(key) {
            Some(&Bson::String(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent)
        }
    }

    /// Get a reference to an array for this key if it exists and has
    /// the correct type.
    pub fn get_array(&self, key: &str) -> ValueAccessResult<&Array> {
        match self.get(key) {
            Some(&Bson::Array(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent)
        }
    }

    /// Get a reference to a document for this key if it exists and has
    /// the correct type.
    pub fn get_document(&self, key: &str) -> ValueAccessResult<&Document> {
        match self.get(key) {
            Some(&Bson::Document(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent)
        }
    }

    /// Get a bool value for this key if it exists and has the correct type.
    pub fn get_bool(&self, key: &str) -> ValueAccessResult<bool> {
        match self.get(key) {
            Some(&Bson::Boolean(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent)
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
            None => Err(ValueAccessError::NotPresent)
        }
    }

    /// Get an i64 value for this key if it exists and has the correct type.
    pub fn get_i64(&self, key: &str) -> ValueAccessResult<i64> {
        match self.get(key) {
            Some(&Bson::I64(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent)
        }
    }

    /// Get a time stamp value for this key if it exists and has the correct type.
    pub fn get_time_stamp(&self, key: &str) -> ValueAccessResult<i64> {
        match self.get(key) {
            Some(&Bson::TimeStamp(v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent)
        }
    }

    /// Get a generic binary value for this key if it exists and has the correct type.
    pub fn get_binary_generic(&self, key: &str) -> ValueAccessResult<&Vec<u8>> {
        match self.get(key) {
            Some(&Bson::Binary(BinarySubtype::Generic, ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent)
        }
    }

    /// Get an object id value for this key if it exists and has the correct type.
    pub fn get_object_id(&self, key: &str) -> ValueAccessResult<&ObjectId> {
        match self.get(key) {
            Some(&Bson::ObjectId(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent)
        }
    }

    /// Get a UTC datetime value for this key if it exists and has the correct type.
    pub fn get_utc_datetime(&self, key: &str) -> ValueAccessResult<&DateTime<UTC>> {
        match self.get(key) {
            Some(&Bson::UtcDatetime(ref v)) => Ok(v),
            Some(_) => Err(ValueAccessError::UnexpectedType),
            None => Err(ValueAccessError::NotPresent)
        }
    }

    /// Returns true if the map contains a value for the specified key.
    pub fn contains_key(&self, key: &str) -> bool {
        self.document.contains_key(key)
    }

    /// Returns the position of the key in the ordered vector, if it exists.
    pub fn position(&self, key: &str) -> Option<usize> {
        self.keys.iter().position(|x| x == key)
    }

    /// Gets a collection of all keys in the document.
    pub fn keys<'a>(&'a self) -> Keys<'a> {
        fn first<A, B>((a, _): (A, B)) -> A { a }
        let first: fn((&'a String, &'a Bson)) -> &'a String = first;

        Keys { inner: self.iter().map(first) }
    }

    /// Gets a collection of all values in the document.
    pub fn values<'a>(&'a self) -> Values<'a> {
        fn second<A, B>((_, b): (A, B)) -> B { b }
        let second: fn((&'a String, &'a Bson)) -> &'a Bson = second;

        Values { inner: self.iter().map(second) }
    }

    /// Returns the number of elements in the document.
    pub fn len(&self) -> usize {
        self.keys.len()
    }

    /// Returns true if the document contains no elements
    pub fn is_empty(&self) -> bool {
        self.document.is_empty()
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
        {
            let key_slice = &key[..];
            if self.contains_key(key_slice) {
                let position = self.position(key_slice).unwrap();
                self.keys.remove(position);
            }
        }

        self.keys.push(key.to_owned());
        self.document.insert(key, val)
    }

    /// Takes the value of the entry out of the document, and returns it.
    pub fn remove(&mut self, key: &str) -> Option<Bson> {
        let position = self.position(key);
        if position.is_some() {
            self.keys.remove(position.unwrap());
        }
        self.document.remove(key)
    }
}

impl From<BTreeMap<String, Bson>> for OrderedDocument {
    fn from(tree: BTreeMap<String, Bson>) -> OrderedDocument {        
        OrderedDocument {
            keys: tree.keys().cloned().collect(),
            document: tree,
        }
    }
}
