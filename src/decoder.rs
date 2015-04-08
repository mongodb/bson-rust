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

use std::io::{self, Read};
use std::str;
use std::convert::From;

use byteorder::{self, LittleEndian, ReadBytesExt};
use chrono::{DateTime, NaiveDateTime, UTC};

use spec::{self, BinarySubtype};
use bson;

#[derive(Debug)]
pub enum DecoderError {
    IoError(io::Error),
    Utf8Error(str::Utf8Error),
    UnrecognizedElementType(u8),
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

pub type DecoderResult<T> = Result<T, DecoderError>;

pub struct Decoder<'a> {
    reader: &'a mut Read,
}

impl<'a> Decoder<'a> {
    pub fn new(r: &'a mut Read) -> Decoder<'a> {
        Decoder {
            reader: r,
        }
    }

    fn read_string(&mut self) -> Result<String, DecoderError> {
        let len = try!(self.reader.read_i32::<LittleEndian>());

        let mut s = String::new();
        try!(self.reader.take(len as u64 - 1).read_to_string(&mut s));
        try!(self.reader.read_u8()); // The last 0x00

        Ok(s)
    }

    fn read_cstring(&mut self) -> Result<String, DecoderError> {
        let mut v = Vec::new();

        loop {
            let c = try!(self.reader.read_u8());
            if c == 0 { break; }
            v.push(c);
        }

        Ok(try!(str::from_utf8(&v[..])).to_string())
    }

    pub fn decode_floating_point(&mut self) -> Result<f64, DecoderError> {
        let f = try!(self.reader.read_f64::<LittleEndian>());
        Ok(f)
    }

    pub fn decode_utf8_string(&mut self) -> Result<String, DecoderError> {
        self.read_string()
    }

    pub fn decode_binary_data(&mut self) -> Result<(BinarySubtype, Vec<u8>), DecoderError> {
        let len = try!(self.reader.read_i32::<LittleEndian>());
        let t: BinarySubtype = From::from(try!(self.reader.read_u8()));
        let mut data = Vec::new();
        try!(self.reader.take(len as u64).read_to_end(&mut data));

        Ok((t, data))
    }

    pub fn decode_objectid(&mut self) -> Result<[u8; 12], DecoderError> {
        let mut objid = [0u8; 12];

        for x in objid.iter_mut() {
            *x = try!(self.reader.read_u8());
        }

        Ok(objid)
    }

    pub fn decode_boolean(&mut self) -> Result<bool, DecoderError> {
        let x = try!(self.reader.read_u8());

        if x == 0x00 {
            Ok(false)
        } else {
            Ok(true)
        }
    }

    pub fn decode_regexp(&mut self) -> Result<(String, String), DecoderError> {
        let pat = try!(self.read_cstring());
        let opt = try!(self.read_cstring());

        Ok((pat, opt))
    }

    pub fn decode_javascript_code(&mut self) -> Result<String, DecoderError> {
        let code = try!(self.read_string());

        Ok(code)
    }

    pub fn decode_javascript_code_with_scope(&mut self) -> Result<(String, bson::Document), DecoderError> {
        let code = try!(self.read_string());
        let doc = try!(self.decode_document());

        Ok((code, doc))
    }

    pub fn decode_integer_32bit(&mut self) -> Result<i32, DecoderError> {
        let x = try!(self.reader.read_i32::<LittleEndian>());

        Ok(x)
    }

    pub fn decode_integer_64bit(&mut self) -> Result<i64, DecoderError> {
        let x = try!(self.reader.read_i64::<LittleEndian>());

        Ok(x)
    }

    pub fn decode_timestamp(&mut self) -> Result<i64, DecoderError> {
        let x = try!(self.reader.read_i64::<LittleEndian>());

        Ok(x)
    }

    pub fn decode_utc_datetime(&mut self) -> Result<DateTime<UTC>, DecoderError> {
        let x = try!(self.reader.read_i64::<LittleEndian>());

        let d = DateTime::from_utc(NaiveDateTime::from_timestamp(x, 0), UTC);

        Ok(d)
    }

    pub fn decode_document(&mut self) -> Result<bson::Document, DecoderError> {
        let mut doc = bson::Document::new();

        try!(self.reader.read_i32::<LittleEndian>()); // Total length, we don't need it

        loop {
            let t = try!(self.reader.read_u8());

            if t == 0 {
                break;
            }

            let (k, v) = try!(self.decode_bson(t));

            doc.insert(k, v);
        }

        Ok(doc)
    }

    pub fn decode_array(&mut self) -> Result<bson::Array, DecoderError> {
        let mut arr = bson::Array::new();

        try!(self.reader.read_i32::<LittleEndian>()); // Total length, we don't need it

        loop {
            let t = try!(self.reader.read_u8());
            if t == 0 {
                break;
            }
            // TODO: Ignore the key or not?
            let (_, v) = try!(self.decode_bson(t));

            arr.push(v)
        }

        Ok(arr)
    }

    fn decode_bson(&mut self, t: u8) -> Result<(String, bson::Bson), DecoderError> {
        let res = match t {
            spec::ELEMENT_TYPE_FLOATING_POINT => {
                let key = try!(self.read_cstring());
                let val = try!(self.decode_floating_point());

                (key, bson::Bson::FloatingPoint(val))
            },
            spec::ELEMENT_TYPE_UTF8_STRING => {
                let key = try!(self.read_cstring());
                let val = try!(self.decode_utf8_string());

                (key, bson::Bson::String(val))
            },
            spec::ELEMENT_TYPE_EMBEDDED_DOCUMENT => {
                let key = try!(self.read_cstring());
                let val = try!(self.decode_document());

                (key, bson::Bson::Document(val))
            },
            spec::ELEMENT_TYPE_ARRAY => {
                let key = try!(self.read_cstring());
                let val = try!(self.decode_array());

                (key, bson::Bson::Array(val))
            },
            spec::ELEMENT_TYPE_BINARY => {
                let key = try!(self.read_cstring());
                let (t, dat) = try!(self.decode_binary_data());

                (key, bson::Bson::Binary(t, dat))
            },
            spec::ELEMENT_TYPE_OBJECT_ID => {
                let key = try!(self.read_cstring());
                let val = try!(self.decode_objectid());

                (key, bson::Bson::ObjectId(val))
            },
            spec::ELEMENT_TYPE_BOOLEAN => {
                let key = try!(self.read_cstring());
                let val = try!(self.decode_boolean());

                (key, bson::Bson::Boolean(val))
            },
            spec::ELEMENT_TYPE_NULL_VALUE => {
                let key = try!(self.read_cstring());

                (key, bson::Bson::Null)
            },
            spec::ELEMENT_TYPE_REGULAR_EXPRESSION => {
                let key = try!(self.read_cstring());
                let (pat, opt) = try!(self.decode_regexp());

                (key, bson::Bson::RegExp(pat, opt))
            },
            spec::ELEMENT_TYPE_JAVASCRIPT_CODE => {
                let key = try!(self.read_cstring());
                let code = try!(self.decode_javascript_code());

                (key, bson::Bson::JavaScriptCode(code))
            },
            spec::ELEMENT_TYPE_JAVASCRIPT_CODE_WITH_SCOPE => {
                let key = try!(self.read_cstring());
                let (code, scope) = try!(self.decode_javascript_code_with_scope());

                (key, bson::Bson::JavaScriptCodeWithScope(code, scope))
            },
            spec::ELEMENT_TYPE_DEPRECATED => {
                let key = try!(self.read_cstring());

                (key, bson::Bson::Deprecated)
            },
            spec::ELEMENT_TYPE_32BIT_INTEGER => {
                let key = try!(self.read_cstring());
                let val = try!(self.decode_integer_32bit());

                (key, bson::Bson::I32(val))
            },
            spec::ELEMENT_TYPE_64BIT_INTEGER => {
                let key = try!(self.read_cstring());
                let val = try!(self.decode_integer_64bit());

                (key, bson::Bson::I64(val))
            },
            spec::ELEMENT_TYPE_TIMESTAMP => {
                let key = try!(self.read_cstring());
                let val = try!(self.decode_timestamp());

                (key, bson::Bson::TimeStamp(val))
            },
            spec::ELEMENT_TYPE_UTC_DATETIME => {
                let key = try!(self.read_cstring());
                let val = try!(self.decode_utc_datetime());

                (key, bson::Bson::UtcDatetime(val))
            },
            _ => return Err(DecoderError::UnrecognizedElementType(t)),
        };

        Ok(res)
    }
}
