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

//! Decoder

mod error;
mod serde;

pub use self::error::{DecoderError, DecoderResult};
pub use self::serde::Decoder;

use std::io::Read;

use byteorder::{LittleEndian, ReadBytesExt};
use chrono::offset::{LocalResult, TimeZone};
use chrono::Utc;

use bson::{Array, Bson, Document};
use oid;
use spec::{self, BinarySubtype};

use serde::de::Deserialize;

fn read_string<R: Read + ?Sized>(reader: &mut R, utf8_lossy: bool) -> DecoderResult<String> {
    let len = reader.read_i32::<LittleEndian>()?;

    // UTF-8 String must have at least 1 byte (the last 0x00).
    if len < 1 {
        return Err(DecoderError::InvalidLength(len as usize, format!("invalid length {} for UTF-8 string", len)));
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
    reader.read_u8()?; // The last 0x00

    Ok(s)
}

fn read_cstring<R: Read + ?Sized>(reader: &mut R) -> DecoderResult<String> {
    let mut v = Vec::new();

    loop {
        let c = reader.read_u8()?;
        if c == 0 {
            break;
        }
        v.push(c);
    }

    Ok(String::from_utf8(v)?)
}

#[inline]
fn read_i32<R: Read + ?Sized>(reader: &mut R) -> DecoderResult<i32> {
    reader.read_i32::<LittleEndian>().map_err(From::from)
}

#[inline]
fn read_i64<R: Read + ?Sized>(reader: &mut R) -> DecoderResult<i64> {
    reader.read_i64::<LittleEndian>().map_err(From::from)
}

/// Attempt to decode a `Document` from a byte stream.
pub fn decode_document<R: Read + ?Sized>(reader: &mut R) -> DecoderResult<Document> {
    let mut doc = Document::new();

    // disregard the length: using Read::take causes infinite type recursion
    read_i32(reader)?;

    loop {
        let tag = reader.read_u8()?;

        if tag == 0 {
            break;
        }

        let key = read_cstring(reader)?;
        let val = decode_bson(reader, tag, false)?;

        doc.insert(key, val);
    }

    Ok(doc)
}

/// Attempt to decode a `Document` that may contain invalid UTF-8 strings from a byte stream.
pub fn decode_document_utf8_lossy<R: Read + ?Sized>(reader: &mut R) -> DecoderResult<Document> {
    let mut doc = Document::new();

    // disregard the length: using Read::take causes infinite type recursion
    read_i32(reader)?;

    loop {
        let tag = reader.read_u8()?;

        if tag == 0 {
            break;
        }

        let key = read_cstring(reader)?;
        let val = decode_bson(reader, tag, true)?;

        doc.insert(key, val);
    }

    Ok(doc)
}

fn decode_array<R: Read + ?Sized>(reader: &mut R, utf8_lossy: bool) -> DecoderResult<Array> {
    let mut arr = Array::new();

    // disregard the length: using Read::take causes infinite type recursion
    read_i32(reader)?;

    loop {
        let tag = reader.read_u8()?;
        if tag == 0 {
            break;
        }

        // check that the key is as expected
        let key = read_cstring(reader)?;
        match key.parse::<usize>() {
            Err(..) => return Err(DecoderError::InvalidArrayKey(arr.len(), key)),
            Ok(idx) => {
                if idx != arr.len() {
                    return Err(DecoderError::InvalidArrayKey(arr.len(), key));
                }
            }
        }

        let val = decode_bson(reader, tag, utf8_lossy)?;
        arr.push(val)
    }

    Ok(arr)
}

fn decode_bson<R: Read + ?Sized>(reader: &mut R, tag: u8, utf8_lossy: bool) -> DecoderResult<Bson> {
    use spec::ElementType::*;
    match spec::ElementType::from(tag) {
        Some(FloatingPoint) => Ok(Bson::FloatingPoint(reader.read_f64::<LittleEndian>()?)),
        Some(Utf8String) => read_string(reader, utf8_lossy).map(Bson::String),
        Some(EmbeddedDocument) => decode_document(reader).map(Bson::Document),
        Some(Array) => decode_array(reader, utf8_lossy).map(Bson::Array),
        Some(Binary) => {
            let len = read_i32(reader)?;
            let subtype = BinarySubtype::from(reader.read_u8()?);
            let mut data = Vec::with_capacity(len as usize);
            reader.take(len as u64).read_to_end(&mut data)?;
            Ok(Bson::Binary(subtype, data))
        }
        Some(ObjectId) => {
            let mut objid = [0; 12];
            for x in &mut objid {
                *x = reader.read_u8()?;
            }
            Ok(Bson::ObjectId(oid::ObjectId::with_bytes(objid)))
        }
        Some(Boolean) => Ok(Bson::Boolean(reader.read_u8()? != 0)),
        Some(NullValue) => Ok(Bson::Null),
        Some(RegularExpression) => {
            let pat = read_cstring(reader)?;
            let opt = read_cstring(reader)?;
            Ok(Bson::RegExp(pat, opt))
        }
        Some(JavaScriptCode) => read_string(reader, utf8_lossy).map(Bson::JavaScriptCode),
        Some(JavaScriptCodeWithScope) => {
            // disregard the length:
            //     using Read::take causes infinite type recursion
            read_i32(reader)?;

            let code = read_string(reader, utf8_lossy)?;
            let scope = decode_document(reader)?;
            Ok(Bson::JavaScriptCodeWithScope(code, scope))
        }
        Some(Integer32Bit) => read_i32(reader).map(Bson::I32),
        Some(Integer64Bit) => read_i64(reader).map(Bson::I64),
        Some(TimeStamp) => read_i64(reader).map(Bson::TimeStamp),
        Some(UtcDatetime) => {
            let time = read_i64(reader)?;

            match Utc.timestamp_opt(time / 1000, (time % 1000) as u32 * 1000000) {
                LocalResult::None => Err(DecoderError::InvalidTimestamp(time)),
                LocalResult::Ambiguous(..) => Err(DecoderError::AmbiguousTimestamp(time)),
                LocalResult::Single(t) => Ok(Bson::UtcDatetime(t)),
            }
        }
        Some(Symbol) => read_string(reader, utf8_lossy).map(Bson::Symbol),
        Some(Undefined) | Some(DbPointer) | Some(MaxKey) | Some(MinKey) | None => {
            Err(DecoderError::UnrecognizedElementType(tag))
        }
    }
}

/// Decode a BSON `Value` into a `T` Deserializable.
pub fn from_bson<'de, T>(bson: Bson) -> DecoderResult<T>
    where T: Deserialize<'de>
{
    let de = Decoder::new(bson);
    Deserialize::deserialize(de)
}
