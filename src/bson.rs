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

//! BSON definition

use std::fmt::{self, Display, Debug};

use chrono::{DateTime, Timelike, UTC};
use chrono::offset::TimeZone;
use rustc_serialize::json;
use rustc_serialize::hex::{FromHex, ToHex};

use oid;
use ordered::OrderedDocument;
use decimal128::Decimal128;
use spec::{ElementType, BinarySubtype};

/// Possible BSON value types.
#[derive(Clone, PartialEq)]
pub enum Bson {
    /// 64-bit binary floating point
    FloatingPoint(f64),
    /// UTF-8 string
    String(String),
    /// Array
    Array(Array),
    /// Embedded document
    Document(Document),
    /// Boolean value
    Boolean(bool),
    /// Null value
    Null,
    /// Regular expression - The first cstring is the regex pattern, the second is the regex options string.
    /// Options are identified by characters, which must be stored in alphabetical order.
    /// Valid options are 'i' for case insensitive matching, 'm' for multiline matching, 'x' for verbose mode,
    /// 'l' to make \w, \W, etc. locale dependent, 's' for dotall mode ('.' matches everything), and 'u' to
    /// make \w, \W, etc. match unicode.
    RegExp(String, String),
    /// JavaScript code
    JavaScriptCode(String),
    /// JavaScript code w/ scope
    JavaScriptCodeWithScope(String, Document),
    /// 32-bit integer
    I32(i32),
    /// 64-bit integer
    I64(i64),
    /// Timestamp
    TimeStamp(i64),
    /// Binary data
    Binary(BinarySubtype, Vec<u8>),
    /// [ObjectId](http://dochub.mongodb.org/core/objectids)
    ObjectId(oid::ObjectId),
    /// UTC datetime
    UtcDatetime(DateTime<UTC>),
    /// Symbol (Deprecated)
    Symbol(String),
    /// [128-bit decimal floating point](https://github.com/mongodb/specifications/blob/master/source/bson-decimal128/decimal128.rst)
    Decimal128(Decimal128),
}

/// Alias for `Vec<Bson>`.
pub type Array = Vec<Bson>;
/// Alias for `OrderedDocument`.
pub type Document = OrderedDocument;

impl Debug for Bson {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Bson::FloatingPoint(p) => write!(f, "FloatingPoint({:?})", p),
            &Bson::String(ref s) => write!(f, "String({:?})", s),
            &Bson::Array(ref vec) => write!(f, "Array({:?})", vec),
            &Bson::Document(ref doc) => write!(f, "Document({})", doc),
            &Bson::Boolean(b) => write!(f, "Boolean({:?})", b),
            &Bson::Null => write!(f, "Null"),
            &Bson::RegExp(ref pat, ref opt) => write!(f, "RegExp(/{:?}/{:?})", pat, opt),
            &Bson::JavaScriptCode(ref s) => write!(f, "JavaScriptCode({:?})", s),
            &Bson::JavaScriptCodeWithScope(ref s, ref scope) => {
                write!(f, "JavaScriptCodeWithScope({:?}, {:?})", s, scope)
            }
            &Bson::I32(v) => write!(f, "I32({:?})", v),
            &Bson::I64(v) => write!(f, "I64({:?})", v),
            &Bson::TimeStamp(i) => {
                let time = (i >> 32) as i32;
                let inc = (i & 0xFFFFFFFF) as i32;

                write!(f, "TimeStamp({}, {})", time, inc)
            }
            &Bson::Binary(t, ref vec) => write!(f, "BinData({}, 0x{})", u8::from(t), vec.to_hex()),
            &Bson::ObjectId(ref id) => write!(f, "ObjectId({:?})", id),
            &Bson::UtcDatetime(date_time) => write!(f, "UtcDatetime({:?})", date_time),
            &Bson::Symbol(ref sym) => write!(f, "Symbol({:?})", sym),
            &Bson::Decimal128(ref d) => write!(f, "Decimal128({:?})", d),
        }
    }
}

