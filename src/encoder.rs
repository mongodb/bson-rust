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

pub struct Encoder<'a> {
    writer: &'a mut Write
}

impl<'a> Encoder<'a> {
    pub fn new(writer: &'a mut Write) -> Encoder<'a> {
        Encoder {
            writer: writer
        }
    }

    fn write_string(&mut self, s: &str) -> Result<(), EncoderError> {
        try!(self.writer.write_i32::<LittleEndian>(s.len() as i32 + 1));
        try!(self.writer.write_all(s.as_bytes()));
        try!(self.writer.write_u8(0));

        Ok(())
    }

    fn write_cstring(&mut self, s: &str) -> Result<(), EncoderError> {
        try!(self.writer.write_all(s.as_bytes()));
        try!(self.writer.write_u8(0));

        Ok(())
    }

    pub fn encode_floating_point(&mut self, key: &str, val: f64) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::FloatingPoint as u8));
        try!(self.write_cstring(key));

        try!(self.writer.write_f64::<LittleEndian>(val));

        Ok(())
    }

    pub fn encode_utf8_string(&mut self, key: &str, val: &str) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::Utf8String as u8));
        try!(self.write_cstring(key));

        self.write_string(val)
    }

    pub fn encode_binary_data(&mut self, key: &str, t: BinarySubtype, data: &[u8]) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::Binary as u8));
        try!(self.write_cstring(key));

        try!(self.writer.write_i32::<LittleEndian>(data.len() as i32));
        try!(self.writer.write_u8(From::from(t)));
        try!(self.writer.write_all(data));

        Ok(())
    }

    pub fn encode_undefined(&mut self, key: &str) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::Undefined as u8));
        try!(self.write_cstring(key));

        Ok(())
    }

    pub fn encode_objectid(&mut self, key: &str, val: &[u8]) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::ObjectId as u8));
        try!(self.write_cstring(key));

        try!(self.writer.write_all(val));
        Ok(())
    }

    pub fn encode_boolean(&mut self, key: &str, val: bool) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::Boolean as u8));
        try!(self.write_cstring(key));

        try!(self.writer.write_u8(if val { 0x00 } else { 0x01 }));
        Ok(())
    }

    pub fn encode_null(&mut self, key: &str) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::NullValue as u8));
        try!(self.write_cstring(key));

        Ok(())
    }

    pub fn encode_regexp(&mut self, key: &str, pat: &str, opt: &str) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::RegularExpression as u8));
        try!(self.write_cstring(key));

        try!(self.write_cstring(pat));
        try!(self.write_cstring(opt));

        Ok(())
    }

    pub fn encode_javascript_code(&mut self, key: &str, code: &str) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::JavaScriptCode as u8));
        try!(self.write_cstring(key));

        try!(self.write_string(code));

        Ok(())
    }

    pub fn encode_deprecated(&mut self, key: &str) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::Deprecated as u8));
        try!(self.write_cstring(key));

        Ok(())
    }

    pub fn encode_javascript_code_with_scope(&mut self, key: &str, code: &str, scope: &Document)
            -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::JavaScriptCodeWithScope as u8));
        try!(self.write_cstring(key));

        let mut buf = Vec::new();
        {
            let mut enc = Encoder::new(&mut buf);
            try!(enc.write_string(code));
            try!(enc.encode_document(scope));
        }

        try!(self.writer.write_i32::<LittleEndian>(buf.len() as i32 + 1));
        try!(self.writer.write_all(&buf[..]));

        Ok(())
    }

    pub fn encode_integer_32bit(&mut self, key: &str, val: i32) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::Integer32Bit as u8));
        try!(self.write_cstring(key));

        try!(self.writer.write_i32::<LittleEndian>(val));

        Ok(())
    }

    pub fn encode_integer_64bit(&mut self, key: &str, val: i64) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::Integer64Bit as u8));
        try!(self.write_cstring(key));

        try!(self.writer.write_i64::<LittleEndian>(val));

        Ok(())
    }

    pub fn encode_timestamp(&mut self, key: &str, val: i64) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::TimeStamp as u8));
        try!(self.write_cstring(key));

        try!(self.writer.write_i64::<LittleEndian>(val));

        Ok(())
    }

    pub fn encode_utc_datetime(&mut self, key: &str, val: &DateTime<UTC>) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::UtcDatetime as u8));
        try!(self.write_cstring(key));

        try!(self.writer.write_i64::<LittleEndian>(val.timestamp()));

        Ok(())
    }

    pub fn encode_embedded_document(&mut self, key: &str, doc: &Document) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::EmbeddedDocument as u8));
        try!(self.write_cstring(key));

        self.encode_document(doc)
    }

    pub fn encode_embedded_array(&mut self, key: &str, arr: &Array) -> Result<(), EncoderError> {
        try!(self.writer.write_u8(ElementType::Array as u8));
        try!(self.write_cstring(key));

        self.encode_array(arr)
    }

    pub fn encode_document(&mut self, doc: &Document) -> Result<(), EncoderError> {
        let mut buf = Vec::new();

        {
            let mut enc = Encoder::new(&mut buf);
            for (key, val) in doc.iter() {
                try!(enc.encode_bson(key, val));
            }
        }

        try!(self.writer.write_i32::<LittleEndian>((buf.len() + mem::size_of::<i32>() + mem::size_of::<u8>()) as i32));
        try!(self.writer.write_all(&buf[..]));
        try!(self.writer.write_u8(0));

        Ok(())
    }

    pub fn encode_array(&mut self, arr: &Array) -> Result<(), EncoderError> {
        let mut buf = Vec::new();

        {
            let mut enc = Encoder::new(&mut buf);
            for (key, val) in arr.iter().enumerate() {
                try!(enc.encode_bson(&key.to_string(), val));
            }
        }

        try!(self.writer.write_i32::<LittleEndian>((buf.len() + mem::size_of::<i32>() + mem::size_of::<u8>()) as i32));
        try!(self.writer.write_all(&buf[..]));
        try!(self.writer.write_u8(0));

        Ok(())
    }

    fn encode_bson(&mut self, key: &str, val: &Bson) -> Result<(), EncoderError> {
        match val {
            &Bson::FloatingPoint(v)                     => self.encode_floating_point(&key[..], v),
            &Bson::String(ref v)                        => self.encode_utf8_string(&key[..], &v[..]),
            &Bson::Array(ref v)                         => self.encode_embedded_array(&key[..], &v),
            &Bson::Document(ref v)                      => self.encode_embedded_document(&key[..], &v),
            &Bson::Boolean(v)                           => self.encode_boolean(&key[..], v),
            &Bson::Null                                 => self.encode_null(&key[..]),
            &Bson::RegExp(ref pat, ref opt)             => self.encode_regexp(&key[..], &pat[..], &opt[..]),
            &Bson::JavaScriptCode(ref code)             => self.encode_javascript_code(&key[..], &code[..]),
            &Bson::ObjectId(id)                         => self.encode_objectid(&key[..], &id[..]),
            &Bson::Deprecated                           => self.encode_deprecated(&key[..]),
            &Bson::JavaScriptCodeWithScope(ref code, ref scope)
                => self.encode_javascript_code_with_scope(&key[..], &code[..], &scope),
            &Bson::I32(v)                               => self.encode_integer_32bit(&key[..], v),
            &Bson::I64(v)                               => self.encode_integer_64bit(&key[..], v),
            &Bson::TimeStamp(v)                         => self.encode_timestamp(&key[..], v),
            &Bson::Binary(t, ref v)                     => self.encode_binary_data(&key[..], t, &v[..]),
            &Bson::UtcDatetime(ref v)                   => self.encode_utc_datetime(&key[..], v),
        }
    }
}

