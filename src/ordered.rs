use bson::Bson;
use std::collections::BTreeMap;
use std::iter::{FromIterator, Map};

/// A BSON document represented as an associative BTree Map with insertion ordering.
#[derive(Debug, Clone)]
pub struct OrderedDocument {
    pub keys: Vec<String>,
    document: BTreeMap<String, Bson>,
}

/// An iterator over OrderedDocument entries.
#[derive(Clone)]
pub struct OrderedDocumentIntoIterator {
    ordered_document: OrderedDocument,
    index: usize,
}

/// An owning iterator over OrderedDocument entries.
#[derive(Clone)]
pub struct OrderedDocumentIterator<'a> {
    ordered_document: &'a OrderedDocument,
    index: usize,
}

/// An iterator over an OrderedDocument's keys.
pub struct Keys<'a> {
    inner: Map<OrderedDocumentIterator<'a>, fn((&'a String, &'a Bson)) -> &'a String>
}

/// An iterator over an OrderedDocument's values.
pub struct Values<'a> {
    inner: Map<OrderedDocumentIterator<'a>, fn((&'a String, &'a Bson)) -> &'a Bson>
}

impl<'a> Clone for Keys<'a> {
    fn clone(&self) -> Keys<'a> { Keys { inner: self.inner.clone() } }
}

impl<'a> Iterator for Keys<'a> {
    type Item = &'a String;
    fn next(&mut self) -> Option<(&'a String)> { self.inner.next() }
}

impl<'a> Clone for Values<'a> {
    fn clone(&self) -> Values<'a> { Values { inner: self.inner.clone() } }
}

impl<'a> Iterator for Values<'a> {
    type Item = &'a Bson;

    fn next(&mut self) -> Option<(&'a Bson)> { self.inner.next() }
}

impl IntoIterator for OrderedDocument {
    type Item = (String, Bson);
    type IntoIter = OrderedDocumentIntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        OrderedDocumentIntoIterator { ordered_document: self, index: 0 }
    }
}

impl<'a> IntoIterator for &'a OrderedDocument {
    type Item = (&'a String, &'a Bson);
    type IntoIter = OrderedDocumentIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        OrderedDocumentIterator { ordered_document: self, index: 0 }
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
        if self.ordered_document.keys.len() <= self.index {
            return None;
        }

        let ref key = self.ordered_document.keys[self.index];
        let val = self.ordered_document.get(&key[..]).unwrap();
        self.index += 1;
        Some((key.to_owned(), val.to_owned()))
    }
}

impl<'a> Iterator for OrderedDocumentIterator<'a> {
    type Item = (&'a String, &'a Bson);
    fn next(&mut self) -> Option<(&'a String, &'a Bson)> {
        if self.ordered_document.keys.len() <= self.index {
            return None;
        }

        let ref key = self.ordered_document.keys[self.index];
        let val = self.ordered_document.get(&key[..]).unwrap();
        self.index += 1;
        Some((key, val))
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

    /// Gets a mutable reference to the value in the entry.
    pub fn get_mut(&mut self, key: &str) -> Option<&mut Bson> {
        self.document.get_mut(key)
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
    /// and returns the entry's old value.
    pub fn insert(&mut self, key: String, val: Bson) -> Option<Bson> {
        let key_slice = &key[..];

        if self.contains_key(key_slice) {
            let position = self.position(key_slice).unwrap();
            self.keys.remove(position);
        }

        self.keys.push(key.to_owned());
        self.document.insert(key.to_owned(), val.to_owned())
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

#[cfg(test)]
mod test {
    use super::OrderedDocument;
    use bson::Bson;

    #[test]
    fn ordered_insert() {
        let mut doc = OrderedDocument::new();
        doc.insert("first".to_owned(), Bson::I32(1));
        doc.insert("second".to_owned(), Bson::String("foo".to_owned()));
        doc.insert("alphanumeric".to_owned(), Bson::String("bar".to_owned()));

        let expected_keys = vec!(
            "first".to_owned(),
            "second".to_owned(),
            "alphanumeric".to_owned(),
        );

        let keys: Vec<_> = doc.iter().map(|(key, _)| key.to_owned()).collect();
        assert_eq!(expected_keys, keys);
    }

    #[test]
    fn remove() {
        let mut doc = OrderedDocument::new();
        doc.insert("first".to_owned(), Bson::I32(1));
        doc.insert("second".to_owned(), Bson::String("foo".to_owned()));
        doc.insert("alphanumeric".to_owned(), Bson::String("bar".to_owned()));

        assert!(doc.remove("second").is_some());
        assert!(doc.remove("none").is_none());

        let expected_keys = vec!(
            "first",
            "alphanumeric",
        );

        let keys: Vec<_> = doc.iter().map(|(key, _)| key.to_owned()).collect();
        assert_eq!(expected_keys, keys);
    }
}
