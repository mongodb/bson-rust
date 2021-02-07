/*!
A rawbson document can be created from a `Vec<u8>` containing raw BSON data, and elements
accessed via methods similar to those in the [bson-rust](https://crates.io/crate/bson-rust)
crate.  Note that rawbson returns a Result<Option<T>>, since the bytes contained in the
document are not fully validated until trying to access the contained data.

```rust
use bson::raw::{
    DocBuf,
    elem,
};

// \x13\x00\x00\x00           // total document size
// \x02                       // 0x02 = type String
// hi\x00                     // field name
// \x06\x00\x00\x00y'all\x00  // field value
// \x00                       // document terminating NUL

let doc = DocBuf::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
let elem: Option<elem::Element> = doc.get("hi")?;
assert_eq!(
    elem.unwrap().as_str()?,
    "y'all",
);
# Ok::<(), bson::raw::RawError>(())
```

### bson-rust interop

This crate is designed to interoperate smoothly with the bson crate.

A [`DocBuf`] can be created from a [`bson::document::Document`].  Internally, this
serializes the `Document` to a `Vec<u8>`, and then includes those bytes in the [`DocBuf`].

```rust
use bson::doc;
use bson::raw::{
    DocBuf,
};

let document = doc!{"goodbye": {"cruel": "world"}};
let raw = DocBuf::from_document(&document);
let value: Option<&str> = raw.get_document("goodbye")?
    .map(|doc| doc.get_str("cruel"))
    .transpose()?
    .flatten();

assert_eq!(
    value,
    Some("world"),
);
# Ok::<(), bson::raw::RawError>(())
```

### Reference types

A BSON document can also be accessed with the [`Doc`] reference type,
which is an unsized type that represents the BSON payload as a `[u8]`.
This allows accessing nested documents without reallocation.  [Doc]
must always be accessed via a pointer type, similarly to `[T]` and `str`.

This type will coexist with the now deprecated [DocRef] type for at
least one minor release.

The below example constructs a bson document in a stack-based array,
and extracts a &str from it, performing no heap allocation.

```rust
use bson::raw::Doc;

let bytes = b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00";
assert_eq!(Doc::new(bytes)?.get_str("hi")?, Some("y'all"));
# Ok::<(), bson::raw::RawError>(())
```

### Iteration

[`Doc`] implements [`IntoIterator`](std::iter::IntoIterator), which can also
be accessed via [`DocBuf::iter`].

```rust
use bson::doc;
use bson::raw::{DocBuf, elem::Element};

let doc = DocBuf::from_document(&doc! {"crate": "rawbson", "license": "MIT"});
let mut dociter = doc.iter();

let (key, value): (&str, Element) = dociter.next().unwrap()?;
assert_eq!(key, "crate");
assert_eq!(value.as_str()?, "rawbson");

let (key, value): (&str, Element) = dociter.next().unwrap()?;
assert_eq!(key, "license");
assert_eq!(value.as_str()?, "MIT");
# Ok::<(), bson::raw::RawError>(())
```

### serde support

There is also serde deserialization support.

Serde serialization support is not yet provided.  For now, use
[`bson::to_document`] instead, and then serialize it out using
[`bson::Document::to_writer`] or [`DocBuf::from_document`].

```rust
use serde::Deserialize;
use bson::{doc, Document, oid::ObjectId, DateTime};
use bson::raw::{DocBuf, de::from_docbuf};

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct User {
    #[serde(rename = "_id")]
    id: ObjectId,
    first_name: String,
    last_name: String,
    birthdate: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(flatten)]
    extra: Document,
}

let doc = DocBuf::from_document(&doc!{
    "_id": ObjectId::with_string("543254325432543254325432")?,
    "firstName": "John",
    "lastName": "Doe",
    "birthdate": null,
    "luckyNumbers": [3, 60, 2147483647],
    "nickname": "Red",
});

let user: User = from_docbuf(&doc)?;
assert_eq!(user.id.to_hex(), "543254325432543254325432");
assert_eq!(user.first_name, "John");
assert_eq!(user.last_name, "Doe");
assert_eq!(user.extra.get_str("nickname")?, "Red");
assert!(user.birthdate.is_none());
# Ok::<(), Box<dyn std::error::Error>>(())
```
*/

use std::{
    borrow::Borrow,
    convert::{TryFrom, TryInto},
    ops::Deref,
};

use chrono::{DateTime, Utc};

#[cfg(feature = "decimal128")]
use crate::decimal128::Decimal128;

use crate::{document::ValueAccessError, oid, spec::ElementType, Bson};

pub mod de;
pub mod elem;

#[cfg(test)]
mod props;

/// Error to indicate that either a value was empty or it contained an unexpected
/// type, for use with the direct getters.
#[derive(Debug, PartialEq)]
pub enum RawError {
    /// Found a Bson value with the specified key, but not with the expected type
    UnexpectedType,

    /// The found value was not well-formed
    MalformedValue(String),

    /// Found a value where a utf-8 string was expected, but it was not valid
    /// utf-8.  The error value contains the malformed data as a string.
    Utf8EncodingError(Vec<u8>),
}

impl std::fmt::Display for RawError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        use RawError::*;
        match self {
            UnexpectedType => write!(f, "unexpected type"),
            MalformedValue(s) => write!(f, "malformed value: {:?}", s),
            Utf8EncodingError(_) => write!(f, "utf-8 encoding error"),
        }
    }
}

impl std::error::Error for RawError {}

pub type RawResult<T> = Result<T, RawError>;
type OptResult<T> = RawResult<Option<T>>;

impl<'a> From<RawError> for ValueAccessError {
    fn from(src: RawError) -> ValueAccessError {
        match src {
            RawError::UnexpectedType => ValueAccessError::UnexpectedType,
            RawError::MalformedValue(_) => ValueAccessError::UnexpectedType,
            RawError::Utf8EncodingError(_) => ValueAccessError::UnexpectedType,
        }
    }
}

impl<'a> From<ValueAccessError> for RawError {
    fn from(src: ValueAccessError) -> RawError {
        match src {
            ValueAccessError::NotPresent => unreachable!("This should be converted to an Option"),
            ValueAccessError::UnexpectedType => RawError::UnexpectedType,
        }
    }
}

/// A BSON document, stored as raw binary data on the heap.  This can be created from
/// a `Vec<u8>` or a [`bson::Document`].
///
/// Accessing elements within the `DocBuf` is similar to element access in [bson::Document],
/// but as the contents are parsed during iteration, instead of at creation time, format
/// errors can happen at any time during use, instead of at creation time.
///
/// DocBuf can be iterated over, yielding a Result containing key-value pairs that
/// borrow from the DocBuf instead of allocating, when necessary.
///
/// ```
/// # use bson::raw::{DocBuf, RawError};
/// let docbuf = DocBuf::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
/// let mut iter = docbuf.iter();
/// let (key, value) = iter.next().unwrap()?;
/// assert_eq!(key, "hi");
/// assert_eq!(value.as_str(), Ok("y'all"));
/// assert!(iter.next().is_none());
/// # Ok::<(), RawError>(())
/// ```
///
/// Individual elements can be accessed using [`docbuf.get(&key)`](Doc::get), or any of
/// the `get_*` methods, like [`docbuf.get_object_id(&key)`](Doc::get_object_id), and
/// [`docbuf.get_str(&str)`](Doc::get_str).  Accessing elements is an O(N) operation,
/// as it requires iterating through the document from the beginning to find the requested
/// key.
///
/// ```
/// # use bson::raw::{DocBuf, RawError};
/// let docbuf = DocBuf::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
/// assert_eq!(docbuf.get_str("hi")?, Some("y'all"));
/// # Ok::<(), RawError>(())
/// ```
#[derive(Clone, Debug)]
pub struct DocBuf {
    data: Box<[u8]>,
}