#[cfg(test)]
mod test {
    use super::Encoder;
    use bson::{Document, Bson};

    #[test]
    fn test_encode_floating_point() {
        let src = 1020.123;
        let dst = [18, 0, 0, 0, 1, 107, 101, 121, 0, 68, 139, 108, 231, 251, 224, 143, 64, 0];

        let mut buf = Vec::new();
        {
            let mut enc = Encoder::new(&mut buf);

            let mut doc = Document::new();
            doc.insert("key".to_owned(), Bson::FloatingPoint(src));
            enc.encode_document(&doc).unwrap();
        }

        assert_eq!(&buf[..], dst);
    }

    #[test]
    fn test_encode_utf8_string() {
        let src = "test你好吗".to_owned();
        let dst = [28, 0, 0, 0, 2, 107, 101, 121, 0, 14, 0, 0, 0, 116, 101, 115, 116, 228, 189, 160, 229, 165, 189, 229, 144, 151, 0, 0];

        let mut buf = Vec::new();
        {
            let mut enc = Encoder::new(&mut buf);

            let mut doc = Document::new();
            doc.insert("key".to_owned(), Bson::String(src));
            enc.encode_document(&doc).unwrap();
        }

        assert_eq!(&buf, &dst);
    }

    #[test]
    fn test_encode_array() {
        let src = vec![Bson::FloatingPoint(1.01), Bson::String("xyz".to_owned())];
        let dst = [37, 0, 0, 0, 4, 107, 101, 121, 0, 27, 0, 0, 0, 1, 48, 0, 41, 92, 143, 194, 245, 40, 240, 63, 2, 49, 0, 4, 0, 0, 0, 120, 121, 122, 0, 0, 0];

        let mut buf = Vec::new();
        {
            let mut enc = Encoder::new(&mut buf);

            let mut doc = Document::new();
            doc.insert("key".to_owned(), Bson::Array(src));
            enc.encode_document(&doc).unwrap();
        }

        assert_eq!(&buf[..], &dst[..]);
    }
}
