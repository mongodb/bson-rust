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
mod serde;

pub use self::{
    error::{Error, Result},
    serde::Deserializer,
};

use std::io::Read;

use chrono::{
    offset::{LocalResult, TimeZone},
    Utc,
};

use crate::{
    bson::{Array, Binary, Bson, DbPointer, Document, JavaScriptCodeWithScope, Regex, Timestamp},
    oid,
    spec::{self, BinarySubtype},
    Decimal128,
};

use ::serde::{
    de::{DeserializeOwned, Error as _, Unexpected},
    Deserialize,
};

const MAX_BSON_SIZE: i32 = 16 * 1024 * 1024;
pub(crate) const MIN_BSON_DOCUMENT_SIZE: i32 = 4 + 1; // 4 bytes for length, one byte for null terminator
const MIN_BSON_STRING_SIZE: i32 = 4 + 1; // 4 bytes for length, one byte for null terminator
const MIN_CODE_WITH_SCOPE_SIZE: i32 = 4 + MIN_BSON_STRING_SIZE + MIN_BSON_DOCUMENT_SIZE;

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

fn read_string<R: Read + ?Sized>(reader: &mut R, utf8_lossy: bool) -> Result<String> {
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
fn read_u8<R: Read + ?Sized>(reader: &mut R) -> Result<u8> {
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
fn read_i64<R: Read + ?Sized>(reader: &mut R) -> Result<i64> {
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
#[cfg(not(feature = "decimal128"))]
#[inline]
fn read_f128<R: Read + ?Sized>(reader: &mut R) -> Result<Decimal128> {
    let mut buf = [0u8; 128 / 8];
    reader.read_exact(&mut buf)?;
    Ok(Decimal128 { bytes: buf })
}

#[cfg(feature = "decimal128")]
#[inline]
fn read_f128<R: Read + ?Sized>(reader: &mut R) -> Result<Decimal128> {
    use std::mem;

    let mut local_buf: [u8; 16] = unsafe { mem::MaybeUninit::uninit().assume_init() };
    reader.read_exact(&mut local_buf)?;
    let val = unsafe { Decimal128::from_raw_bytes_le(local_buf) };
    Ok(val)
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
        Some(ElementType::Binary) => {
            let mut len = read_i32(reader)?;
            if !(0..=MAX_BSON_SIZE).contains(&len) {
                return Err(Error::invalid_length(
                    len as usize,
                    &format!("binary length must be between 0 and {}", MAX_BSON_SIZE).as_str(),
                ));
            }
            let subtype = BinarySubtype::from(read_u8(reader)?);

            // Skip length data in old binary.
            if let BinarySubtype::BinaryOld = subtype {
                let data_len = read_i32(reader)?;

                if !(0..=(MAX_BSON_SIZE - 4)).contains(&data_len) {
                    return Err(Error::invalid_length(
                        data_len as usize,
                        &format!("0x02 length must be between 0 and {}", MAX_BSON_SIZE - 4)
                            .as_str(),
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
            Bson::Binary(Binary { subtype, bytes })
        }
        Some(ElementType::ObjectId) => {
            let mut objid = [0; 12];
            for x in &mut objid {
                *x = read_u8(reader)?;
            }
            Bson::ObjectId(oid::ObjectId::with_bytes(objid))
        }
        Some(ElementType::Boolean) => {
            let val = read_u8(reader)?;
            if val > 1 {
                return Err(Error::invalid_value(
                    Unexpected::Unsigned(val as u64),
                    &"boolean must be stored as 0 or 1",
                ));
            }

            Bson::Boolean(val != 0)
        }
        Some(ElementType::Null) => Bson::Null,
        Some(ElementType::RegularExpression) => {
            let pattern = read_cstring(reader)?;

            let mut options: Vec<_> = read_cstring(reader)?.chars().collect();
            options.sort_unstable();

            Bson::RegularExpression(Regex {
                pattern,
                options: options.into_iter().collect(),
            })
        }
        Some(ElementType::JavaScriptCode) => {
            read_string(reader, utf8_lossy).map(Bson::JavaScriptCode)?
        }
        Some(ElementType::JavaScriptCodeWithScope) => {
            let length = read_i32(reader)?;
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
            Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope { code, scope })
        }
        Some(ElementType::Int32) => read_i32(reader).map(Bson::Int32)?,
        Some(ElementType::Int64) => read_i64(reader).map(Bson::Int64)?,
        Some(ElementType::Timestamp) => {
            read_i64(reader).map(|val| Bson::Timestamp(Timestamp::from_le_i64(val)))?
        }
        Some(ElementType::DateTime) => {
            // The int64 is UTC milliseconds since the Unix epoch.
            let time = read_i64(reader)?;

            let mut sec = time / 1000;
            let tmp_msec = time % 1000;
            let msec = if tmp_msec < 0 {
                sec -= 1;
                1000 + tmp_msec
            } else {
                tmp_msec
            };

            match Utc.timestamp_opt(sec, (msec as u32) * 1_000_000) {
                LocalResult::None => return Err(Error::InvalidTimestamp(time)),
                LocalResult::Ambiguous(..) => return Err(Error::AmbiguousTimestamp(time)),
                LocalResult::Single(t) => Bson::DateTime(t),
            }
        }
        Some(ElementType::Symbol) => read_string(reader, utf8_lossy).map(Bson::Symbol)?,
        Some(ElementType::Decimal128) => read_f128(reader).map(Bson::Decimal128)?,
        Some(ElementType::Undefined) => Bson::Undefined,
        Some(ElementType::DbPointer) => {
            let namespace = read_string(reader, utf8_lossy)?;
            let mut objid = [0; 12];
            reader.read_exact(&mut objid)?;
            Bson::DbPointer(DbPointer {
                namespace,
                id: oid::ObjectId::with_bytes(objid),
            })
        }
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

/// Decode a BSON `Value` into a `T` Deserializable.
pub fn from_bson<T>(bson: Bson) -> Result<T>
where
    T: DeserializeOwned,
{
    let de = Deserializer::new(bson);
    Deserialize::deserialize(de)
}

/// Decode a BSON `Document` into a `T` Deserializable.
pub fn from_document<T>(doc: Document) -> Result<T>
where
    T: DeserializeOwned,
{
    from_bson(Bson::Document(doc))
}
