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

use oid;
use ordered::OrderedDocument;
use rustc_serialize::hex::{FromHex, ToHex};
use serde_json:: Value;
use spec::{ElementType, BinarySubtype};

/// Possible BSON value types.
#[derive(Clone, PartialEq)]
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
    Symbol(String),
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
        }
    }

    /// Convert this value to the best approximate `Json`.
    pub fn to_json(&self) -> Value {
        match self {
            &Bson::FloatingPoint(v) => json!(v),
            &Bson::String(ref v) => json!(v),
            &Bson::Array(ref v) => json!(v),
            &Bson::Document(ref v) => json!(v),
            &Bson::Boolean(v) => json!(v),
            &Bson::Null => Value::Null,
            &Bson::RegExp(ref pat, ref opt) => json!({
                "$regex": pat,
                "$options": opt
            }),
            &Bson::JavaScriptCode(ref code) => json!({"$code": code}),
            &Bson::JavaScriptCodeWithScope(ref code, ref scope) => {
                json!({
                    "$code": code,
                    "scope": scope
                })
            },
            &Bson::I32(v) => v.into(),
            &Bson::I64(v) => v.into(),
            &Bson::TimeStamp(v) => {
                let time = v >> 32;
                let inc = v & 0x0000FFFF;

                json!({
                    "t": time,
                    "i": inc
                })
            },
            &Bson::Binary(t, ref v) => {
                let tval: u8 = From::from(t);
                json!({
                    "type": tval,
                    "$binary": v.to_hex()
                })
            },
            &Bson::ObjectId(ref v) => json!({"$oid": v.to_string()}),
            &Bson::UtcDatetime(ref v) => json!({
                "$date": {
                    "$numberLong": (v.timestamp() * 1000) + ((v.nanosecond() / 1000000) as i64)
                }
            }),
            // FIXME: Don't know what is the best way to encode Symbol type
            &Bson::Symbol(ref v) => json!({"$symbol": v})
        }
    }

    /// Create a `Bson` from a `Json`.
    pub fn from_json(j: &Value) -> Bson {
        match j {
            &Value::Number(ref x) =>
                x.as_i64().map(Bson::from)
                .or(x.as_u64().map(Bson::from))
                .expect(&format!("Invalid number value: {}", x)),
            &Value::String(ref x) => x.into(),
            &Value::Bool(x) => x.into(),
            &Value::Array(ref x) => Bson::Array(x.iter().map(Bson::from_json).collect()),
            &Value::Object(ref x) => {
                Bson::from_extended_document(x.iter()
                    .map(|(k, v)| (k.clone(), Bson::from_json(v)))
                    .collect())
            }
            &Value::Null => Bson::Null,
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
            }
        }

        Bson::Document(values)
    }
}
