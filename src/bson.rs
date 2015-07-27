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

use std::fmt::{Display, Error, Formatter};
use std::str;

use chrono::{DateTime, UTC};
use rustc_serialize::json;
use rustc_serialize::hex::ToHex;

use ordered::OrderedDocument;
use spec::{ElementType, BinarySubtype};
use oid;

/// Possible BSON value types.
#[derive(Debug, Clone)]
pub enum Bson {
    FloatingPoint(f64),
    String(String),
    Array(Array),
    Document(Document),
    Boolean(bool),
    Null,
    RegExp(String, String),
    JavaScriptCode(String),
    JavaScriptCodeWithScope(String, Document),
    I32(i32),
    I64(i64),
    TimeStamp(i64),
    Binary(BinarySubtype, Vec<u8>),
    ObjectId(oid::ObjectId),
    UtcDatetime(DateTime<UTC>),
}

/// Alias for `Vec<Bson>`.
pub type Array = Vec<Bson>;
/// Alias for `OrderedDocument`.
pub type Document = OrderedDocument;

impl Display for Bson {
    fn fmt(&self, fmt: &mut Formatter) -> Result<(), Error> {
        let bson_string = match self {
            &Bson::FloatingPoint(f) => format!("{}", f),
            &Bson::String(ref s) => format!("\"{}\"", s),
            &Bson::Array(ref vec) => {
                let mut string = "[".to_owned();

                for bson in vec.iter() {
                    if !string.eq("[") {
                        string.push_str(", ");
                    }

                    string.push_str(&format!("{}", bson));
                }

                string.push_str("]");
                string
            }
            &Bson::Document(ref doc) => format!("{}", doc),
            &Bson::Boolean(b) => format!("{}", b),
            &Bson::Null => "null".to_owned(),
            &Bson::RegExp(ref pat, ref opt) => format!("/{}/{}", pat, opt),
            &Bson::JavaScriptCode(ref s) |
            &Bson::JavaScriptCodeWithScope(ref s, _) => s.to_owned(),
            &Bson::I32(i) => format!("{}", i),
            &Bson::I64(i) => format!("{}", i),
            &Bson::TimeStamp(i) => {
                let time = (i >> 32) as i32;
                let inc = (i & 0xFFFFFFFF) as i32;

                format!("Timestamp({}, {})", time, inc)
            }
            &Bson::Binary(t, ref vec) => format!("BinData({}, 0x{})", u8::from(t), vec.to_hex()),
            &Bson::ObjectId(ref id) => {
                let mut vec = vec![];

                for byte in id.bytes().iter() {
                    vec.push(byte.to_owned());
                }

                let string = unsafe { String::from_utf8_unchecked(vec) };
                format!("ObjectId(\"{}\")", string)
            }
            &Bson::UtcDatetime(date_time) => format!("Date(\"{}\")", date_time)
        };

        fmt.write_str(&bson_string)
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
        }
    }

    /// Convert this value to the best approximate `Json`.
    pub fn to_json(&self) -> json::Json {
        match self {
            &Bson::FloatingPoint(v) => json::Json::F64(v),
            &Bson::String(ref v) => json::Json::String(v.clone()),
            &Bson::Array(ref v) =>
                json::Json::Array(v.iter().map(|x| x.to_json()).collect()),
            &Bson::Document(ref v) =>
                json::Json::Object(v.iter().map(|(k, v)| (k.clone(), v.to_json())).collect()),
            &Bson::Boolean(v) => json::Json::Boolean(v),
            &Bson::Null => json::Json::Null,
            &Bson::RegExp(ref pat, ref opt) => {
                let mut re = json::Object::new();
                re.insert("pattern".to_owned(), json::Json::String(pat.clone()));
                re.insert("options".to_owned(), json::Json::String(opt.clone()));

                json::Json::Object(re)
            },
            &Bson::JavaScriptCode(ref code) => json::Json::String(code.clone()),
            &Bson::JavaScriptCodeWithScope(ref code, ref scope) => {
                let mut obj = json::Object::new();
                obj.insert("code".to_owned(), json::Json::String(code.clone()));

                let scope_obj =
                    scope.iter().map(|(k, v)| (k.clone(), v.to_json())).collect();

                obj.insert("scope".to_owned(), json::Json::Object(scope_obj));

                json::Json::Object(obj)
            },
            &Bson::I32(v) => json::Json::I64(v as i64),
            &Bson::I64(v) => json::Json::I64(v),
            &Bson::TimeStamp(v) => json::Json::I64(v),
            &Bson::Binary(t, ref v) => {
                let mut obj = json::Object::new();
                let tval: u8 = From::from(t);
                obj.insert("type".to_owned(), json::Json::I64(tval as i64));
                obj.insert("data".to_owned(), json::Json::String(v.to_hex()));

                json::Json::Object(obj)
            },
            &Bson::ObjectId(ref v) => json::Json::String(v.bytes().to_hex()),
            &Bson::UtcDatetime(ref v) => json::Json::String(v.to_string()),
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
            &json::Json::Object(ref x) => Bson::Document(x.iter().map(|(k, v)| (k.clone(), Bson::from_json(v))).collect()),
            &json::Json::Null => Bson::Null,
        }
    }
}