impl Display for Bson {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &Bson::FloatingPoint(f) => write!(fmt, "{}", f),
            &Bson::String(ref s) => write!(fmt, "\"{}\"", s),
            &Bson::Array(ref vec) => {
                try!(write!(fmt, "["));

                let mut first = true;
                for bson in vec.iter() {
                    if !first {
                        try!(write!(fmt, ", "));
                    }

                    try!(write!(fmt, "{}", bson));
                    first = false;
                }

                write!(fmt, "]")
            }
            &Bson::Document(ref doc) => write!(fmt, "{}", doc),
            &Bson::Boolean(b) => write!(fmt, "{}", b),
            &Bson::Null => write!(fmt, "null"),
            &Bson::RegExp(ref pat, ref opt) => write!(fmt, "/{}/{}", pat, opt),
            &Bson::JavaScriptCode(ref s) |
            &Bson::JavaScriptCodeWithScope(ref s, _) => fmt.write_str(&s),
            &Bson::I32(i) => write!(fmt, "{}", i),
            &Bson::I64(i) => write!(fmt, "{}", i),
            &Bson::TimeStamp(i) => {
                let time = (i >> 32) as i32;
                let inc = (i & 0xFFFFFFFF) as i32;

                write!(fmt, "Timestamp({}, {})", time, inc)
            }
            &Bson::Binary(t, ref vec) => {
                write!(fmt, "BinData({}, 0x{})", u8::from(t), vec.to_hex())
            }
            &Bson::ObjectId(ref id) => write!(fmt, "ObjectId(\"{}\")", id),
            &Bson::UtcDatetime(date_time) => write!(fmt, "Date(\"{}\")", date_time),
            &Bson::Symbol(ref sym) => write!(fmt, "Symbol(\"{}\")", sym),
            &Bson::Decimal128(ref d) => write!(fmt, "{}", d),
        }
    }
}

impl From<f32> for Bson {
    fn from(a: f32) -> Bson {
        Bson::FloatingPoint(a as f64)
    }
}

impl From<f64> for Bson {
    fn from(a: f64) -> Bson {
        Bson::FloatingPoint(a)
    }
}

impl<'a> From<&'a str> for Bson {
    fn from(s: &str) -> Bson {
        Bson::String(s.to_owned())
    }
}

impl From<String> for Bson {
    fn from(a: String) -> Bson {
        Bson::String(a)
    }
}

impl<'a> From<&'a String> for Bson {
    fn from(a: &'a String) -> Bson {
        Bson::String(a.to_owned())
    }
}

impl From<Array> for Bson {
    fn from(a: Array) -> Bson {
        Bson::Array(a)
    }
}

impl From<Document> for Bson {
    fn from(a: Document) -> Bson {
        Bson::Document(a)
    }
}

impl From<bool> for Bson {
    fn from(a: bool) -> Bson {
        Bson::Boolean(a)
    }
}

impl From<(String, String)> for Bson {
    fn from(a: (String, String)) -> Bson {
        let (a1, a2) = a;
        Bson::RegExp(a1.to_owned(), a2.to_owned())
    }
}

impl From<(String, Document)> for Bson {
    fn from(a: (String, Document)) -> Bson {
        let (a1, a2) = a;
        Bson::JavaScriptCodeWithScope(a1, a2)
    }
}

impl From<(BinarySubtype, Vec<u8>)> for Bson {
    fn from(a: (BinarySubtype, Vec<u8>)) -> Bson {
        let (a1, a2) = a;
        Bson::Binary(a1, a2)
    }
}

impl From<i32> for Bson {
    fn from(a: i32) -> Bson {
        Bson::I32(a)
    }
}

impl From<i64> for Bson {
    fn from(a: i64) -> Bson {
        Bson::I64(a)
    }
}

impl From<u32> for Bson {
    fn from(a: u32) -> Bson {
        Bson::I32(a as i32)
    }
}

impl From<u64> for Bson {
    fn from(a: u64) -> Bson {
        Bson::I64(a as i64)
    }
}

impl From<[u8; 12]> for Bson {
    fn from(a: [u8; 12]) -> Bson {
        Bson::ObjectId(oid::ObjectId::with_bytes(a))
    }
}

impl From<oid::ObjectId> for Bson {
    fn from(a: oid::ObjectId) -> Bson {
        Bson::ObjectId(a.to_owned())
    }
}

impl From<DateTime<UTC>> for Bson {
    fn from(a: DateTime<UTC>) -> Bson {
        Bson::UtcDatetime(a)
    }
}

