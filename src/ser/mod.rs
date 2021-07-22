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

use std::{io::Write, iter::FromIterator, mem};

#[cfg(feature = "decimal128")]
use crate::decimal128::Decimal128;
use crate::{
    bson::{Binary, Bson, DbPointer, Document, JavaScriptCodeWithScope, Regex},
    de::MAX_BSON_SIZE,
    spec::BinarySubtype,
};
use ::serde::{ser::Error as SerdeError, Serialize};

fn write_string<W: Write + ?Sized>(writer: &mut W, s: &str) -> Result<()> {
    writer.write_all(&(s.len() as i32 + 1).to_le_bytes())?;
    writer.write_all(s.as_bytes())?;
    writer.write_all(b"\0")?;
    Ok(())
}

fn write_cstring<W: Write + ?Sized>(writer: &mut W, s: &str) -> Result<()> {
    writer.write_all(s.as_bytes())?;
    writer.write_all(b"\0")?;
    Ok(())
}

#[inline]
pub(crate) fn write_i32<W: Write + ?Sized>(writer: &mut W, val: i32) -> Result<()> {
    writer
        .write_all(&val.to_le_bytes())
        .map(|_| ())
        .map_err(From::from)
}

#[inline]
fn write_i64<W: Write + ?Sized>(writer: &mut W, val: i64) -> Result<()> {
    writer
        .write_all(&val.to_le_bytes())
        .map(|_| ())
        .map_err(From::from)
}

#[inline]
fn write_f64<W: Write + ?Sized>(writer: &mut W, val: f64) -> Result<()> {
    writer
        .write_all(&val.to_le_bytes())
        .map(|_| ())
        .map_err(From::from)
}

#[cfg(feature = "decimal128")]
#[inline]
fn write_f128<W: Write + ?Sized>(writer: &mut W, val: Decimal128) -> Result<()> {
    let raw = val.to_raw_bytes_le();
    writer.write_all(&raw).map_err(From::from)
}

#[inline]
fn write_binary<W: Write>(mut writer: W, bytes: &[u8], subtype: BinarySubtype) -> Result<()> {
    let len = if let BinarySubtype::BinaryOld = subtype {
        bytes.len() + 4
    } else {
        bytes.len()
    };

    if len > MAX_BSON_SIZE as usize {
        return Err(Error::custom(format!(
            "binary length {} exceeded maximum size",
            bytes.len()
        )));
    }

    write_i32(&mut writer, len as i32)?;
    writer.write_all(&[subtype.into()])?;

    if let BinarySubtype::BinaryOld = subtype {
        write_i32(&mut writer, len as i32 - 4)?;
    };

    writer.write_all(bytes).map_err(From::from)
}

fn serialize_array<W: Write + ?Sized>(writer: &mut W, arr: &[Bson]) -> Result<()> {
    let mut buf = Vec::new();
    for (key, val) in arr.iter().enumerate() {
        serialize_bson(&mut buf, &key.to_string(), val)?;
    }

    write_i32(
        writer,
        (buf.len() + mem::size_of::<i32>() + mem::size_of::<u8>()) as i32,
    )?;
    writer.write_all(&buf)?;
    writer.write_all(b"\0")?;
    Ok(())
}

pub(crate) fn serialize_bson<W: Write + ?Sized>(
    writer: &mut W,
    key: &str,
    val: &Bson,
) -> Result<()> {
    writer.write_all(&[val.element_type() as u8])?;
    write_cstring(writer, key)?;

    match *val {
        Bson::Double(v) => write_f64(writer, v),
        Bson::String(ref v) => write_string(writer, v),
        Bson::Array(ref v) => serialize_array(writer, v),
        Bson::Document(ref v) => v.to_writer(writer),
        Bson::Boolean(v) => writer
            .write_all(&[if v { 0x01 } else { 0x00 }])
            .map_err(From::from),
        Bson::RegularExpression(Regex {
            ref pattern,
            ref options,
        }) => {
            write_cstring(writer, pattern)?;

            let mut chars: Vec<char> = options.chars().collect();
            chars.sort_unstable();

            write_cstring(writer, String::from_iter(chars).as_str())
        }
        Bson::JavaScriptCode(ref code) => write_string(writer, code),
        Bson::ObjectId(ref id) => writer.write_all(&id.bytes()).map_err(From::from),
        Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope {
            ref code,
            ref scope,
        }) => {
            let mut buf = Vec::new();
            write_string(&mut buf, code)?;
            scope.to_writer(&mut buf)?;

            write_i32(writer, buf.len() as i32 + 4)?;
            writer.write_all(&buf).map_err(From::from)
        }
        Bson::Int32(v) => write_i32(writer, v),
        Bson::Int64(v) => write_i64(writer, v),
        Bson::Timestamp(ts) => write_i64(writer, ts.to_le_i64()),
        Bson::Binary(Binary { subtype, ref bytes }) => write_binary(writer, bytes, subtype),
        Bson::DateTime(ref v) => write_i64(writer, v.timestamp_millis()),
        Bson::Null => Ok(()),
        Bson::Symbol(ref v) => write_string(writer, v),
        #[cfg(not(feature = "decimal128"))]
        Bson::Decimal128(ref v) => {
            writer.write_all(&v.bytes)?;
            Ok(())
        }
        #[cfg(feature = "decimal128")]
        Bson::Decimal128(ref v) => write_f128(writer, v.clone()),
        Bson::Undefined => Ok(()),
        Bson::MinKey => Ok(()),
        Bson::MaxKey => Ok(()),
        Bson::DbPointer(DbPointer {
            ref namespace,
            ref id,
        }) => {
            write_string(writer, namespace)?;
            writer.write_all(&id.bytes()).map_err(From::from)
        }
    }
}

/// Encode a `T` Serializable into a BSON `Value`.
pub fn to_bson<T: ?Sized>(value: &T) -> Result<Bson>
where
    T: Serialize,
{
    let ser = Serializer::new();
    value.serialize(ser)
}

/// Encode a `T` Serializable into a BSON `Document`.
pub fn to_document<T: ?Sized>(value: &T) -> Result<Document>
where
    T: Serialize,
{
    match to_bson(value)? {
        Bson::Document(doc) => Ok(doc),
        bson => Err(Error::SerializationError {
            message: format!(
                "Could not be serialized to Document, got {:?} instead",
                bson.element_type()
            ),
        }),
    }
}

/// Serialize the given `T` as BSON bytes into the provided writer.
#[inline]
pub fn to_writer<T, W>(value: &T, mut writer: W) -> Result<()>
where
    T: Serialize,
    W: Write,
{
    let mut serializer = raw::Serializer::new();
    value.serialize(&mut serializer)?;
    writer.write_all(serializer.into_vec().as_slice())?;
    Ok(())
}

/// Serialize the given `T` as a BSON byte vector.
#[inline]
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: Serialize,
{
    let mut serializer = raw::Serializer::new();
    value.serialize(&mut serializer)?;
    Ok(serializer.into_vec())
}
