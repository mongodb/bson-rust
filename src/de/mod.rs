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

//! Deserializer

mod error;
mod raw;
mod serde;

pub use self::{
    error::{Error, Result},
    serde::{Deserializer, DeserializerOptions},
};

use std::io::Read;

use crate::{
    bson::{Array, Bson, DbPointer, Document, JavaScriptCodeWithScope, Regex, Timestamp},
    oid::{self, ObjectId},
    raw::RawBinaryRef,
    ser::write_i32,
    spec::{self, BinarySubtype},
    Binary,
    Decimal128,
};

use ::serde::{
    de::{DeserializeOwned, Error as _, Unexpected},
    Deserialize,
};

pub(crate) use self::serde::{convert_unsigned_to_signed_raw, BsonVisitor};

#[cfg(test)]
pub(crate) use self::raw::Deserializer as RawDeserializer;

pub(crate) const MAX_BSON_SIZE: i32 = 16 * 1024 * 1024;
pub(crate) const MIN_BSON_DOCUMENT_SIZE: i32 = 4 + 1; // 4 bytes for length, one byte for null terminator
pub(crate) const MIN_BSON_STRING_SIZE: i32 = 4 + 1; // 4 bytes for length, one byte for null terminator
pub(crate) const MIN_CODE_WITH_SCOPE_SIZE: i32 = 4 + MIN_BSON_STRING_SIZE + MIN_BSON_DOCUMENT_SIZE;

/// Hint provided to the deserializer via `deserialize_newtype_struct` as to the type of thing
/// being deserialized.
#[derive(Debug, Clone, Copy)]
enum DeserializerHint {
    /// No hint provided, deserialize normally.
    None,

    /// The type being deserialized expects the BSON to contain a binary value with the provided
    /// subtype. This is currently used to deserialize [`bson::Uuid`] values.
    BinarySubtype(BinarySubtype),

    /// The type being deserialized is raw BSON, meaning no allocations should occur as part of
    /// deserializing and everything should be visited via borrowing or [`Copy`] if possible.
    RawBson,
}

/// Run the provided closure, ensuring that over the course of its execution, exactly `length` bytes
/// were read from the reader.
pub(crate) fn ensure_read_exactly<F, R>(
    reader: &mut R,
    length: usize,
    error_message: &str,
    func: F,
) -> Result<()>
where
    F: FnOnce(&mut std::io::Cursor<Vec<u8>>) -> Result<()>,
    R: Read + ?Sized,
{
    let mut buf = vec![0u8; length];
    reader.read_exact(&mut buf)?;
    let mut cursor = std::io::Cursor::new(buf);

    func(&mut cursor)?;

    if cursor.position() != length as u64 {
        return Err(Error::invalid_length(length, &error_message));
    }
    Ok(())
}

pub(crate) fn read_string<R: Read + ?Sized>(reader: &mut R, utf8_lossy: bool) -> Result<String> {
    let len = read_i32(reader)?;

    // UTF-8 String must have at least 1 byte (the last 0x00).
    if len < 1 {
        return Err(Error::invalid_length(
            len as usize,
            &"UTF-8 string must have at least 1 byte",
        ));
    }

    let s = if utf8_lossy {
        let mut buf = Vec::with_capacity(len as usize - 1);
        reader.take(len as u64 - 1).read_to_end(&mut buf)?;
        String::from_utf8_lossy(&buf).to_string()
    } else {
        let mut s = String::with_capacity(len as usize - 1);
        reader.take(len as u64 - 1).read_to_string(&mut s)?;
        s
    };

    // read the null terminator
    if read_u8(reader)? != 0 {
        return Err(Error::invalid_length(
            len as usize,
            &"contents of string longer than provided length",
        ));
    }

    Ok(s)
}