impl Bson {
    /// Get the `ElementType` of this value.
    pub fn element_type(&self) -> ElementType {
        match self {
            &Bson::FloatingPoint(..) => ElementType::FloatingPoint,
            &Bson::String(..) => ElementType::Utf8String,
            &Bson::Array(..) => ElementType::Array,
            &Bson::Document(..) => ElementType::EmbeddedDocument,
            &Bson::Boolean(..) => ElementType::Boolean,
            &Bson::Null => ElementType::NullValue,
            &Bson::RegExp(..) => ElementType::RegularExpression,
            &Bson::JavaScriptCode(..) => ElementType::JavaScriptCode,
            &Bson::JavaScriptCodeWithScope(..) => ElementType::JavaScriptCodeWithScope,
            &Bson::I32(..) => ElementType::Integer32Bit,
            &Bson::I64(..) => ElementType::Integer64Bit,
            &Bson::TimeStamp(..) => ElementType::TimeStamp,
            &Bson::Binary(..) => ElementType::Binary,
            &Bson::ObjectId(..) => ElementType::ObjectId,
            &Bson::UtcDatetime(..) => ElementType::UtcDatetime,
            &Bson::Symbol(..) => ElementType::Symbol,
            &Bson::Decimal128(..) => ElementType::Decimal128Bit,
        }
    }

    /// Convert this value to the best approximate `Json`.
    pub fn to_json(&self) -> json::Json {
        match self {
            &Bson::FloatingPoint(v) => json::Json::F64(v),
            &Bson::String(ref v) => json::Json::String(v.clone()),
            &Bson::Array(ref v) => json::Json::Array(v.iter().map(|x| x.to_json()).collect()),
            &Bson::Document(ref v) => {
                json::Json::Object(v.iter().map(|(k, v)| (k.clone(), v.to_json())).collect())
            }
            &Bson::Boolean(v) => json::Json::Boolean(v),
            &Bson::Null => json::Json::Null,
            &Bson::RegExp(ref pat, ref opt) => {
                let mut re = json::Object::new();
                re.insert("$regex".to_owned(), json::Json::String(pat.clone()));
                re.insert("$options".to_owned(), json::Json::String(opt.clone()));

                json::Json::Object(re)
            }
            &Bson::JavaScriptCode(ref code) => {
                let mut obj = json::Object::new();
                obj.insert("$code".to_owned(), json::Json::String(code.clone()));
                json::Json::Object(obj)
            }
            &Bson::JavaScriptCodeWithScope(ref code, ref scope) => {
                let mut obj = json::Object::new();
                obj.insert("$code".to_owned(), json::Json::String(code.clone()));

                let scope_obj = scope.iter().map(|(k, v)| (k.clone(), v.to_json())).collect();

                obj.insert("scope".to_owned(), json::Json::Object(scope_obj));

                json::Json::Object(obj)
            }
            &Bson::I32(v) => json::Json::I64(v as i64),
            &Bson::I64(v) => json::Json::I64(v),
            &Bson::TimeStamp(v) => {
                let time = v >> 32;
                let inc = v & 0x0000FFFF;

                let mut obj = json::Object::new();
                obj.insert("t".to_owned(), json::Json::I64(time));
                obj.insert("i".to_owned(), json::Json::I64(inc));

                json::Json::Object(obj)
            }
            &Bson::Binary(t, ref v) => {
                let mut obj = json::Object::new();
                let tval: u8 = From::from(t);
                obj.insert("type".to_owned(), json::Json::I64(tval as i64));
                obj.insert("$binary".to_owned(), json::Json::String(v.to_hex()));

                json::Json::Object(obj)
            }
            &Bson::ObjectId(ref v) => {
                let mut obj = json::Object::new();
                obj.insert("$oid".to_owned(), json::Json::String(v.to_string()));

                json::Json::Object(obj)
            }
            &Bson::UtcDatetime(ref v) => {
                let mut obj = json::Object::new();
                let mut inner = json::Object::new();
                inner.insert("$numberLong".to_owned(),
                             json::Json::I64((v.timestamp() * 1000) +
                                             (v.nanosecond() / 1000000) as i64));
                obj.insert("$date".to_owned(), json::Json::Object(inner));
                json::Json::Object(obj)
            }
            &Bson::Symbol(ref v) => {
                // FIXME: Don't know what is the best way to encode Symbol type
                let mut obj = json::Object::new();
                obj.insert("$symbol".to_owned(), json::Json::String(v.to_owned()));
                json::Json::Object(obj)
            }
            &Bson::Decimal128(ref v) => {
                let mut obj = json::Object::new();
                obj.insert("$numberDecimal".to_owned(),
                           json::Json::String(v.to_string()));
                json::Json::Object(obj)
            }
        }
    }