impl DocBuf {
    /// Create a new `DocBuf` from the provided `Vec`.
    ///
    /// The data is checked for a declared length equal to the length of the Vec,
    /// and a trailing NUL byte.  Other validation is deferred to access time.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, RawError};
    /// let docbuf: DocBuf = DocBuf::new(b"\x05\0\0\0\0".to_vec())?;
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn new(data: Vec<u8>) -> RawResult<DocBuf> {
        if data.len() < 5 {
            return Err(RawError::MalformedValue("document too short".into()));
        }
        let length = i32_from_slice(&data[..4]);
        if data.len() as i32 != length {
            return Err(RawError::MalformedValue("document length incorrect".into()));
        }
        if data[data.len() - 1] != 0 {
            return Err(RawError::MalformedValue(
                "document not null-terminated".into(),
            ));
        }
        Ok(unsafe { DocBuf::new_unchecked(data) })
    }

    /// Create a DocBuf from a [bson::Document].
    ///
    /// ```
    /// # use bson::raw::{DocBuf, RawError};
    /// use bson::{doc, oid};
    /// let document = doc! {
    ///     "_id": oid::ObjectId::new(),
    ///     "name": "Herman Melville",
    ///     "title": "Moby-Dick",
    /// };
    /// let docbuf: DocBuf = DocBuf::from_document(&document);
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn from_document(doc: &crate::Document) -> DocBuf {
        let mut data = Vec::new();
        doc.to_writer(&mut data).unwrap();
        unsafe { DocBuf::new_unchecked(data) }
    }

    /// Create a DocBuf from an owned Vec<u8> without performing any checks on the provided data.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, RawError};
    /// let docbuf: DocBuf = unsafe {
    ///     DocBuf::new_unchecked(b"\x05\0\0\0\0".to_vec())
    /// };
    /// # Ok::<(), RawError>(())
    /// ```
    ///
    /// # Safety
    ///
    /// The provided bytes must have a valid length marker, and be NUL terminated.
    pub unsafe fn new_unchecked(data: Vec<u8>) -> DocBuf {
        DocBuf {
            data: data.into_boxed_slice(),
        }
    }

    /// Return a [`&Doc`](Doc) borrowing from the data contained in self.
    ///
    /// # Deprecation
    ///
    /// DocRef is now a deprecated type alias for [Doc].  DocBuf can
    /// dereference to &Doc directly, or be converted using [AsRef::as_ref],
    /// so this function is unnecessary.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, DocRef, RawError};
    /// let docbuf = DocBuf::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
    /// let docref: DocRef = docbuf.as_docref();
    /// # Ok::<(), RawError>(())
    /// ```
    #[deprecated(since = "0.2.0", note = "use docbuf.as_ref() instead")]
    pub fn as_docref(&self) -> &Doc {
        self.as_ref()
    }

    /// Return an iterator over the elements in the `DocBuf`, borrowing data.
    ///
    /// The associated item type is `Result<&str, Element<'_>>`.  An error is
    /// returned if data is malformed.
    ///
    /// ```
    /// # use bson::raw::{elem, DocBuf, RawError};
    /// use bson::doc;
    /// let docbuf = DocBuf::from_document(&doc! { "ferris": true });
    /// for element in docbuf.iter() {
    ///     let (key, value): (&str, elem::Element) = element?;
    ///     assert_eq!(key, "ferris");
    ///     assert_eq!(value.as_bool()?, true);
    /// }
    /// # Ok::<(), RawError>(())
    /// ```
    ///
    /// # Note:
    ///
    /// There is no owning iterator for DocBuf.  If you need ownership over
    /// elements that might need to allocate, you must explicitly convert
    /// them to owned types yourself.
    pub fn iter(&self) -> DocIter<'_> {
        self.into_iter()
    }

    /// Return the contained data as a `Vec<u8>`
    ///
    /// ```
    /// # use bson::raw::DocBuf;
    /// use bson::doc;
    /// let docbuf = DocBuf::from_document(&doc!{});
    /// assert_eq!(docbuf.into_inner(), b"\x05\x00\x00\x00\x00".to_vec());
    /// ```
    pub fn into_inner(self) -> Vec<u8> {
        self.data.to_vec()
    }
}

impl TryFrom<DocBuf> for crate::Document {
    type Error = RawError;

    fn try_from(rawdoc: DocBuf) -> RawResult<crate::Document> {
        crate::Document::try_from(rawdoc.as_ref())
    }
}

impl<'a> IntoIterator for &'a DocBuf {
    type IntoIter = DocIter<'a>;
    type Item = RawResult<(&'a str, elem::Element<'a>)>;

    fn into_iter(self) -> DocIter<'a> {
        DocIter {
            doc: &self,
            offset: 4,
        }
    }
}

impl AsRef<Doc> for DocBuf {
    fn as_ref(&self) -> &Doc {
        // SAFETY: Constructing the DocBuf checks the envelope validity of the BSON document.
        unsafe { Doc::new_unchecked(&self.data) }
    }
}

impl Borrow<Doc> for DocBuf {
    fn borrow(&self) -> &Doc {
        &*self
    }
}

impl ToOwned for Doc {
    type Owned = DocBuf;

    fn to_owned(&self) -> Self::Owned {
        self.to_docbuf()
    }
}

/// A BSON document, referencing raw binary data stored elsewhere.  This can be created from
/// a [DocBuf] or any type that contains valid BSON data, and can be referenced as a `[u8]`,
/// including static binary literals, [Vec<u8>](std::vec::Vec), or arrays.
///
/// Accessing elements within the `Doc` is similar to element access in [bson::Document],
/// but as the contents are parsed during iteration, instead of at creation time, format
/// errors can happen at any time during use, instead of at creation time.
///
/// Doc can be iterated over, yielding a Result containing key-value pairs that share the
/// borrow with the source bytes instead of allocating, when necessary.
///
/// ```
/// # use bson::raw::{Doc, RawError};
/// let doc = Doc::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00")?;
/// let mut iter = doc.into_iter();
/// let (key, value) = iter.next().unwrap()?;
/// assert_eq!(key, "hi");
/// assert_eq!(value.as_str(), Ok("y'all"));
/// assert!(iter.next().is_none());
/// # Ok::<(), RawError>(())
/// ```
///
/// Individual elements can be accessed using [`doc.get(&key)`](Doc::get), or any of
/// the `get_*` methods, like [`doc.get_object_id(&key)`](Doc::get_object_id), and
/// [`doc.get_str(&str)`](Doc::get_str).  Accessing elements is an O(N) operation,
/// as it requires iterating through the document from the beginning to find the requested
/// key.
///
/// ```
/// # use bson::raw::{DocBuf, RawError};
/// let docbuf = DocBuf::new(b"\x13\x00\x00\x00\x02hi\x00\x06\x00\x00\x00y'all\x00\x00".to_vec())?;
/// assert_eq!(docbuf.get_str("hi")?, Some("y'all"));
/// # Ok::<(), RawError>(())
/// ```
#[derive(Debug)]
pub struct Doc {
    data: [u8],
}

impl Doc {
    pub fn new<D: AsRef<[u8]> + ?Sized>(data: &D) -> RawResult<&Doc> {
        let data = data.as_ref();
        if data.len() < 5 {
            return Err(RawError::MalformedValue("document too short".into()));
        }
        let length = i32_from_slice(&data[..4]);
        if data.len() as i32 != length {
            return Err(RawError::MalformedValue("document length incorrect".into()));
        }
        if data[data.len() - 1] != 0 {
            return Err(RawError::MalformedValue(
                "document not null-terminated".into(),
            ));
        }
        Ok(unsafe { Doc::new_unchecked(data) })
    }