pub(crate) fn read_bool<R: Read>(mut reader: R) -> Result<bool> {
    let val = read_u8(&mut reader)?;
    if val > 1 {
        return Err(Error::invalid_value(
            Unexpected::Unsigned(val as u64),
            &"boolean must be stored as 0 or 1",
        ));
    }

    Ok(val != 0)
}

fn read_cstring<R: Read + ?Sized>(reader: &mut R) -> Result<String> {
    let mut v = Vec::new();

    loop {
        let c = read_u8(reader)?;
        if c == 0 {
            break;
        }
        v.push(c);
    }

    Ok(String::from_utf8(v)?)
}

#[inline]
pub(crate) fn read_u8<R: Read + ?Sized>(reader: &mut R) -> Result<u8> {
    let mut buf = [0; 1];
    reader.read_exact(&mut buf)?;
    Ok(u8::from_le_bytes(buf))
}

#[inline]
pub(crate) fn read_i32<R: Read + ?Sized>(reader: &mut R) -> Result<i32> {
    let mut buf = [0; 4];
    reader.read_exact(&mut buf)?;
    Ok(i32::from_le_bytes(buf))
}

#[inline]
pub(crate) fn read_i64<R: Read + ?Sized>(reader: &mut R) -> Result<i64> {
    let mut buf = [0; 8];
    reader.read_exact(&mut buf)?;
    Ok(i64::from_le_bytes(buf))
}

#[inline]
fn read_f64<R: Read + ?Sized>(reader: &mut R) -> Result<f64> {
    let mut buf = [0; 8];
    reader.read_exact(&mut buf)?;
    Ok(f64::from_le_bytes(buf))
}

/// Placeholder decoder for `Decimal128`. Reads 128 bits and just stores them, does no validation or
/// parsing.
#[inline]
fn read_f128<R: Read + ?Sized>(reader: &mut R) -> Result<Decimal128> {
    let mut buf = [0u8; 128 / 8];
    reader.read_exact(&mut buf)?;
    Ok(Decimal128 { bytes: buf })
}

fn deserialize_array<R: Read + ?Sized>(reader: &mut R, utf8_lossy: bool) -> Result<Array> {
    let mut arr = Array::new();
    let length = read_i32(reader)?;

    if !(MIN_BSON_DOCUMENT_SIZE..=MAX_BSON_SIZE).contains(&length) {
        return Err(Error::invalid_length(
            length as usize,
            &format!(
                "array length must be between {} and {}",
                MIN_BSON_DOCUMENT_SIZE, MAX_BSON_SIZE
            )
            .as_str(),
        ));
    }

    ensure_read_exactly(
        reader,
        (length as usize) - 4,
        "array length longer than contents",
        |cursor| {
            loop {
                let tag = read_u8(cursor)?;
                if tag == 0 {
                    break;
                }

                let (_, val) = deserialize_bson_kvp(cursor, tag, utf8_lossy)?;
                arr.push(val)
            }
            Ok(())
        },
    )?;

    Ok(arr)
}

