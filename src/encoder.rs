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
use std::{mem, error, fmt};

use byteorder::{self, LittleEndian, WriteBytesExt};
use chrono::{DateTime, UTC};

use spec::{ElementType, BinarySubtype};
use bson::{Array, Document, Bson};

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

fn encode_floating_point<W: Write + ?Sized>(writer: &mut W, key: &str, val: f64) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::FloatingPoint as u8));
	try!(write_cstring(writer, key));

	try!(writer.write_f64::<LittleEndian>(val));

	Ok(())
}

fn encode_utf8_string<W: Write + ?Sized>(writer: &mut W, key: &str, val: &str) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::Utf8String as u8));
	try!(write_cstring(writer, key));

	write_string(writer, val)
}

fn encode_binary_data<W: Write + ?Sized>(writer: &mut W, key: &str, t: BinarySubtype, data: &[u8]) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::Binary as u8));
	try!(write_cstring(writer, key));

	try!(writer.write_i32::<LittleEndian>(data.len() as i32));
	try!(writer.write_u8(From::from(t)));
	try!(writer.write_all(data));

	Ok(())
}

fn encode_objectid<W: Write + ?Sized>(writer: &mut W, key: &str, val: &[u8]) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::ObjectId as u8));
	try!(write_cstring(writer, key));

	try!(writer.write_all(val));
	Ok(())
}

fn encode_boolean<W: Write + ?Sized>(writer: &mut W, key: &str, val: bool) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::Boolean as u8));
	try!(write_cstring(writer, key));

	try!(writer.write_u8(if val { 0x00 } else { 0x01 }));
	Ok(())
}

fn encode_null<W: Write + ?Sized>(writer: &mut W, key: &str) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::NullValue as u8));
	try!(write_cstring(writer, key));

	Ok(())
}

fn encode_regexp<W: Write + ?Sized>(writer: &mut W, key: &str, pat: &str, opt: &str) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::RegularExpression as u8));
	try!(write_cstring(writer, key));

	try!(write_cstring(writer, pat));
	try!(write_cstring(writer, opt));

	Ok(())
}

fn encode_javascript_code<W: Write + ?Sized>(writer: &mut W, key: &str, code: &str) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::JavaScriptCode as u8));
	try!(write_cstring(writer, key));

	try!(write_string(writer, code));

	Ok(())
}

fn encode_deprecated<W: Write + ?Sized>(writer: &mut W, key: &str) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::Deprecated as u8));
	try!(write_cstring(writer, key));

	Ok(())
}

fn encode_javascript_code_with_scope<W: Write + ?Sized>(writer: &mut W, key: &str, code: &str, scope: &Document) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::JavaScriptCodeWithScope as u8));
	try!(write_cstring(writer, key));

	let mut buf = Vec::new();
	try!(write_string(&mut buf, code));
	try!(encode_document(&mut buf, scope));

	try!(writer.write_i32::<LittleEndian>(buf.len() as i32 + 1));
	try!(writer.write_all(&buf[..]));

	Ok(())
}

fn encode_integer_32bit<W: Write + ?Sized>(writer: &mut W, key: &str, val: i32) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::Integer32Bit as u8));
	try!(write_cstring(writer, key));

	try!(writer.write_i32::<LittleEndian>(val));

	Ok(())
}

fn encode_integer_64bit<W: Write + ?Sized>(writer: &mut W, key: &str, val: i64) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::Integer64Bit as u8));
	try!(write_cstring(writer, key));

	try!(writer.write_i64::<LittleEndian>(val));

	Ok(())
}

fn encode_timestamp<W: Write + ?Sized>(writer: &mut W, key: &str, val: i64) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::TimeStamp as u8));
	try!(write_cstring(writer, key));

	try!(writer.write_i64::<LittleEndian>(val));

	Ok(())
}

fn encode_utc_datetime<W: Write + ?Sized>(writer: &mut W, key: &str, val: &DateTime<UTC>) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::UtcDatetime as u8));
	try!(write_cstring(writer, key));

	try!(writer.write_i64::<LittleEndian>(val.timestamp()));

	Ok(())
}

fn encode_embedded_document<W: Write + ?Sized>(writer: &mut W, key: &str, doc: &Document) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::EmbeddedDocument as u8));
	try!(write_cstring(writer, key));

	encode_document(writer, doc)
}

fn encode_embedded_array<W: Write + ?Sized>(writer: &mut W, key: &str, arr: &Array) -> EncoderResult<()> {
	try!(writer.write_u8(ElementType::Array as u8));
	try!(write_cstring(writer, key));

	let mut buf = Vec::new();
	for (key, val) in arr.iter().enumerate() {
		try!(encode_bson(&mut buf, &key.to_string(), val));
	}

	try!(writer.write_i32::<LittleEndian>((buf.len() + mem::size_of::<i32>() + mem::size_of::<u8>()) as i32));
	try!(writer.write_all(&buf[..]));
	try!(writer.write_u8(0));

	Ok(())
}

pub fn encode_document<W: Write + ?Sized>(writer: &mut W, doc: &Document) -> EncoderResult<()> {
	let mut buf = Vec::new();
	for (key, val) in doc.iter() {
		try!(encode_bson(&mut buf, key, val));
	}

	try!(writer.write_i32::<LittleEndian>((buf.len() + mem::size_of::<i32>() + mem::size_of::<u8>()) as i32));
	try!(writer.write_all(&buf[..]));
	try!(writer.write_u8(0));

	Ok(())
}

fn encode_bson<W: Write + ?Sized>(writer: &mut W, key: &str, val: &Bson) -> EncoderResult<()> {
	match val {
		&Bson::FloatingPoint(v)                     => encode_floating_point(writer, &key, v),
		&Bson::String(ref v)                        => encode_utf8_string(writer, &key, &v[..]),
		&Bson::Array(ref v)                         => encode_embedded_array(writer, &key, &v),
		&Bson::Document(ref v)                      => encode_embedded_document(writer, &key, &v),
		&Bson::Boolean(v)                           => encode_boolean(writer, &key, v),
		&Bson::Null                                 => encode_null(writer, &key),
		&Bson::RegExp(ref pat, ref opt)             => encode_regexp(writer, &key, &pat[..], &opt[..]),
		&Bson::JavaScriptCode(ref code)             => encode_javascript_code(writer, &key, &code[..]),
		&Bson::ObjectId(id)                         => encode_objectid(writer, &key, &id[..]),
		&Bson::Deprecated                           => encode_deprecated(writer, &key),
		&Bson::JavaScriptCodeWithScope(ref code, ref scope)
			=> encode_javascript_code_with_scope(writer, &key, &code[..], &scope),
		&Bson::I32(v)                               => encode_integer_32bit(writer, &key, v),
		&Bson::I64(v)                               => encode_integer_64bit(writer, &key, v),
		&Bson::TimeStamp(v)                         => encode_timestamp(writer, &key, v),
		&Bson::Binary(t, ref v)                     => encode_binary_data(writer, &key, t, &v[..]),
		&Bson::UtcDatetime(ref v)                   => encode_utc_datetime(writer, &key, v),
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