    /// Create a new Doc referencing the provided data slice.
    ///
    /// # Safety
    ///
    /// The provided data must begin with a valid size
    /// and end with a NUL-terminator.
    ///
    /// ```
    /// # use bson::raw::{Doc, RawError};
    /// let doc: &Doc = unsafe { Doc::new_unchecked(b"\x05\0\0\0\0") };
    /// ```
    pub unsafe fn new_unchecked<D: AsRef<[u8]> + ?Sized>(data: &D) -> &Doc {
        #[allow(unused_unsafe)]
        unsafe {
            &*(data.as_ref() as *const [u8] as *const Doc)
        }
    }

    /// Create a new DocBuf with an owned copy of the data in self.
    ///
    /// ```
    /// # use bson::raw::{Doc, RawError};
    /// use bson::raw::DocBuf;
    /// let data = b"\x05\0\0\0\0";
    /// let doc = Doc::new(data)?;
    /// let docbuf: DocBuf = doc.to_docbuf();
    /// # Ok::<(), RawError>(())
    pub fn to_docbuf(&self) -> DocBuf {
        // SAFETY: The validity of the data is checked by self.
        unsafe { DocBuf::new_unchecked(self.data.to_owned()) }
    }

    /// Get an element from the document.  Finding a particular key requires
    /// iterating over the document from the beginning, so this is an O(N)
    /// operation.
    ///
    /// Returns an error if the document is malformed.  Returns `Ok(None)`
    /// if the key is not found in the document.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, elem::Element, RawError};
    /// use bson::{doc, oid::ObjectId};
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "_id": ObjectId::new(),
    ///     "f64": 2.5,
    /// });
    /// let element = docbuf.get("f64")?.expect("finding key f64");
    /// assert_eq!(element.as_f64(), Ok(2.5));
    /// assert!(docbuf.get("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get<'a>(&'a self, key: &str) -> OptResult<elem::Element<'a>> {
        for result in self.into_iter() {
            let (thiskey, bson) = result?;
            if thiskey == key {
                return Ok(Some(bson));
            }
        }
        Ok(None)
    }

    fn get_with<'a, T>(
        &'a self,
        key: &str,
        f: impl FnOnce(elem::Element<'a>) -> RawResult<T>,
    ) -> OptResult<T> {
        self.get(key)?.map(f).transpose()
    }

    /// Get an element from the document, and convert it to f64.
    ///
    /// Returns an error if the document is malformed, or if the retrieved value
    /// is not an f64.  Returns `Ok(None)` if the key is not found in the document.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, elem::Element, RawError};
    /// use bson::doc;
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "bool": true,
    ///     "f64": 2.5,
    /// });
    /// assert_eq!(docbuf.get_f64("f64"), Ok(Some(2.5)));
    /// assert_eq!(docbuf.get_f64("bool"), Err(RawError::UnexpectedType));
    /// assert_eq!(docbuf.get_f64("unknown"), Ok(None));
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_f64(&self, key: &str) -> OptResult<f64> {
        self.get_with(key, elem::Element::as_f64)
    }

    /// Get an element from the document, and convert it to a &str.
    ///
    /// The returned &str is a borrowed reference into the DocBuf.  To use it
    /// beyond the lifetime of self, call to_docbuf() on it.
    ///
    /// Returns an error if the document is malformed or if the retrieved value
    /// is not a string.  Returns `Ok(None)` if the key is not found in the
    /// document.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, elem::Element, RawError};
    /// use bson::doc;
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "string": "hello",
    ///     "bool": true,
    /// });
    /// assert_eq!(docbuf.get_str("string"), Ok(Some("hello")));
    /// assert_eq!(docbuf.get_str("bool"), Err(RawError::UnexpectedType));
    /// assert_eq!(docbuf.get_str("unknown"), Ok(None));
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_str<'a>(&'a self, key: &str) -> OptResult<&'a str> {
        self.get_with(key, elem::Element::as_str)
    }

    /// Get an element from the document, and convert it to a [Doc].
    ///
    /// The returned [Doc] is a borrowed reference into self.  To use it
    /// beyond the lifetime of self, call to_owned() on it.
    ///
    /// Returns an error if the document is malformed or if the retrieved value
    /// is not a document.  Returns `Ok(None)` if the key is not found in the
    /// document.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, elem::Element, RawError};
    /// use bson::doc;
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "doc": { "key": "value"},
    ///     "bool": true,
    /// });
    /// assert_eq!(docbuf.get_document("doc")?.expect("finding key doc").get_str("key"), Ok(Some("value")));
    /// assert_eq!(docbuf.get_document("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(docbuf.get_document("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_document<'a>(&'a self, key: &str) -> OptResult<&'a Doc> {
        self.get_with(key, elem::Element::as_document)
    }

    /// Get an element from the document, and convert it to an [ArrayRef].
    ///
    /// The returned [ArrayRef] is a borrowed reference into the DocBuf.
    ///
    /// Returns an error if the document is malformed or if the retrieved value
    /// is not a document.  Returns `Ok(None)` if the key is not found in the
    /// document.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, elem::Element, RawError};
    /// use bson::doc;
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "array": [true, 3, null],
    ///     "bool": true,
    /// });
    /// let mut arriter = docbuf.get_array("array")?.expect("finding key array").into_iter();
    /// let _: bool = arriter.next().unwrap()?.as_bool()?;
    /// let _: i32 = arriter.next().unwrap()?.as_i32()?;
    /// let () = arriter.next().unwrap()?.as_null()?;
    /// assert!(arriter.next().is_none());
    /// assert!(docbuf.get_array("bool").is_err());
    /// assert!(docbuf.get_array("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_array<'a>(&'a self, key: &str) -> OptResult<&'a Array> {
        self.get_with(key, elem::Element::as_array)
    }

    /// Get an element from the document, and convert it to an [elem::RawBsonBinary].
    ///
    /// The returned [RawBsonBinary](elem::RawBsonBinary) is a borrowed reference into the DocBuf.
    ///
    /// Returns an error if the document is malformed or if the retrieved value
    /// is not binary data.  Returns `Ok(None)` if the key is not found in the
    /// document.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, elem, RawError};
    /// use bson::{doc, Binary, spec::BinarySubtype};
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1, 2, 3] },
    ///     "bool": true,
    /// });
    /// assert_eq!(docbuf.get_binary("binary")?.map(elem::RawBsonBinary::as_bytes), Some(&[1, 2, 3][..]));
    /// assert_eq!(docbuf.get_binary("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(docbuf.get_binary("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_binary<'a>(&'a self, key: &str) -> OptResult<elem::RawBsonBinary<'a>> {
        self.get_with(key, elem::Element::as_binary)
    }

    /// Get an element from the document, and convert it to a [bson::oid::ObjectId].
    ///
    /// Returns an error if the document is malformed or if the retrieved value
    /// is not an object ID.  Returns `Ok(None)` if the key is not found in the
    /// document.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, RawError};
    /// use bson::{doc, oid::ObjectId};
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "_id": ObjectId::new(),
    ///     "bool": true,
    /// });
    /// let _: ObjectId = docbuf.get_object_id("_id")?.unwrap();
    /// assert_eq!(docbuf.get_object_id("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(docbuf.get_object_id("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_object_id(&self, key: &str) -> OptResult<oid::ObjectId> {
        self.get_with(key, elem::Element::as_object_id)
    }

    /// Get an element from the document, and convert it to a [bool].
    ///
    /// Returns an error if the document is malformed or if the retrieved value
    /// is not a boolean.  Returns `Ok(None)` if the key is not found in the
    /// document.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, RawError};
    /// use bson::{doc, oid::ObjectId};
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "_id": ObjectId::new(),
    ///     "bool": true,
    /// });
    /// assert!(docbuf.get_bool("bool")?.unwrap());
    /// assert_eq!(docbuf.get_bool("_id").unwrap_err(), RawError::UnexpectedType);
    /// assert!(docbuf.get_object_id("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_bool(&self, key: &str) -> OptResult<bool> {
        self.get_with(key, elem::Element::as_bool)
    }

    /// Get an element from the document, and convert it to a [chrono::DateTime].
    ///
    /// Returns an error if the document is malformed or if the retrieved value
    /// is not a boolean.  Returns `Ok(None)` if the key is not found in the
    /// document.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, RawError};
    /// use bson::doc;
    /// use chrono::{Utc, Datelike, TimeZone};
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "created_at": Utc.ymd(2020, 3, 15).and_hms(17, 0, 0),
    ///     "bool": true,
    /// });
    /// assert_eq!(docbuf.get_datetime("created_at")?.unwrap().year(), 2020);
    /// assert_eq!(docbuf.get_datetime("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(docbuf.get_datetime("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_datetime(&self, key: &str) -> OptResult<DateTime<Utc>> {
        self.get_with(key, elem::Element::as_datetime)
    }

    /// Get an element from the document, and convert it to the `()` type.
    ///
    /// Returns an error if the document is malformed or if the retrieved value
    /// is not null.  Returns `Ok(None)` if the key is not found in the
    /// document.
    ///
    /// There is not much reason to use the () value, so this method mostly
    /// exists for consistency with other element types, and as a way to assert
    /// type of the element.
    /// ```
    /// # use bson::raw::{DocBuf, RawError};
    /// use bson::doc;
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "null": null,
    ///     "bool": true,
    /// });
    /// docbuf.get_null("null")?.unwrap();
    /// assert_eq!(docbuf.get_null("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(docbuf.get_null("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_null(&self, key: &str) -> OptResult<()> {
        self.get_with(key, elem::Element::as_null)
    }

    /// Get an element from the document, and convert it to an [elem::RawBsonRegex].
    ///
    /// The [RawBsonRegex](elem::RawBsonRegex) borrows data from the DocBuf.
    ///
    /// Returns an error if the document is malformed or if the retrieved value
    /// is not a regex.  Returns `Ok(None)` if the key is not found in the
    /// document.
    /// ```
    /// # use bson::raw::{DocBuf, RawError, elem};
    /// use bson::{doc, Regex};
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "regex": Regex {
    ///         pattern: String::from(r"end\s*$"),
    ///         options: String::from("i"),
    ///     },
    ///     "bool": true,
    /// });
    /// assert_eq!(docbuf.get_regex("regex")?.unwrap().pattern(), r"end\s*$");
    /// assert_eq!(docbuf.get_regex("regex")?.unwrap().options(), "i");
    /// assert_eq!(docbuf.get_regex("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(docbuf.get_regex("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_regex<'a>(&'a self, key: &str) -> OptResult<elem::RawBsonRegex<'a>> {
        self.get_with(key, elem::Element::as_regex)
    }

    /// Get an element from the document, and convert it to an &str representing the
    /// javascript element type.
    ///
    /// The &str borrows data from the DocBuf.  If you need an owned copy of the data,
    /// you should call .to_owned() on the result.
    ///
    /// Returns an error if the document is malformed or if the retrieved value
    /// is not a javascript code object.  Returns `Ok(None)` if the key is not found
    /// in the document.
    /// ```
    /// # use bson::raw::{DocBuf, RawError, elem};
    /// use bson::{doc, Bson};
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "js": Bson::JavaScriptCode(String::from("console.log(\"hi y'all\");")),
    ///     "bool": true,
    /// });
    /// assert_eq!(docbuf.get_javascript("js")?, Some("console.log(\"hi y'all\");"));
    /// assert_eq!(docbuf.get_javascript("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(docbuf.get_javascript("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_javascript<'a>(&'a self, key: &str) -> OptResult<&'a str> {
        self.get_with(key, elem::Element::as_javascript)
    }

    /// Get an element from the document, and convert it to an &str representing the
    /// symbol element type.
    ///
    /// The &str borrows data from the DocBuf.  If you need an owned copy of the data,
    /// you should call .to_owned() on the result.
    ///
    /// Returns an error if the document is malformed or if the retrieved value
    /// is not a symbol object.  Returns `Ok(None)` if the key is not found
    /// in the document.
    /// ```
    /// # use bson::raw::{DocBuf, RawError, elem};
    /// use bson::{doc, Bson};
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "symbol": Bson::Symbol(String::from("internal")),
    ///     "bool": true,
    /// });
    /// assert_eq!(docbuf.get_symbol("symbol")?, Some("internal"));
    /// assert_eq!(docbuf.get_symbol("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(docbuf.get_symbol("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_symbol<'a>(&'a self, key: &str) -> OptResult<&'a str> {
        self.get_with(key, elem::Element::as_symbol)
    }

    /// Get an element from the document, and extract the data as a javascript code with scope.
    ///
    /// The return value is a `(&str, &Doc)` where the &str represents the javascript code,
    /// and the [`&Doc`](Doc) represents the scope.  Both elements borrow data from the DocBuf.
    /// If you need an owned copy of the data, you should call [js.to_owned()](ToOwned::to_owned) on
    /// the code or [scope.to_docbuf()](Doc::to_docbuf) on the scope.
    ///
    /// Returns an error if the document is malformed or if the retrieved value
    /// is not a javascript code with scope object.  Returns `Ok(None)` if the key is not found
    /// in the document.
    /// ```
    /// # use bson::raw::{DocBuf, RawError, elem};
    /// use bson::{doc, JavaScriptCodeWithScope};
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "js": JavaScriptCodeWithScope {
    ///         code: String::from("console.log(\"i:\", i);"),
    ///         scope: doc!{"i": 42},
    ///     },
    ///     "bool": true,
    /// });
    /// let (js, scope) = docbuf.get_javascript_with_scope("js")?.unwrap();
    /// assert_eq!(js, "console.log(\"i:\", i);");
    /// assert_eq!(scope.get_i32("i")?.unwrap(), 42);
    /// assert_eq!(docbuf.get_javascript_with_scope("bool").unwrap_err(), RawError::UnexpectedType);
    /// assert!(docbuf.get_javascript_with_scope("unknown")?.is_none());
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_javascript_with_scope<'a>(&'a self, key: &str) -> OptResult<(&'a str, &'a Doc)> {
        self.get_with(key, elem::Element::as_javascript_with_scope)
    }

    /// Get an element from the document, and convert it to i32.
    ///
    /// Returns an error if the document is malformed, or if the retrieved value
    /// is not an i32.  Returns `Ok(None)` if the key is not found in the document.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, RawError};
    /// use bson::doc;
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "bool": true,
    ///     "i32": 1_000_000,
    /// });
    /// assert_eq!(docbuf.get_i32("i32"), Ok(Some(1_000_000)));
    /// assert_eq!(docbuf.get_i32("bool"), Err(RawError::UnexpectedType));
    /// assert_eq!(docbuf.get_i32("unknown"), Ok(None));
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_i32(&self, key: &str) -> OptResult<i32> {
        self.get_with(key, elem::Element::as_i32)
    }

    /// Get an element from the document, and convert it to a timestamp.
    ///
    /// Returns an error if the document is malformed, or if the retrieved value
    /// is not an i32.  Returns `Ok(None)` if the key is not found in the document.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, elem, RawError};
    /// use bson::{doc, Timestamp};
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "bool": true,
    ///     "ts": Timestamp { time: 649876543, increment: 9 },
    /// });
    /// let timestamp = docbuf.get_timestamp("ts")?.unwrap();
    ///
    /// assert_eq!(timestamp.time(), 649876543);
    /// assert_eq!(timestamp.increment(), 9);
    /// assert_eq!(docbuf.get_timestamp("bool"), Err(RawError::UnexpectedType));
    /// assert_eq!(docbuf.get_timestamp("unknown"), Ok(None));
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_timestamp<'a>(&'a self, key: &str) -> OptResult<elem::RawBsonTimestamp<'a>> {
        self.get_with(key, elem::Element::as_timestamp)
    }

    /// Get an element from the document, and convert it to i64.
    ///
    /// Returns an error if the document is malformed, or if the retrieved value
    /// is not an i64.  Returns `Ok(None)` if the key is not found in the document.
    ///
    /// ```
    /// # use bson::raw::{DocBuf, elem::Element, RawError};
    /// use bson::doc;
    /// let docbuf = DocBuf::from_document(&doc! {
    ///     "bool": true,
    ///     "i64": 9223372036854775807_i64,
    /// });
    /// assert_eq!(docbuf.get_i64("i64"), Ok(Some(9223372036854775807)));
    /// assert_eq!(docbuf.get_i64("bool"), Err(RawError::UnexpectedType));
    /// assert_eq!(docbuf.get_i64("unknown"), Ok(None));
    /// # Ok::<(), RawError>(())
    /// ```
    pub fn get_i64(&self, key: &str) -> OptResult<i64> {
        self.get_with(key, elem::Element::as_i64)
    }

    /// Return a reference to the contained data as a `&[u8]`
    ///
    /// ```
    /// # use bson::raw::DocBuf;
    /// use bson::doc;
    /// let docbuf = DocBuf::from_document(&doc!{});
    /// assert_eq!(docbuf.as_bytes(), b"\x05\x00\x00\x00\x00");
    /// ```
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