pub(crate) fn deserialize_bson_kvp<R: Read + ?Sized>(
    reader: &mut R,
    tag: u8,
    utf8_lossy: bool,
) -> Result<(String, Bson)> {
    use spec::ElementType;
    let key = read_cstring(reader)?;

    let val = match ElementType::from(tag) {
        Some(ElementType::Double) => Bson::Double(read_f64(reader)?),
        Some(ElementType::String) => read_string(reader, utf8_lossy).map(Bson::String)?,
        Some(ElementType::EmbeddedDocument) => Document::from_reader(reader).map(Bson::Document)?,
        Some(ElementType::Array) => deserialize_array(reader, utf8_lossy).map(Bson::Array)?,
        Some(ElementType::Binary) => Bson::Binary(Binary::from_reader(reader)?),
        Some(ElementType::ObjectId) => {
            let mut objid = [0; 12];
            for x in &mut objid {
                *x = read_u8(reader)?;
            }
            Bson::ObjectId(oid::ObjectId::from_bytes(objid))
        }
        Some(ElementType::Boolean) => Bson::Boolean(read_bool(reader)?),
        Some(ElementType::Null) => Bson::Null,
        Some(ElementType::RegularExpression) => {
            Bson::RegularExpression(Regex::from_reader(reader)?)
        }
        Some(ElementType::JavaScriptCode) => {
            read_string(reader, utf8_lossy).map(Bson::JavaScriptCode)?
        }
        Some(ElementType::JavaScriptCodeWithScope) => {
            Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope::from_reader(reader, utf8_lossy)?)
        }
        Some(ElementType::Int32) => read_i32(reader).map(Bson::Int32)?,
        Some(ElementType::Int64) => read_i64(reader).map(Bson::Int64)?,
        Some(ElementType::Timestamp) => Bson::Timestamp(Timestamp::from_reader(reader)?),
        Some(ElementType::DateTime) => {
            // The int64 is UTC milliseconds since the Unix epoch.
            let time = read_i64(reader)?;
            Bson::DateTime(crate::DateTime::from_millis(time))
        }
        Some(ElementType::Symbol) => read_string(reader, utf8_lossy).map(Bson::Symbol)?,
        Some(ElementType::Decimal128) => read_f128(reader).map(Bson::Decimal128)?,
        Some(ElementType::Undefined) => Bson::Undefined,
        Some(ElementType::DbPointer) => Bson::DbPointer(DbPointer::from_reader(reader)?),
        Some(ElementType::MaxKey) => Bson::MaxKey,
        Some(ElementType::MinKey) => Bson::MinKey,
        None => {
            return Err(Error::UnrecognizedDocumentElementType {
                key,
                element_type: tag,
            })
        }
    };

    Ok((key, val))
}

impl Binary {
    pub(crate) fn from_reader<R: Read>(mut reader: R) -> Result<Self> {
        let mut len = read_i32(&mut reader)?;
        if !(0..=MAX_BSON_SIZE).contains(&len) {
            return Err(Error::invalid_length(
                len as usize,
                &format!("binary length must be between 0 and {}", MAX_BSON_SIZE).as_str(),
            ));
        }
        let subtype = BinarySubtype::from(read_u8(&mut reader)?);

        // Skip length data in old binary.
        if let BinarySubtype::BinaryOld = subtype {
            let data_len = read_i32(&mut reader)?;

            if !(0..=(MAX_BSON_SIZE - 4)).contains(&data_len) {
                return Err(Error::invalid_length(
                    data_len as usize,
                    &format!("0x02 length must be between 0 and {}", MAX_BSON_SIZE - 4).as_str(),
                ));
            }

            if data_len + 4 != len {
                return Err(Error::invalid_length(
                    data_len as usize,
                    &"0x02 length did not match top level binary length",
                ));
            }

            len -= 4;
        }

        let mut bytes = Vec::with_capacity(len as usize);

        reader.take(len as u64).read_to_end(&mut bytes)?;
        Ok(Binary { subtype, bytes })
    }
}

impl<'a> RawBinaryRef<'a> {
    pub(crate) fn from_slice_with_len_and_payload(
        mut bytes: &'a [u8],
        mut len: i32,
        subtype: BinarySubtype,
    ) -> Result<Self> {
        if !(0..=MAX_BSON_SIZE).contains(&len) {
            return Err(Error::invalid_length(
                len as usize,
                &format!("binary length must be between 0 and {}", MAX_BSON_SIZE).as_str(),
            ));
        } else if len as usize > bytes.len() {
            return Err(Error::invalid_length(
                len as usize,
                &format!(
                    "binary length {} exceeds buffer length {}",
                    len,
                    bytes.len()
                )
                .as_str(),
            ));
        }

        // Skip length data in old binary.
        if let BinarySubtype::BinaryOld = subtype {
            let data_len = read_i32(&mut bytes)?;

            if data_len + 4 != len {
                return Err(Error::invalid_length(
                    data_len as usize,
                    &"0x02 length did not match top level binary length",
                ));
            }

            len -= 4;
        }

        Ok(Self {
            bytes: &bytes[0..len as usize],
            subtype,
        })
    }
}