    /// Create a `Bson` from a `Json`.
    pub fn from_json(j: &json::Json) -> Bson {
        match j {
            &json::Json::I64(x) => Bson::I64(x),
            &json::Json::U64(x) => Bson::I64(x as i64),
            &json::Json::F64(x) => Bson::FloatingPoint(x),
            &json::Json::String(ref x) => Bson::String(x.clone()),
            &json::Json::Boolean(x) => Bson::Boolean(x),
            &json::Json::Array(ref x) => Bson::Array(x.iter().map(Bson::from_json).collect()),
            &json::Json::Object(ref x) => {
                Bson::from_extended_document(x.iter()
                    .map(|(k, v)| (k.clone(), Bson::from_json(v)))
                    .collect())
            }
            &json::Json::Null => Bson::Null,
        }
    }

    pub fn to_extended_document(&self) -> Document {
        match *self {
            Bson::RegExp(ref pat, ref opt) => {
                doc! {
                    "$regex" => (pat.clone()),
                    "$options" => (opt.clone())
                }
            }
            Bson::JavaScriptCode(ref code) => {
                doc! {
                    "$code" => (code.clone())
                }
            }
            Bson::JavaScriptCodeWithScope(ref code, ref scope) => {
                doc! {
                    "$code" => (code.clone()),
                    "$scope" => (scope.clone())
                }
            }
            Bson::TimeStamp(v) => {
                let time = (v >> 32) as i32;
                let inc = (v & 0xFFFFFFFF) as i32;

                doc! {
                    "t" => time,
                    "i" => inc
                }
            }
            Bson::Binary(t, ref v) => {
                let tval: u8 = From::from(t);
                doc! {
                    "$binary" => (v.to_hex()),
                    "type" => (tval as i64)
                }
            }
            Bson::ObjectId(ref v) => {
                doc! {
                    "$oid" => (v.to_string())
                }
            }
            Bson::UtcDatetime(ref v) => {
                doc! {
                    "$date" => {
                        "$numberLong" => ((v.timestamp() * 1000) + (v.nanosecond() / 1000000) as i64)
                    }
                }
            }
            Bson::Symbol(ref v) => {
                doc! {
                    "$symbol" => (v.to_owned())
                }
            }
            Bson::Decimal128(ref v) => {
                doc! {
                    "$numberDecimal" => (v.to_string())
                }
            }
            _ => panic!("Attempted conversion of invalid data type: {}", self),
        }
    }

    pub fn from_extended_document(values: Document) -> Bson {
        if values.len() == 2 {
            if let (Ok(pat), Ok(opt)) = (values.get_str("$regex"), values.get_str("$options")) {
                return Bson::RegExp(pat.to_owned(), opt.to_owned());

            } else if let (Ok(code), Ok(scope)) =
                (values.get_str("$code"), values.get_document("$scope")) {
                return Bson::JavaScriptCodeWithScope(code.to_owned(), scope.to_owned());

            } else if let (Ok(t), Ok(i)) = (values.get_i32("t"), values.get_i32("i")) {
                let timestamp = ((t as i64) << 32) + (i as i64);
                return Bson::TimeStamp(timestamp);

            } else if let (Ok(t), Ok(i)) = (values.get_i64("t"), values.get_i64("i")) {
                let timestamp = (t << 32) + i;
                return Bson::TimeStamp(timestamp);

            } else if let (Ok(hex), Ok(t)) = (values.get_str("$binary"), values.get_i64("type")) {
                let ttype = t as u8;
                return Bson::Binary(From::from(ttype), hex.from_hex().unwrap());
            }

        } else if values.len() == 1 {
            if let Ok(code) = values.get_str("$code") {
                return Bson::JavaScriptCode(code.to_owned());

            } else if let Ok(hex) = values.get_str("$oid") {
                return Bson::ObjectId(oid::ObjectId::with_string(hex).unwrap());

            } else if let Ok(long) = values.get_document("$date")
                .and_then(|inner| inner.get_i64("$numberLong")) {
                return Bson::UtcDatetime(UTC.timestamp(long / 1000, (long % 1000) as u32 * 1000000));
            } else if let Ok(sym) = values.get_str("$symbol") {
                return Bson::Symbol(sym.to_owned());
            } else if let Ok(dec) = values.get_str("$numberDecimal") {
                return Bson::Decimal128(dec.parse::<Decimal128>().unwrap());
            }
        }

        Bson::Document(values)
    }
}