impl AsRef<Doc> for Doc {
    fn as_ref(&self) -> &Doc {
        self
    }
}

impl Deref for DocBuf {
    type Target = Doc;

    fn deref(&self) -> &Self::Target {
        // SAFETY: The validity of the data is checked when creating DocBuf.
        unsafe { Doc::new_unchecked(&self.data) }
    }
}

impl TryFrom<&Doc> for crate::Document {
    type Error = RawError;

    fn try_from(rawdoc: &Doc) -> RawResult<crate::Document> {
        rawdoc
            .into_iter()
            .map(|res| res.and_then(|(k, v)| Ok((k.to_owned(), v.try_into()?))))
            .collect()
    }
}

impl<'a> IntoIterator for &'a Doc {
    type IntoIter = DocIter<'a>;
    type Item = RawResult<(&'a str, elem::Element<'a>)>;

    fn into_iter(self) -> DocIter<'a> {
        DocIter {
            doc: self,
            offset: 4,
        }
    }
}

pub struct DocIter<'a> {
    doc: &'a Doc,
    offset: usize,
}

impl<'a> Iterator for DocIter<'a> {
    type Item = RawResult<(&'a str, elem::Element<'a>)>;

    fn next(&mut self) -> Option<RawResult<(&'a str, elem::Element<'a>)>> {
        if self.offset == self.doc.data.len() - 1 {
            if self.doc.data[self.offset] == 0 {
                // end of document marker
                return None;
            } else {
                return Some(Err(RawError::MalformedValue(
                    "document not null terminated".into(),
                )));
            }
        }
        let key = match read_nullterminated(&self.doc.data[self.offset + 1..]) {
            Ok(key) => key,
            Err(err) => return Some(Err(err)),
        };
        let valueoffset = self.offset + 1 + key.len() + 1; // type specifier + key + \0
        let element_type = match ElementType::from(self.doc.data[self.offset]) {
            Some(et) => et,
            None => {
                return Some(Err(RawError::MalformedValue(format!(
                    "invalid tag: {}",
                    self.doc.data[self.offset]
                ))))
            }
        };
        let element_size = match element_type {
            ElementType::Double => 8,
            ElementType::String => {
                let size =
                    4 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;
                if self.doc.data[valueoffset + size - 1] != 0 {
                    return Some(Err(RawError::MalformedValue(
                        "string not null terminated".into(),
                    )));
                }
                size
            }
            ElementType::EmbeddedDocument => {
                let size = i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;
                if self.doc.data[valueoffset + size - 1] != 0 {
                    return Some(Err(RawError::MalformedValue(
                        "document not null terminated".into(),
                    )));
                }
                size
            }
            ElementType::Array => {
                let size = i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;
                if self.doc.data[valueoffset + size - 1] != 0 {
                    return Some(Err(RawError::MalformedValue(
                        "array not null terminated".into(),
                    )));
                }
                size
            }
            ElementType::Binary => {
                5 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize
            }
            ElementType::Undefined => 0,
            ElementType::ObjectId => 12,
            ElementType::Boolean => 1,
            ElementType::DateTime => 8,
            ElementType::Null => 0,
            ElementType::RegularExpression => {
                let regex = match read_nullterminated(&self.doc.data[valueoffset..]) {
                    Ok(regex) => regex,
                    Err(err) => return Some(Err(err)),
                };
                let options =
                    match read_nullterminated(&self.doc.data[valueoffset + regex.len() + 1..]) {
                        Ok(options) => options,
                        Err(err) => return Some(Err(err)),
                    };
                regex.len() + options.len() + 2
            }
            ElementType::DbPointer => {
                let string_size =
                    4 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;
                let id_size = 12;
                if self.doc.data[valueoffset + string_size - 1] != 0 {
                    return Some(Err(RawError::MalformedValue(
                        "DBPointer string not null-terminated".into(),
                    )));
                }
                string_size + id_size
            }
            ElementType::JavaScriptCode => {
                let size =
                    4 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;
                if self.doc.data[valueoffset + size - 1] != 0 {
                    return Some(Err(RawError::MalformedValue(
                        "javascript code not null-terminated".into(),
                    )));
                }
                size
            }
            ElementType::Symbol => {
                4 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize
            }
            ElementType::JavaScriptCodeWithScope => {
                let size = i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize;
                if self.doc.data[valueoffset + size - 1] != 0 {
                    return Some(Err(RawError::MalformedValue(
                        "javascript with scope not null-terminated".into(),
                    )));
                }
                size
            }
            ElementType::Int32 => 4,
            ElementType::Timestamp => 8,
            ElementType::Int64 => 8,
            ElementType::Decimal128 => 16,
            ElementType::MaxKey => 0,
            ElementType::MinKey => 0,
        };
        let nextoffset = valueoffset + element_size;
        self.offset = nextoffset;
        Some(Ok((
            key,
            elem::Element::new(element_type, &self.doc.data[valueoffset..nextoffset]),
        )))
    }
}

