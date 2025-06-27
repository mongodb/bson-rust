// The MIT License (MIT)

// Copyright (c) 2015 Y. T. Chung <zonyitoo@gmail.com>

// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
// FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
// IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! Serializer

mod error;
mod raw;
mod serde;

pub use self::{
    error::{Error, Result},
    serde::Serializer,
};

#[rustfmt::skip]
use ::serde::{ser::Error as SerdeError, Serialize};

use crate::{
    bson::{Bson, Document},
    ser::serde::SerializerOptions,
    RawDocumentBuf,
};

/// Encode a `T` Serializable into a [`Bson`] value.
///
/// The [`Serializer`] used by this function presents itself as human readable, whereas the
/// one used in [`serialize_to_vec`] does not. This means that this function will produce different
/// BSON than [`serialize_to_vec`] for types that change their serialization output depending on
/// whether the format is human readable or not.
pub fn serialize_to_bson<T>(value: &T) -> Result<Bson>
where
    T: Serialize + ?Sized,
{
    let ser = Serializer::new();
    #[cfg(feature = "serde_path_to_error")]
    {
        serde_path_to_error::serialize(value, ser).map_err(Error::with_path)
    }
    #[cfg(not(feature = "serde_path_to_error"))]
    value.serialize(ser)
}

/// Internal-only method to serialize data to BSON with the given options.
pub(crate) fn to_bson_with_options<T>(value: &T, options: SerializerOptions) -> Result<Bson>
where
    T: Serialize + ?Sized,
{
    let ser = Serializer::new_with_options(options);
    value.serialize(ser)
}

/// Serialize a `T` Serializable into a BSON [`Document`].
///
/// The [`Serializer`] used by this function presents itself as human readable, whereas the
/// one used in [`serialize_to_vec`] does not. This means that this function will produce different
/// BSON than [`serialize_to_vec`] for types that change their serialization output depending on
/// whether the format is human readable or not.
pub fn serialize_to_document<T>(value: &T) -> Result<Document>
where
    T: Serialize + ?Sized,
{
    match serialize_to_bson(value)? {
        Bson::Document(doc) => Ok(doc),
        bson => Err(Error::SerializationError {
            message: format!(
                "Could not be serialized to Document, got {:?} instead",
                bson.element_type()
            ),
        }),
    }
}

/// Serialize the given `T` as a BSON byte vector.
#[inline]
pub fn serialize_to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut serializer = raw::Serializer::new();
    #[cfg(feature = "serde_path_to_error")]
    {
        serde_path_to_error::serialize(value, &mut serializer).map_err(Error::with_path)?;
    }
    #[cfg(not(feature = "serde_path_to_error"))]
    {
        value.serialize(&mut serializer)?;
    }
    Ok(serializer.into_vec())
}

/// Serialize the given `T` as a [`RawDocumentBuf`].
///
/// ```rust
/// use serde::Serialize;
/// use bson::rawdoc;
///
/// #[derive(Serialize)]
/// struct Cat {
///     name: String,
///     age: i32
/// }
///
/// let cat = Cat { name: "Garfield".to_string(), age: 43 };
/// let doc = bson::serialize_to_raw_document_buf(&cat)?;
/// assert_eq!(doc, rawdoc! { "name": "Garfield", "age": 43 });
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[inline]
pub fn serialize_to_raw_document_buf<T>(value: &T) -> Result<RawDocumentBuf>
where
    T: Serialize,
{
    RawDocumentBuf::decode_from_bytes(serialize_to_vec(value)?).map_err(Error::custom)
}
