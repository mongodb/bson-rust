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

use std::io::{self, Write};
use std::iter::IntoIterator;
use std::{mem, error, fmt};

use byteorder::{self, LittleEndian, WriteBytesExt};

use bson::Bson;

/// Possible errors that can arise during encoding.
#[derive(Debug)]
pub enum EncoderError {
    IoError(io::Error),
}

impl From<io::Error> for EncoderError {
    fn from(err: io::Error) -> EncoderError {
        EncoderError::IoError(err)
    }
}

impl From<byteorder::Error> for EncoderError {
    fn from(err: byteorder::Error) -> EncoderError {
        EncoderError::IoError(From::from(err))
    }
}

impl fmt::Display for EncoderError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &EncoderError::IoError(ref inner) => inner.fmt(fmt)
        }
    }
}

impl error::Error for EncoderError {
    fn description(&self) -> &str {
        match self {
            &EncoderError::IoError(ref inner) => inner.description(),
        }
    }
    fn cause(&self) -> Option<&error::Error> {
        match self {
            &EncoderError::IoError(ref inner) => Some(inner)
        }
    }
}

/// Alias for `Result<T, EncoderError>`.
pub type EncoderResult<T> = Result<T, EncoderError>;

fn write_string<W: Write + ?Sized>(writer: &mut W, s: &str) -> EncoderResult<()> {
    try!(writer.write_i32::<LittleEndian>(s.len() as i32 + 1));
    try!(writer.write_all(s.as_bytes()));
    try!(writer.write_u8(0));
    Ok(())
}

fn write_cstring<W: Write + ?Sized>(writer: &mut W, s: &str) -> EncoderResult<()> {
    try!(writer.write_all(s.as_bytes()));
    try!(writer.write_u8(0));
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
        try!(encode_bson(&mut buf, &key.to_string(), val));
    }

    try!(write_i32(writer, (buf.len() + mem::size_of::<i32>() + mem::size_of::<u8>()) as i32));
    try!(writer.write_all(&buf));
    try!(writer.write_u8(0));
    Ok(())
}

/// Attempt to encode a `Document` into a byte stream.
///
/// Can encode any type which is iterable as `(key: &str, value: &Bson)` pairs,
/// which generally means most maps.
pub fn encode_document
    <'a, S: AsRef<str> + 'a, W: Write + ?Sized, D: IntoIterator<Item=(&'a S, &'a Bson)>>
    (writer: &mut W, doc: D) -> EncoderResult<()>
{
    let mut buf = Vec::new();
    for (key, val) in doc.into_iter() {
        try!(encode_bson(&mut buf, key.as_ref(), val));
    }

    try!(write_i32(writer, (buf.len() + mem::size_of::<i32>() + mem::size_of::<u8>()) as i32));
    try!(writer.write_all(&buf));
    try!(writer.write_u8(0));
    Ok(())
}

fn encode_bson<W: Write + ?Sized>(writer: &mut W, key: &str, val: &Bson) -> EncoderResult<()> {
    try!(writer.write_u8(val.element_type() as u8));
    try!(write_cstring(writer, key));

    match val {
        &Bson::FloatingPoint(v) => write_f64(writer, v),
        &Bson::String(ref v) => write_string(writer, &v),
        &Bson::Array(ref v) => encode_array(writer, &v),
        &Bson::Document(ref v) => encode_document(writer, v),
        &Bson::Boolean(v) => writer.write_u8(if v { 0x01 } else { 0x00 }).map_err(From::from),
        &Bson::RegExp(ref pat, ref opt) => {
            try!(write_cstring(writer, pat));
            write_cstring(writer, opt)
        },
        &Bson::JavaScriptCode(ref code) => write_string(writer, &code),
        &Bson::ObjectId(ref id) => writer.write_all(&id.bytes()).map_err(From::from),
        &Bson::JavaScriptCodeWithScope(ref code, ref scope) => {
            let mut buf = Vec::new();
            try!(write_string(&mut buf, code));
            try!(encode_document(&mut buf, scope));

            try!(write_i32(writer, buf.len() as i32 + 1));
            writer.write_all(&buf).map_err(From::from)
        },
        &Bson::I32(v) => write_i32(writer, v),
        &Bson::I64(v) => write_i64(writer, v),
        &Bson::TimeStamp(v) => write_i64(writer, v),
        &Bson::Binary(subtype, ref data) => {
            try!(write_i32(writer, data.len() as i32));
            try!(writer.write_u8(From::from(subtype)));
            writer.write_all(data).map_err(From::from)
        },
        &Bson::UtcDatetime(ref v) => write_i64(writer, v.timestamp()),
        &Bson::Null => Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::encode_document;
    use bson::{Document, Bson};

    #[test]
    fn test_encode_floating_point() {
        let src = 1020.123;
        let dst = [18, 0, 0, 0, 1, 107, 101, 121, 0, 68, 139, 108, 231, 251, 224, 143, 64, 0];

        let mut doc = Document::new();
        doc.insert("key".to_owned(), Bson::FloatingPoint(src));

        let mut buf = Vec::new();
        encode_document(&mut buf, &doc).unwrap();

        assert_eq!(&buf, &dst);
    }

    #[test]
    fn test_encode_utf8_string() {
        let src = "test你好吗".to_owned();
        let dst = [28, 0, 0, 0, 2, 107, 101, 121, 0, 14, 0, 0, 0, 116, 101, 115, 116, 228, 189, 160, 229, 165, 189, 229, 144, 151, 0, 0];

        let mut doc = Document::new();
        doc.insert("key".to_owned(), Bson::String(src));

        let mut buf = Vec::new();
        encode_document(&mut buf, &doc).unwrap();

        assert_eq!(&buf, &dst);
    }

    #[test]
    fn test_encode_array() {
        let src = vec![Bson::FloatingPoint(1.01), Bson::String("xyz".to_owned())];
        let dst = [37, 0, 0, 0, 4, 107, 101, 121, 0, 27, 0, 0, 0, 1, 48, 0, 41, 92, 143, 194, 245, 40, 240, 63, 2, 49, 0, 4, 0, 0, 0, 120, 121, 122, 0, 0, 0];

        let mut doc = Document::new();
        doc.insert("key".to_owned(), Bson::Array(src));

        let mut buf = Vec::new();
        encode_document(&mut buf, &doc).unwrap();

        assert_eq!(&buf[..], &dst[..]);
    }
}