pub type ArrayRef<'a> = &'a Array;

pub struct Array {
    doc: Doc,
}

impl Array {
    pub fn new(data: &[u8]) -> RawResult<&Array> {
        Ok(Array::from_doc(Doc::new(data)?))
    }

    /// Return a new Array from the provided bytes.
    ///
    /// # Safety
    ///
    /// The provided bytes must start with a valid length indicator
    /// and end with a NUL terminator, as described in [the bson
    /// spec](http://bsonspec.org/spec.html).
    ///
    /// The following is valid:
    /// ```
    /// # use bson::raw::Array;
    /// // Represents the array [null, 514i32], which is the same as the document
    /// // {"0": null, "1": 514}
    /// let bson = b"\x0f\0\0\0\x0A0\0\x101\0\x02\x02\0\0\0";
    /// let arr = unsafe { Array::new_unchecked(bson) };
    /// let mut arriter = arr.into_iter();
    /// assert!(arriter.next().unwrap().and_then(|b| b.as_null()).is_ok());
    /// assert_eq!(arriter.next().unwrap().and_then(|b| b.as_i32()).unwrap(), 514);
    /// ```
    ///
    /// And so is this, even though the provided document is not an array, because
    /// the errors will be caught during decode.
    ///
    /// ```
    /// # use bson::raw::Array;
    /// // Represents the document {"0": null, "X": 514}
    /// let bson = b"\x0f\0\0\0\x0A0\0\x10X\0\x02\x02\0\0\0";
    /// let arr = unsafe { Array::new_unchecked(bson) };
    /// let mut arriter = arr.into_iter();
    /// assert!(arriter.next().unwrap().and_then(|b| b.as_null()).is_ok());
    /// assert!(arriter.next().unwrap().is_err());
    /// assert!(arriter.next().is_none());
    /// ```
    ///
    /// # Bad:
    ///
    /// The following, however, indicates the wrong size for the document, and is
    /// therefore unsound.
    ///
    /// ```
    /// # use bson::raw::Array;
    /// // Contains a length indicator, that is longer than the array
    /// let invalid = b"\x06\0\0\0\0";
    /// let arr: &Array = unsafe { Array::new_unchecked(invalid) };
    /// ```
    pub unsafe fn new_unchecked(data: &[u8]) -> &Array {
        #[allow(unused_unsafe)]
        let doc = unsafe { Doc::new_unchecked(data) };
        Array::from_doc(doc)
    }

