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

//! Encoder

mod error;
mod serde;

pub use self::error::{EncoderError, EncoderResult};
pub use self::serde::Encoder;

use std::io::Write;
use std::iter::IntoIterator;
use std::mem;

use chrono::Timelike;
use byteorder::{LittleEndian, WriteBytesExt};

use bson::Bson;
use serde::Serialize;

fn write_string<W: Write + ?Sized>(writer: &mut W, s: &str) -> EncoderResult<()> {
    writer.write_i32::<LittleEndian>(s.len() as i32 + 1)?;
    writer.write_all(s.as_bytes())?;
    writer.write_u8(0)?;
    Ok(())
}

fn write_cstring<W: Write + ?Sized>(writer: &mut W, s: &str) -> EncoderResult<()> {
    writer.write_all(s.as_bytes())?;
    writer.write_u8(0)?;
    Ok(())
}

#[inline]
fn write_i32<W: Write + ?Sized>(writer: &mut W, val: i32) -> EncoderResult<()> {
    writer.write_i32::<LittleEndian>(val).map_err(From::from)
}

#[inline]
fn write_i64<W: Write + ?Sized>(writer: &mut W, val: i64) -> EncoderResult<()> {
    writer.write_i64::<LittleEndian>(val).map_err(From::from)
}

#[inline]
fn write_f64<W: Write + ?Sized>(writer: &mut W, val: f64) -> EncoderResult<()> {
    writer.write_f64::<LittleEndian>(val).map_err(From::from)
}

fn encode_array<W: Write + ?Sized>(writer: &mut W, arr: &[Bson]) -> EncoderResult<()> {
    let mut buf = Vec::new();
    for (key, val) in arr.iter().enumerate() {
        encode_bson(&mut buf, &key.to_string(), val)?;
    }

    write_i32(
        writer,
        (buf.len() + mem::size_of::<i32>() + mem::size_of::<u8>()) as i32
    )?;
    writer.write_all(&buf)?;
    writer.write_u8(0)?;
    Ok(())
}

/// Attempt to encode a `Document` into a byte stream.
///
/// Can encode any type which is iterable as `(key: &str, value: &Bson)` pairs,
/// which generally means most maps.
pub fn encode_document<'a,
                       S: AsRef<str> + 'a,
                       W: Write + ?Sized,
                       D: IntoIterator<Item = (&'a S, &'a Bson)>>
    (writer: &mut W,
     doc: D)
     -> EncoderResult<()> {
    let mut buf = Vec::new();
    for (key, val) in doc.into_iter() {
        encode_bson(&mut buf, key.as_ref(), val)?;
    }

    write_i32(
        writer,
        (buf.len() + mem::size_of::<i32>() + mem::size_of::<u8>()) as i32
    )?;
    writer.write_all(&buf)?;
    writer.write_u8(0)?;
    Ok(())
}

fn encode_bson<W: Write + ?Sized>(writer: &mut W, key: &str, val: &Bson) -> EncoderResult<()> {
    writer.write_u8(val.element_type() as u8)?;
    write_cstring(writer, key)?;

    match val {
        &Bson::FloatingPoint(v) => write_f64(writer, v),
        &Bson::String(ref v) => write_string(writer, &v),
        &Bson::Array(ref v) => encode_array(writer, &v),
        &Bson::Document(ref v) => encode_document(writer, v),
        &Bson::Boolean(v) => writer.write_u8(if v { 0x01 } else { 0x00 }).map_err(From::from),
        &Bson::RegExp(ref pat, ref opt) => {
            write_cstring(writer, pat)?;
            write_cstring(writer, opt)
        }
        &Bson::JavaScriptCode(ref code) => write_string(writer, &code),
        &Bson::ObjectId(ref id) => writer.write_all(&id.bytes()).map_err(From::from),
        &Bson::JavaScriptCodeWithScope(ref code, ref scope) => {
            let mut buf = Vec::new();
            write_string(&mut buf, code)?;
            encode_document(&mut buf, scope)?;

            write_i32(writer, buf.len() as i32 + 4)?;
            writer.write_all(&buf).map_err(From::from)
        }
        &Bson::I32(v) => write_i32(writer, v),
        &Bson::I64(v) => write_i64(writer, v),
        &Bson::TimeStamp(v) => write_i64(writer, v),
        &Bson::Binary(subtype, ref data) => {
            write_i32(writer, data.len() as i32)?;
            writer.write_u8(From::from(subtype))?;
            writer.write_all(data).map_err(From::from)
        }
        &Bson::UtcDatetime(ref v) => {
            write_i64(writer,
                      (v.timestamp() * 1000) + (v.nanosecond() / 1000000) as i64)
        }
        &Bson::Null => Ok(()),
        &Bson::Symbol(ref v) => write_string(writer, &v),
    }
}

/// Encode a `T` Serializable into a BSON `Value`.
pub fn to_bson<T: ?Sized>(value: &T) -> EncoderResult<Bson>
    where T: Serialize
{
    let ser = Encoder::new();
    value.serialize(ser)
}
