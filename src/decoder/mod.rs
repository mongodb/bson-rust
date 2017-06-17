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
use std::mem::size_of;

use byteorder::{LittleEndian, ReadBytesExt};
use chrono::UTC;
use chrono::offset::TimeZone;

use spec::{self, BinarySubtype};
use bson::{Bson, Array, Document};
use oid;

use serde::de::Deserialize;

fn read_string<R: Read + ?Sized>(reader: &mut R) -> DecoderResult<String> {
    let len = try!(reader.read_i32::<LittleEndian>());

    let mut s = String::with_capacity(len as usize - 1);
    try!(reader.take(len as u64 - 1).read_to_string(&mut s));
    try!(reader.read_u8()); // The last 0x00

    Ok(s)
}

fn read_cstring<R: Read + ?Sized>(reader: &mut R) -> DecoderResult<String> {
    let mut v = Vec::new();

    loop {
        let c = try!(reader.read_u8());
        if c == 0 {
            break;
        }
        v.push(c);
    }

    Ok(try!(String::from_utf8(v)))
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
    decode_document_and_get_length(reader).map(|(_, doc)| doc)
}

/// Attempt to decode a `Document` from a byte stream, returning the length as well.
pub fn decode_document_and_get_length<R: Read + ?Sized>(
    reader: &mut R,
) -> DecoderResult<(usize, Document)> {
    let mut doc = Document::new();
    let mut length = 0;

    // disregard the length: using Read::take causes infinite type recursion
    try!(read_i32(reader));
    length += size_of::<i32>();

    loop {
        let tag = try!(reader.read_u8());
        length += size_of::<u8>();

        if tag == 0 {
            break;
        }

        let key = try!(read_cstring(reader));
        length += key.len() + 1;
        let (val_length, val) = try!(decode_bson_and_get_length(reader, tag));
        length += val_length;

        doc.insert(key, val);
    }

    Ok((length, doc))
}

fn decode_array_and_get_length<R: Read + ?Sized>(reader: &mut R) -> DecoderResult<(usize, Array)> {
    let mut arr = Array::new();
    let mut length = 0;

    // disregard the length: using Read::take causes infinite type recursion
    try!(read_i32(reader));
    length += size_of::<i32>();

    loop {
        let tag = try!(reader.read_u8());
        length += size_of::<u8>();

        if tag == 0 {
            break;
        }

        // check that the key is as expected
        let key = try!(read_cstring(reader));
        length += key.len() + 1;

        match key.parse::<usize>() {
            Err(..) => return Err(DecoderError::InvalidArrayKey(arr.len(), key)),
            Ok(idx) => {
                if idx != arr.len() {
                    return Err(DecoderError::InvalidArrayKey(arr.len(), key));
                }
            }
        }

        let (val_length, val) = try!(decode_bson_and_get_length(reader, tag));
        length += val_length;
        arr.push(val)
    }

    Ok((length, arr))
}

fn decode_bson_and_get_length<R: Read + ?Sized>(
    reader: &mut R,
    tag: u8,
) -> DecoderResult<(usize, Bson)> {
    use spec::ElementType::*;
    match spec::ElementType::from(tag) {
        Some(FloatingPoint) => {
            Ok((
                size_of::<f64>(),
                Bson::FloatingPoint(try!(reader.read_f64::<LittleEndian>())),
            ))
        }
        Some(Utf8String) => {
            read_string(reader).map(|s| (size_of::<i32>() + s.len() + 1, Bson::String(s)))
        }
        Some(EmbeddedDocument) => {
            decode_document_and_get_length(reader).map(
                |(length, doc)| {
                    (length, Bson::Document(doc))
                },
            )
        }
        Some(Array) => {
            decode_array_and_get_length(reader).map(|(length, array)| (length, Bson::Array(array)))
        }
        Some(Binary) => {
            let len = try!(read_i32(reader)) as usize;
            let subtype = BinarySubtype::from(try!(reader.read_u8()));
            let mut data = Vec::with_capacity(len);
            try!(reader.take(len as u64).read_to_end(&mut data));
            Ok((
                size_of::<i32>() + size_of::<u8>() + len,
                Bson::Binary(subtype, data),
            ))
        }
        Some(ObjectId) => {
            let mut objid = [0; 12];
            for x in &mut objid {
                *x = try!(reader.read_u8());
            }
            Ok((12, Bson::ObjectId(oid::ObjectId::with_bytes(objid))))
        }
        Some(Boolean) => Ok((1, Bson::Boolean(try!(reader.read_u8()) != 0))),
        Some(NullValue) => Ok((0, Bson::Null)),
        Some(RegularExpression) => {
            let pat = try!(read_cstring(reader));
            let opt = try!(read_cstring(reader));
            Ok((pat.len() + opt.len() + 2, Bson::RegExp(pat, opt)))
        }
        Some(JavaScriptCode) => {
            read_string(reader).map(|s| {
                (size_of::<i32>() + s.len() + 1, Bson::JavaScriptCode(s))
            })
        }
        Some(JavaScriptCodeWithScope) => {
            // disregard the length:
            //     using Read::take causes infinite type recursion
            try!(read_i32(reader));

            let code = try!(read_string(reader));
            let (scope_length, scope) = try!(decode_document_and_get_length(reader));
            let length = 2 * size_of::<i32>() + code.len() + 1 + scope_length;
            Ok((length, Bson::JavaScriptCodeWithScope(code, scope)))
        }
        Some(Integer32Bit) => read_i32(reader).map(|i| (size_of::<i32>(), Bson::I32(i))),
        Some(Integer64Bit) => read_i64(reader).map(|i| (size_of::<i64>(), Bson::I64(i))),
        Some(TimeStamp) => read_i64(reader).map(|i| (size_of::<i64>(), Bson::TimeStamp(i))),
        Some(UtcDatetime) => {
            let time = try!(read_i64(reader));
            let timestamp = UTC.timestamp(time / 1000, (time % 1000) as u32 * 1000000);
            Ok((size_of::<i64>(), Bson::UtcDatetime(timestamp)))
        }
        Some(Symbol) => {
            read_string(reader).map(|s| (size_of::<i32>() + s.len() + 1, Bson::Symbol(s)))
        }
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