    pub fn from_doc(doc: &Doc) -> &Array {
        // SAFETY: Array layout matches Doc layout
        unsafe { &*(doc as *const Doc as *const Array) }
    }

    pub fn get(&self, index: usize) -> OptResult<elem::Element<'_>> {
        self.into_iter().nth(index).transpose()
    }

    fn get_with<'a, T>(
        &'a self,
        index: usize,
        f: impl FnOnce(elem::Element<'a>) -> RawResult<T>,
    ) -> OptResult<T> {
        self.get(index)?.map(f).transpose()
    }

    pub fn get_f64(&self, index: usize) -> OptResult<f64> {
        self.get_with(index, elem::Element::as_f64)
    }

    pub fn get_str(&self, index: usize) -> OptResult<&str> {
        self.get_with(index, elem::Element::as_str)
    }

    pub fn get_document(&self, index: usize) -> OptResult<&Doc> {
        self.get_with(index, elem::Element::as_document)
    }

    pub fn get_array(&self, index: usize) -> OptResult<&Array> {
        self.get_with(index, elem::Element::as_array)
    }

    pub fn get_binary(&self, index: usize) -> OptResult<elem::RawBsonBinary<'_>> {
        self.get_with(index, elem::Element::as_binary)
    }

    pub fn get_object_id(&self, index: usize) -> OptResult<oid::ObjectId> {
        self.get_with(index, elem::Element::as_object_id)
    }

    pub fn get_bool(&self, index: usize) -> OptResult<bool> {
        self.get_with(index, elem::Element::as_bool)
    }

    pub fn get_datetime(&self, index: usize) -> OptResult<DateTime<Utc>> {
        self.get_with(index, elem::Element::as_datetime)
    }

    pub fn get_null(&self, index: usize) -> OptResult<()> {
        self.get_with(index, elem::Element::as_null)
    }

    pub fn get_regex(&self, index: usize) -> OptResult<elem::RawBsonRegex<'_>> {
        self.get_with(index, elem::Element::as_regex)
    }

    pub fn get_javascript(&self, index: usize) -> OptResult<&str> {
        self.get_with(index, elem::Element::as_javascript)
    }

    pub fn get_symbol(&self, index: usize) -> OptResult<&str> {
        self.get_with(index, elem::Element::as_symbol)
    }

    pub fn get_javascript_with_scope(&self, index: usize) -> OptResult<(&str, &Doc)> {
        self.get_with(index, elem::Element::as_javascript_with_scope)
    }

    pub fn get_i32(&self, index: usize) -> OptResult<i32> {
        self.get_with(index, elem::Element::as_i32)
    }

    pub fn get_timestamp(&self, index: usize) -> OptResult<elem::RawBsonTimestamp<'_>> {
        self.get_with(index, elem::Element::as_timestamp)
    }

    pub fn get_i64(&self, index: usize) -> OptResult<i64> {
        self.get_with(index, elem::Element::as_i64)
    }

    pub fn to_vec(&self) -> RawResult<Vec<elem::Element<'_>>> {
        self.into_iter().collect()
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.doc.as_bytes()
    }
}

impl TryFrom<&Array> for Vec<Bson> {
    type Error = RawError;

    fn try_from(arr: &Array) -> RawResult<Vec<Bson>> {
        arr.into_iter()
            .map(|result| {
                let rawbson = result?;
                Bson::try_from(rawbson)
            })
            .collect()
    }
}

impl<'a> IntoIterator for &'a Array {
    type IntoIter = ArrayIter<'a>;
    type Item = RawResult<elem::Element<'a>>;

    fn into_iter(self) -> ArrayIter<'a> {
        ArrayIter {
            dociter: self.doc.into_iter(),
            index: 0,
        }
    }
}

pub struct ArrayIter<'a> {
    dociter: DocIter<'a>,
    index: usize,
}

impl<'a> Iterator for ArrayIter<'a> {
    type Item = RawResult<elem::Element<'a>>;

    fn next(&mut self) -> Option<RawResult<elem::Element<'a>>> {
        let value = self.dociter.next().map(|result| {
            let (key, bson) = match result {
                Ok(value) => value,
                Err(err) => return Err(err),
            };

            let index: usize = key
                .parse()
                .map_err(|_| RawError::MalformedValue("non-integer array index found".into()))?;

            if index == self.index {
                Ok(bson)
            } else {
                Err(RawError::MalformedValue("wrong array index found".into()))
            }
        });
        self.index += 1;
        value
    }
}
/// Given a 4 byte u8 slice, return an i32 calculated from the bytes in
/// little endian order
///
/// # Panics
///
/// This function panics if given a slice that is not four bytes long.
fn i32_from_slice(val: &[u8]) -> i32 {
    i32::from_le_bytes(val.try_into().expect("i32 is four bytes"))
}

/// Given an 8 byte u8 slice, return an i64 calculated from the bytes in
/// little endian order
///
/// # Panics
///
/// This function panics if given a slice that is not eight bytes long.
fn i64_from_slice(val: &[u8]) -> i64 {
    i64::from_le_bytes(val.try_into().expect("i64 is eight bytes"))
}

/// Given a 4 byte u8 slice, return a u32 calculated from the bytes in
/// little endian order
///
/// # Panics
///
/// This function panics if given a slice that is not four bytes long.
fn u32_from_slice(val: &[u8]) -> u32 {
    u32::from_le_bytes(val.try_into().expect("u32 is four bytes"))
}

#[cfg(feature = "decimal128")]
fn d128_from_slice(val: &[u8]) -> Decimal128 {
    // TODO: Handle Big Endian platforms
    let d =
        unsafe { decimal::d128::from_raw_bytes(val.try_into().expect("d128 is sixteen bytes")) };
    Decimal128::from(d)
}