impl DbPointer {
    pub(crate) fn from_reader<R: Read>(mut reader: R) -> Result<Self> {
        let ns = read_string(&mut reader, false)?;
        let oid = ObjectId::from_reader(&mut reader)?;
        Ok(DbPointer {
            namespace: ns,
            id: oid,
        })
    }
}

impl Regex {
    pub(crate) fn from_reader<R: Read>(mut reader: R) -> Result<Self> {
        let pattern = read_cstring(&mut reader)?;
        let options = read_cstring(&mut reader)?;

        Ok(Regex { pattern, options })
    }
}

impl Timestamp {
    pub(crate) fn from_reader<R: Read>(mut reader: R) -> Result<Self> {
        read_i64(&mut reader).map(Timestamp::from_le_i64)
    }
}

impl ObjectId {
    pub(crate) fn from_reader<R: Read>(mut reader: R) -> Result<Self> {
        let mut buf = [0u8; 12];
        reader.read_exact(&mut buf)?;
        Ok(Self::from_bytes(buf))
    }
}

impl JavaScriptCodeWithScope {
    pub(crate) fn from_reader<R: Read>(mut reader: R, utf8_lossy: bool) -> Result<Self> {
        let length = read_i32(&mut reader)?;
        if length < MIN_CODE_WITH_SCOPE_SIZE {
            return Err(Error::invalid_length(
                length as usize,
                &format!(
                    "code with scope length must be at least {}",
                    MIN_CODE_WITH_SCOPE_SIZE
                )
                .as_str(),
            ));
        } else if length > MAX_BSON_SIZE {
            return Err(Error::invalid_length(
                length as usize,
                &"code with scope length too large",
            ));
        }

        let mut buf = vec![0u8; (length - 4) as usize];
        reader.read_exact(&mut buf)?;

        let mut slice = buf.as_slice();
        let code = read_string(&mut slice, utf8_lossy)?;
        let scope = Document::from_reader(&mut slice)?;
        Ok(JavaScriptCodeWithScope { code, scope })
    }
}

/// Deserialize a `T` from the provided [`Bson`] value.
///
/// The [`Deserializer`] used by this function presents itself as human readable, whereas the
/// one used in [`from_slice`] does not. This means that this function may deserialize differently
/// than [`from_slice`] for types that change their deserialization logic depending on whether
/// the format is human readable or not. To deserialize from [`Bson`] with a deserializer that
/// presents itself as not human readable, use [`from_bson_with_options`] with
/// [`DeserializerOptions::human_readable`] set to false.
pub fn from_bson<T>(bson: Bson) -> Result<T>
where
    T: DeserializeOwned,
{
    let de = Deserializer::new(bson);
    Deserialize::deserialize(de)
}

/// Deserialize a `T` from the provided [`Bson`] value, configuring the underlying
/// deserializer with the provided options.
/// ```
/// # use serde::Deserialize;
/// # use bson::{bson, DeserializerOptions};
/// #[derive(Debug, Deserialize, PartialEq)]
/// struct MyData {
///     a: String,
/// }
///
/// let bson = bson!({ "a": "hello" });
/// let options = DeserializerOptions::builder().human_readable(false).build();
/// let data: MyData = bson::from_bson_with_options(bson, options)?;
/// assert_eq!(data, MyData { a: "hello".to_string() });
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn from_bson_with_options<T>(bson: Bson, options: DeserializerOptions) -> Result<T>
where
    T: DeserializeOwned,
{
    let de = Deserializer::new_with_options(bson, options);
    Deserialize::deserialize(de)
}

