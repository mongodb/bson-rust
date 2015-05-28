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

        Ok(try!(str::from_utf8(&v)).to_owned())
    }

    pub fn decode_floating_point(&mut self) -> Result<f64, DecoderError> {
        self.reader.read_f64::<LittleEndian>().map_err(From::from)
    }

    pub fn decode_utf8_string(&mut self) -> Result<String, DecoderError> {
        self.read_string()
    }

    pub fn decode_binary_data(&mut self) -> Result<(BinarySubtype, Vec<u8>), DecoderError> {
        let len = try!(self.reader.read_i32::<LittleEndian>());
        let t = BinarySubtype::from(try!(self.reader.read_u8()));
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
        Ok(x != 0x00)
    }

    pub fn decode_regexp(&mut self) -> Result<(String, String), DecoderError> {
        let pat = try!(self.read_cstring());
        let opt = try!(self.read_cstring());

        Ok((pat, opt))
    }

    pub fn decode_javascript_code(&mut self) -> Result<String, DecoderError> {
        self.read_string()
    }

    pub fn decode_javascript_code_with_scope(&mut self) -> Result<(String, Document), DecoderError> {
        let code = try!(self.read_string());
        let doc = try!(self.decode_document());
        Ok((code, doc))
    }

    pub fn decode_integer_32bit(&mut self) -> Result<i32, DecoderError> {
        self.reader.read_i32::<LittleEndian>().map_err(From::from)
    }

    pub fn decode_integer_64bit(&mut self) -> Result<i64, DecoderError> {
        self.reader.read_i64::<LittleEndian>().map_err(From::from)
    }

    pub fn decode_timestamp(&mut self) -> Result<i64, DecoderError> {
        self.reader.read_i64::<LittleEndian>().map_err(From::from)
    }

    pub fn decode_utc_datetime(&mut self) -> Result<DateTime<UTC>, DecoderError> {
        let x = try!(self.reader.read_i64::<LittleEndian>());

        let d = DateTime::from_utc(NaiveDateTime::from_timestamp(x, 0), UTC);

        Ok(d)
    }

    pub fn decode_document(&mut self) -> Result<Document, DecoderError> {
        let mut doc = Document::new();

        try!(self.reader.read_i32::<LittleEndian>()); // Total length, we don't need it

        loop {
            let t = try!(self.reader.read_u8());

            if t == 0 {
                break;
            }

            let k = try!(self.read_cstring());
            let v = try!(self.decode_bson(t));

            doc.insert(k, v);
        }

        Ok(doc)
    }

    pub fn decode_array(&mut self) -> Result<Array, DecoderError> {
        let mut arr = Array::new();

        try!(self.reader.read_i32::<LittleEndian>()); // Total length, we don't need it

        loop {
            let t = try!(self.reader.read_u8());
            if t == 0 {
                break;
            }

            let k = try!(self.read_cstring());
            if k != &arr.len().to_string()[..] {
                return Err(DecoderError::InvalidArrayKey(arr.len(), k));
            }
            let v = try!(self.decode_bson(t));

            arr.push(v)
        }

        Ok(arr)
    }

    fn decode_bson(&mut self, tag: u8) -> Result<Bson, DecoderError> {
        match tag {
            spec::ELEMENT_TYPE_FLOATING_POINT => {
                self.decode_floating_point().map(Bson::FloatingPoint)
            },
            spec::ELEMENT_TYPE_UTF8_STRING => {
                self.decode_utf8_string().map(Bson::String)
            },
            spec::ELEMENT_TYPE_EMBEDDED_DOCUMENT => {
                self.decode_document().map(Bson::Document)
            },
            spec::ELEMENT_TYPE_ARRAY => {
                self.decode_array().map(Bson::Array)
            },
            spec::ELEMENT_TYPE_BINARY => {
                self.decode_binary_data().map(|(t, dat)| Bson::Binary(t, dat))
            },
            spec::ELEMENT_TYPE_OBJECT_ID => {
                self.decode_objectid().map(Bson::ObjectId)
            },
            spec::ELEMENT_TYPE_BOOLEAN => {
                self.decode_boolean().map(Bson::Boolean)
            },
            spec::ELEMENT_TYPE_NULL_VALUE => {
                Ok(Bson::Null)
            },
            spec::ELEMENT_TYPE_REGULAR_EXPRESSION => {
                self.decode_regexp().map(|(pat, opt)| Bson::RegExp(pat, opt))
            },
            spec::ELEMENT_TYPE_JAVASCRIPT_CODE => {
                self.decode_javascript_code().map(Bson::JavaScriptCode)
            },
            spec::ELEMENT_TYPE_JAVASCRIPT_CODE_WITH_SCOPE => {
                self.decode_javascript_code_with_scope().map(
                    |(code, scope)| Bson::JavaScriptCodeWithScope(code, scope)
                )
            },
            spec::ELEMENT_TYPE_DEPRECATED => {
                Ok(Bson::Deprecated)
            },
            spec::ELEMENT_TYPE_32BIT_INTEGER => {
                self.decode_integer_32bit().map(Bson::I32)
            },
            spec::ELEMENT_TYPE_64BIT_INTEGER => {
                self.decode_integer_64bit().map(Bson::I64)
            },
            spec::ELEMENT_TYPE_TIMESTAMP => {
                self.decode_timestamp().map(Bson::TimeStamp)
            },
            spec::ELEMENT_TYPE_UTC_DATETIME => {
                self.decode_utc_datetime().map(Bson::UtcDatetime)
            },
            _ => Err(DecoderError::UnrecognizedElementType(tag)),
        }
    }
}