fn read_nullterminated(buf: &[u8]) -> RawResult<&str> {
    let mut splits = buf.splitn(2, |x| *x == 0);
    let value = splits
        .next()
        .ok_or_else(|| RawError::MalformedValue("no value".into()))?;
    if splits.next().is_some() {
        Ok(try_to_str(value)?)
    } else {
        Err(RawError::MalformedValue("expected null terminator".into()))
    }
}

fn read_lenencoded(buf: &[u8]) -> RawResult<&str> {
    let length = i32_from_slice(&buf[..4]);
    assert!(buf.len() as i32 >= length + 4);
    try_to_str(&buf[4..4 + length as usize - 1])
}

fn try_to_str(data: &[u8]) -> RawResult<&str> {
    match std::str::from_utf8(data) {
        Ok(s) => Ok(s),
        Err(_) => Err(RawError::Utf8EncodingError(data.into())),
    }
}

pub type DocRef<'a> = &'a Doc;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        doc, spec::BinarySubtype, Binary, Bson, JavaScriptCodeWithScope, Regex, Timestamp,
    };
    use chrono::TimeZone;

    fn to_bytes(doc: &crate::Document) -> Vec<u8> {
        let mut docbytes = Vec::new();
        doc.to_writer(&mut docbytes).unwrap();
        docbytes
    }

    #[test]
    fn string_from_document() {
        let docbytes = to_bytes(&doc! {
            "this": "first",
            "that": "second",
            "something": "else",
        });
        let rawdoc = Doc::new(&docbytes).unwrap();
        assert_eq!(
            rawdoc.get("that").unwrap().unwrap().as_str().unwrap(),
            "second",
        );
    }

    #[test]
    fn nested_document() {
        let docbytes = to_bytes(&doc! {
            "outer": {
                "inner": "surprise",
            },
        });
        let rawdoc = Doc::new(&docbytes).unwrap();
        assert_eq!(
            rawdoc
                .get("outer")
                .expect("get doc result")
                .expect("get doc option")
                .as_document()
                .expect("as doc")
                .get("inner")
                .expect("get str result")
                .expect("get str option")
                .as_str()
                .expect("as str"),
            "surprise",
        );
    }

    #[test]
    fn iterate() {
        let docbytes = to_bytes(&doc! {
            "apples": "oranges",
            "peanut butter": "chocolate",
            "easy as": {"do": 1, "re": 2, "mi": 3},
        });
        let rawdoc = Doc::new(&docbytes).expect("malformed bson document");
        let mut dociter = rawdoc.into_iter();
        let next = dociter.next().expect("no result").expect("invalid bson");
        assert_eq!(next.0, "apples");
        assert_eq!(next.1.as_str().expect("result was not a str"), "oranges");
        let next = dociter.next().expect("no result").expect("invalid bson");
        assert_eq!(next.0, "peanut butter");
        assert_eq!(next.1.as_str().expect("result was not a str"), "chocolate");
        let next = dociter.next().expect("no result").expect("invalid bson");
        assert_eq!(next.0, "easy as");
        let _doc = next.1.as_document().expect("result was a not a document");
        let next = dociter.next();
        assert!(next.is_none());
    }

    #[test]
    fn rawdoc_to_doc() {
        let docbytes = to_bytes(&doc! {
            "f64": 2.5,
            "string": "hello",
            "document": {},
            "array": ["binary", "serialized", "object", "notation"],
            "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1, 2, 3] },
            "object_id": oid::ObjectId::with_bytes([1, 2, 3, 4, 5,6,7,8,9,10, 11,12]),
            "boolean": true,
            "datetime": Utc::now(),
            "null": Bson::Null,
            "regex": Bson::RegularExpression(Regex { pattern: String::from(r"end\s*$"), options: String::from("i")}),
            "javascript": Bson::JavaScriptCode(String::from("console.log(console);")),
            "symbol": Bson::Symbol(String::from("artist-formerly-known-as")),
            "javascript_with_scope": Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope{ code: String::from("console.log(msg);"), scope: doc!{"ok": true}}),
            "int32": 23i32,
            "timestamp": Bson::Timestamp(Timestamp { time: 3542578, increment: 0 }),
            "int64": 46i64,
            "end": "END",
        });

        let rawdoc = Doc::new(&docbytes).expect("invalid document");
        let _doc: crate::Document = rawdoc.try_into().expect("invalid bson");
    }

    #[test]
    fn f64() {
        #![allow(clippy::float_cmp)]

        let rawdoc = DocBuf::from_document(&doc! {"f64": 2.5});
        assert_eq!(
            rawdoc
                .get("f64")
                .expect("error finding key f64")
                .expect("no key f64")
                .as_f64()
                .expect("result was not a f64"),
            2.5,
        );
    }

    #[test]
    fn string() {
        let rawdoc = DocBuf::from_document(&doc! {"string": "hello"});

        assert_eq!(
            rawdoc
                .get("string")
                .expect("error finding key string")
                .expect("no key string")
                .as_str()
                .expect("result was not a string"),
            "hello",
        );
    }
    #[test]
    fn document() {
        let rawdoc = DocBuf::from_document(&doc! {"document": {}});

        let doc = rawdoc
            .get("document")
            .expect("error finding key document")
            .expect("no key document")
            .as_document()
            .expect("result was not a document");
        assert_eq!(&doc.data, [5, 0, 0, 0, 0].as_ref()); // Empty document
    }

    #[test]
    fn array() {
        let rawdoc =
            DocBuf::from_document(&doc! { "array": ["binary", "serialized", "object", "notation"]});

        let array = rawdoc
            .get("array")
            .expect("error finding key array")
            .expect("no key array")
            .as_array()
            .expect("result was not an array");
        assert_eq!(array.get_str(0), Ok(Some("binary")));
        assert_eq!(array.get_str(3), Ok(Some("notation")));
        assert_eq!(array.get_str(4), Ok(None));
    }

    #[test]
    fn binary() {
        let rawdoc = DocBuf::from_document(&doc! {
            "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] }
        });
        let binary: elem::RawBsonBinary<'_> = rawdoc
            .get("binary")
            .expect("error finding key binary")
            .expect("no key binary")
            .as_binary()
            .expect("result was not a binary object");
        assert_eq!(binary.subtype, BinarySubtype::Generic);
        assert_eq!(binary.data, &[1, 2, 3]);
    }

    #[test]
    fn object_id() {
        let rawdoc = DocBuf::from_document(&doc! {
            "object_id": oid::ObjectId::with_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
        });
        let oid = rawdoc
            .get("object_id")
            .expect("error finding key object_id")
            .expect("no key object_id")
            .as_object_id()
            .expect("result was not an object id");
        assert_eq!(oid.to_hex(), "0102030405060708090a0b0c");
    }

    #[test]
    fn boolean() {
        let rawdoc = DocBuf::from_document(&doc! {
            "boolean": true,
        });

        let boolean = rawdoc
            .get("boolean")
            .expect("error finding key boolean")
            .expect("no key boolean")
            .as_bool()
            .expect("result was not boolean");

        assert_eq!(boolean, true);
    }

    #[test]
    fn datetime() {
        let rawdoc = DocBuf::from_document(&doc! {
            "boolean": true,
            "datetime": Utc.ymd(2000,10,31).and_hms(12, 30, 45),
        });
        let datetime = rawdoc
            .get("datetime")
            .expect("error finding key datetime")
            .expect("no key datetime")
            .as_datetime()
            .expect("result was not datetime");
        assert_eq!(datetime.to_rfc3339(), "2000-10-31T12:30:45+00:00");
    }

    #[test]
    fn null() {
        let rawdoc = DocBuf::from_document(&doc! {
            "null": null,
        });
        let () = rawdoc
            .get("null")
            .expect("error finding key null")
            .expect("no key null")
            .as_null()
            .expect("was not null");
    }

    #[test]
    fn regex() {
        let rawdoc = DocBuf::from_document(&doc! {
            "regex": Bson::RegularExpression(Regex { pattern: String::from(r"end\s*$"), options: String::from("i")}),
        });
        let regex = rawdoc
            .get("regex")
            .expect("error finding key regex")
            .expect("no key regex")
            .as_regex()
            .expect("was not regex");
        assert_eq!(regex.pattern, r"end\s*$");
        assert_eq!(regex.options, "i");
    }
    #[test]
    fn javascript() {
        let rawdoc = DocBuf::from_document(&doc! {
            "javascript": Bson::JavaScriptCode(String::from("console.log(console);")),
        });
        let js = rawdoc
            .get("javascript")
            .expect("error finding key javascript")
            .expect("no key javascript")
            .as_javascript()
            .expect("was not javascript");
        assert_eq!(js, "console.log(console);");
    }

    #[test]
    fn symbol() {
        let rawdoc = DocBuf::from_document(&doc! {
            "symbol": Bson::Symbol(String::from("artist-formerly-known-as")),
        });

        let symbol = rawdoc
            .get("symbol")
            .expect("error finding key symbol")
            .expect("no key symbol")
            .as_symbol()
            .expect("was not symbol");
        assert_eq!(symbol, "artist-formerly-known-as");
    }

    #[test]
    fn javascript_with_scope() {
        let rawdoc = DocBuf::from_document(&doc! {
            "javascript_with_scope": Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope{ code: String::from("console.log(msg);"), scope: doc!{"ok": true}}),
        });
        let (js, scopedoc) = rawdoc
            .get("javascript_with_scope")
            .expect("error finding key javascript_with_scope")
            .expect("no key javascript_with_scope")
            .as_javascript_with_scope()
            .expect("was not javascript with scope");
        assert_eq!(js, "console.log(msg);");
        let (scope_key, scope_value_bson) = scopedoc
            .into_iter()
            .next()
            .expect("no next value in scope")
            .expect("invalid element");
        assert_eq!(scope_key, "ok");
        let scope_value = scope_value_bson.as_bool().expect("not a boolean");
        assert_eq!(scope_value, true);
    }

    #[test]
    fn int32() {
        let rawdoc = DocBuf::from_document(&doc! {
            "int32": 23i32,
        });
        let int32 = rawdoc
            .get("int32")
            .expect("error finding key int32")
            .expect("no key int32")
            .as_i32()
            .expect("was not int32");
        assert_eq!(int32, 23i32);
    }

    #[test]
    fn timestamp() {
        let rawdoc = DocBuf::from_document(&doc! {
            "timestamp": Bson::Timestamp(Timestamp { time: 3542578, increment: 7 }),
        });
        let ts = rawdoc
            .get("timestamp")
            .expect("error finding key timestamp")
            .expect("no key timestamp")
            .as_timestamp()
            .expect("was not a timestamp");

        assert_eq!(ts.increment(), 7);
        assert_eq!(ts.time(), 3542578);
    }

    #[test]
    fn int64() {
        let rawdoc = DocBuf::from_document(&doc! {
            "int64": 46i64,
        });
        let int64 = rawdoc
            .get("int64")
            .expect("error finding key int64")
            .expect("no key int64")
            .as_i64()
            .expect("was not int64");
        assert_eq!(int64, 46i64);
    }
    #[test]
    fn document_iteration() {
        let docbytes = to_bytes(&doc! {
            "f64": 2.5,
            "string": "hello",
            "document": {},
            "array": ["binary", "serialized", "object", "notation"],
            "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] },
            "object_id": oid::ObjectId::with_bytes([1, 2, 3, 4, 5,6,7,8,9,10, 11,12]),
            "boolean": true,
            "datetime": Utc::now(),
            "null": Bson::Null,
            "regex": Bson::RegularExpression(Regex { pattern: String::from(r"end\s*$"), options: String::from("i")}),
            "javascript": Bson::JavaScriptCode(String::from("console.log(console);")),
            "symbol": Bson::Symbol(String::from("artist-formerly-known-as")),
            "javascript_with_scope": Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope{ code: String::from("console.log(msg);"), scope: doc!{"ok": true}}),
            "int32": 23i32,
            "timestamp": Bson::Timestamp(Timestamp { time: 3542578, increment: 0 }),
            "int64": 46i64,
            "end": "END",
        });
        let rawdoc = unsafe { Doc::new_unchecked(&docbytes) };

        assert_eq!(
            rawdoc
                .into_iter()
                .collect::<Result<Vec<(&str, _)>, RawError>>()
                .expect("collecting iterated doc")
                .len(),
            17
        );
        let end = rawdoc
            .get("end")
            .expect("error finding key end")
            .expect("no key end")
            .as_str()
            .expect("was not str");
        assert_eq!(end, "END");
    }

    #[test]
    fn into_bson_conversion() {
        let docbytes = to_bytes(&doc! {
            "f64": 2.5,
            "string": "hello",
            "document": {},
            "array": ["binary", "serialized", "object", "notation"],
            "object_id": oid::ObjectId::with_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
            "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] },
            "boolean": false,
        });
        let rawbson = elem::Element::new(ElementType::EmbeddedDocument, &docbytes);
        let b: Bson = rawbson.try_into().expect("invalid bson");
        let doc = b.as_document().expect("not a document");
        assert_eq!(*doc.get("f64").expect("f64 not found"), Bson::Double(2.5));
        assert_eq!(
            *doc.get("string").expect("string not found"),
            Bson::String(String::from("hello"))
        );
        assert_eq!(
            *doc.get("document").expect("document not found"),
            Bson::Document(doc! {})
        );
        assert_eq!(
            *doc.get("array").expect("array not found"),
            Bson::Array(
                vec!["binary", "serialized", "object", "notation"]
                    .into_iter()
                    .map(|s| Bson::String(String::from(s)))
                    .collect()
            )
        );
        assert_eq!(
            *doc.get("object_id").expect("object_id not found"),
            Bson::ObjectId(oid::ObjectId::with_bytes([
                1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12
            ]))
        );
        assert_eq!(
            *doc.get("binary").expect("binary not found"),
            Bson::Binary(Binary {
                subtype: BinarySubtype::Generic,
                bytes: vec![1, 2, 3]
            })
        );
        assert_eq!(
            *doc.get("boolean").expect("boolean not found"),
            Bson::Boolean(false)
        );
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;
    use std::convert::TryInto;

    use super::props::arbitrary_bson;
    use super::DocBuf;
    use crate::doc;

    fn to_bytes(doc: &crate::Document) -> Vec<u8> {
        let mut docbytes = Vec::new();
        doc.to_writer(&mut docbytes).unwrap();
        docbytes
    }

    proptest! {
        #[test]
        fn no_crashes(s: Vec<u8>) {
            let _ = DocBuf::new(s);
        }

        #[test]
        fn roundtrip_bson(bson in arbitrary_bson()) {
            println!("{:?}", bson);
            let doc = doc!{"bson": bson};
            let raw = to_bytes(&doc);
            let raw = DocBuf::new(raw);
            prop_assert!(raw.is_ok());
            let raw = raw.unwrap();
            let roundtrip: Result<crate::Document, _> = raw.try_into();
            prop_assert!(roundtrip.is_ok());
            let roundtrip = roundtrip.unwrap();
            prop_assert_eq!(doc, roundtrip);
        }
    }
}