/// Deserialize a `T` from the provided [`Document`].
///
/// The [`Deserializer`] used by this function presents itself as human readable, whereas the
/// one used in [`from_slice`] does not. This means that this function may deserialize differently
/// than [`from_slice`] for types that change their deserialization logic depending on whether
/// the format is human readable or not. To deserialize from [`Document`] with a deserializer that
/// presents itself as not human readable, use [`from_document_with_options`] with
/// [`DeserializerOptions::human_readable`] set to false.
pub fn from_document<T>(doc: Document) -> Result<T>
where
    T: DeserializeOwned,
{
    from_bson(Bson::Document(doc))
}

/// Deserialize a `T` from the provided [`Document`], configuring the underlying
/// deserializer with the provided options.
/// ```
/// # use serde::Deserialize;
/// # use bson::{doc, DeserializerOptions};
/// #[derive(Debug, Deserialize, PartialEq)]
/// struct MyData {
///     a: String,
/// }
///
/// let doc = doc! { "a": "hello" };
/// let options = DeserializerOptions::builder().human_readable(false).build();
/// let data: MyData = bson::from_document_with_options(doc, options)?;
/// assert_eq!(data, MyData { a: "hello".to_string() });
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn from_document_with_options<T>(doc: Document, options: DeserializerOptions) -> Result<T>
where
    T: DeserializeOwned,
{
    let de = Deserializer::new_with_options(Bson::Document(doc), options);
    Deserialize::deserialize(de)
}

fn reader_to_vec<R: Read>(mut reader: R) -> Result<Vec<u8>> {
    let length = read_i32(&mut reader)?;

    if length < MIN_BSON_DOCUMENT_SIZE {
        return Err(Error::custom("document size too small"));
    }

    let mut bytes = Vec::with_capacity(length as usize);
    write_i32(&mut bytes, length).map_err(Error::custom)?;

    reader.take(length as u64 - 4).read_to_end(&mut bytes)?;
    Ok(bytes)
}

/// Deserialize an instance of type `T` from an I/O stream of BSON.
pub fn from_reader<R, T>(reader: R) -> Result<T>
where
    T: DeserializeOwned,
    R: Read,
{
    let bytes = reader_to_vec(reader)?;
    from_slice(bytes.as_slice())
}

/// Deserialize an instance of type `T` from an I/O stream of BSON, replacing any invalid UTF-8
/// sequences with the Unicode replacement character.
///
/// This is mainly useful when reading raw BSON returned from a MongoDB server, which
/// in rare cases can contain invalidly truncated strings (<https://jira.mongodb.org/browse/SERVER-24007>).
/// For most use cases, [`crate::from_reader`] can be used instead.
pub fn from_reader_utf8_lossy<R, T>(reader: R) -> Result<T>
where
    T: DeserializeOwned,
    R: Read,
{
    let bytes = reader_to_vec(reader)?;
    from_slice_utf8_lossy(bytes.as_slice())
}

/// Deserialize an instance of type `T` from a slice of BSON bytes.
pub fn from_slice<'de, T>(bytes: &'de [u8]) -> Result<T>
where
    T: Deserialize<'de>,
{
    let mut deserializer = raw::Deserializer::new(bytes, false);
    T::deserialize(&mut deserializer)
}

/// Deserialize an instance of type `T` from a slice of BSON bytes, replacing any invalid UTF-8
/// sequences with the Unicode replacement character.
///
/// This is mainly useful when reading raw BSON returned from a MongoDB server, which
/// in rare cases can contain invalidly truncated strings (<https://jira.mongodb.org/browse/SERVER-24007>).
/// For most use cases, [`crate::from_slice`] can be used instead.
pub fn from_slice_utf8_lossy<'de, T>(bytes: &'de [u8]) -> Result<T>
where
    T: Deserialize<'de>,
{
    let mut deserializer = raw::Deserializer::new(bytes, true);
    T::deserialize(&mut deserializer)
}
