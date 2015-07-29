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

use std::io::{self, Read};
use std::{str, error, fmt};

use byteorder::{self, LittleEndian, ReadBytesExt};
use chrono::{DateTime, NaiveDateTime, UTC};

use spec::{self, BinarySubtype};
use bson::{Bson, Array, Document};
use oid;

/// Possible errors that can arise during decoding.
#[derive(Debug)]
pub enum DecoderError {
    IoError(io::Error),
    Utf8Error(str::Utf8Error),
    UnrecognizedElementType(u8),
    InvalidArrayKey(usize, String)
}

impl From<io::Error> for DecoderError {
    fn from(err: io::Error) -> DecoderError {
        DecoderError::IoError(err)
    }
}

impl From<str::Utf8Error> for DecoderError {
    fn from(err: str::Utf8Error) -> DecoderError {
        DecoderError::Utf8Error(err)
    }
}

impl From<byteorder::Error> for DecoderError {
    fn from(err: byteorder::Error) -> DecoderError {
        DecoderError::IoError(From::from(err))
    }
}

impl fmt::Display for DecoderError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &DecoderError::IoError(ref inner) => inner.fmt(fmt),
            &DecoderError::Utf8Error(ref inner) => inner.fmt(fmt),
            &DecoderError::UnrecognizedElementType(tag) => {
                write!(fmt, "unrecognized element type `{}`", tag)
            }
            &DecoderError::InvalidArrayKey(ref want, ref got) => {
                write!(fmt, "invalid array key: expected `{}`, got `{}`", want, got)
            }
        }
    }
}

impl error::Error for DecoderError {
    fn description(&self) -> &str {
        match self {
            &DecoderError::IoError(ref inner) => inner.description(),
            &DecoderError::Utf8Error(ref inner) => inner.description(),
            &DecoderError::UnrecognizedElementType(_) => "unrecognized element type",
            &DecoderError::InvalidArrayKey(_, _) => "invalid array key"
        }
    }
    fn cause(&self) -> Option<&error::Error> {
        match self {
            &DecoderError::IoError(ref inner) => Some(inner),
            &DecoderError::Utf8Error(ref inner) => Some(inner),
            _ => None
        }
    }
}

/// Alias for `Result<T, DecoderError>`.
pub type DecoderResult<T> = Result<T, DecoderError>;

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
        if c == 0 { break; }
        v.push(c);
    }

    Ok(try!(str::from_utf8(&v)).to_owned())
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
    try!(read_i32(reader));

    loop {
        let tag = try!(reader.read_u8());

        if tag == 0 {
            break;
        }

        let key = try!(read_cstring(reader));
        let val = try!(decode_bson(reader, tag));

        doc.insert(key, val);
    }

    Ok(doc)
}

fn decode_array<R: Read + ?Sized>(reader: &mut R) -> DecoderResult<Array> {
    let mut arr = Array::new();

    // disregard the length: using Read::take causes infinite type recursion
    try!(read_i32(reader));

    loop {
        let tag = try!(reader.read_u8());
        if tag == 0 {
            break;
        }

        // check that the key is as expected
        let key = try!(read_cstring(reader));
        if key != &arr.len().to_string()[..] {
            return Err(DecoderError::InvalidArrayKey(arr.len(), key));
        }

        let val = try!(decode_bson(reader, tag));
        arr.push(val)
    }

    Ok(arr)
}

fn decode_bson<R: Read + ?Sized>(reader: &mut R, tag: u8) -> DecoderResult<Bson> {
    use spec::ElementType::*;
    match spec::ElementType::from(tag) {
        Some(FloatingPoint) => {
            Ok(Bson::FloatingPoint(try!(reader.read_f64::<LittleEndian>())))
        },
        Some(Utf8String) => read_string(reader).map(Bson::String),
        Some(EmbeddedDocument) => decode_document(reader).map(Bson::Document),
        Some(Array) => decode_array(reader).map(Bson::Array),
        Some(Binary) => {
            let len = try!(read_i32(reader));
            let subtype = BinarySubtype::from(try!(reader.read_u8()));
            let mut data = Vec::with_capacity(len as usize);
            try!(reader.take(len as u64).read_to_end(&mut data));
            Ok(Bson::Binary(subtype, data))
        }
        Some(ObjectId) => {
            let mut objid = [0; 12];
            for x in &mut objid {
                *x = try!(reader.read_u8());
            }
            Ok(Bson::ObjectId(oid::ObjectId::with_bytes(objid)))
        }
        Some(Boolean) => Ok(Bson::Boolean(try!(reader.read_u8()) != 0)),
        Some(NullValue) => Ok(Bson::Null),
        Some(RegularExpression) => {
            let pat = try!(read_cstring(reader));
            let opt = try!(read_cstring(reader));
            Ok(Bson::RegExp(pat, opt))
        },
        Some(JavaScriptCode) => read_string(reader).map(Bson::JavaScriptCode),
        Some(JavaScriptCodeWithScope) => {
            let code = try!(read_string(reader));
            let scope = try!(decode_document(reader));
            Ok(Bson::JavaScriptCodeWithScope(code, scope))
        },
        Some(Integer32Bit) => read_i32(reader).map(Bson::I32),
        Some(Integer64Bit) => read_i64(reader).map(Bson::I64),
        Some(TimeStamp) => read_i64(reader).map(Bson::TimeStamp),
        Some(UtcDatetime) => {
            let time = try!(read_i64(reader));
            Ok(Bson::UtcDatetime(DateTime::from_utc(NaiveDateTime::from_timestamp(time / 1000, 0), UTC)))
        },
	Some(Deprecated) |
        Some(Undefined) |
        Some(DbPointer) |
        Some(MaxKey) |
        Some(MinKey) |
        None => Err(DecoderError::UnrecognizedElementType(tag))
    }
}
