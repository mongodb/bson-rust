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

use std::{
    fmt::{self, Debug, Display},
    ops::{Deref, DerefMut},
};

use chrono::{offset::TimeZone, DateTime, Timelike, Utc};
use serde_json::{json, Value};

#[cfg(feature = "decimal128")]
use crate::decimal128::Decimal128;
pub use crate::document::Document;
use crate::{
    oid,
    spec::{BinarySubtype, ElementType},
};

/// Possible BSON value types.
#[derive(Clone, Debug, PartialEq)]
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
    /// Regular expression
    Regex(Regex),
    /// JavaScript code
    JavaScriptCode(String),
    /// JavaScript code w/ scope
    JavaScriptCodeWithScope(JavaScriptCodeWithScope),
    /// 32-bit integer
    I32(i32),
    /// 64-bit integer
    I64(i64),
    /// Timestamp
    TimeStamp(TimeStamp),
    /// Binary data
    Binary(Binary),
    /// [ObjectId](http://dochub.mongodb.org/core/objectids)
    ObjectId(oid::ObjectId),
    /// UTC datetime
    UtcDatetime(DateTime<Utc>),
    /// Symbol (Deprecated)
    Symbol(String),
    /// [128-bit decimal floating point](https://github.com/mongodb/specifications/blob/master/source/bson-decimal128/decimal128.rst)
    #[cfg(feature = "decimal128")]
    Decimal128(Decimal128),
    /// Undefined value (Deprecated)
    Undefined,
    /// Max key
    MaxKey,
    /// Min key
    MinKey,
    /// DBPointer (Deprecated)
    DbPointer(DbPointer),
}

/// Alias for `Vec<Bson>`.
pub type Array = Vec<Bson>;

impl Default for Bson {
    fn default() -> Self {
        Bson::Null
    }
}

impl Display for Bson {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Bson::FloatingPoint(f) => write!(fmt, "{}", f),
            Bson::String(ref s) => write!(fmt, "\"{}\"", s),
            Bson::Array(ref vec) => {
                fmt.write_str("[")?;

                let mut first = true;
                for bson in vec {
                    if !first {
                        fmt.write_str(", ")?;
                    }

                    write!(fmt, "{}", bson)?;
                    first = false;
                }

                fmt.write_str("]")
            }
            Bson::Document(ref doc) => write!(fmt, "{}", doc),
            Bson::Boolean(b) => write!(fmt, "{}", b),
            Bson::Null => write!(fmt, "null"),
            Bson::Regex(Regex {
                ref pattern,
                ref options,
            }) => write!(fmt, "/{}/{}", pattern, options),
            Bson::JavaScriptCode(ref code)
            | Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope { ref code, .. }) => {
                fmt.write_str(&code)
            }
            Bson::I32(i) => write!(fmt, "{}", i),
            Bson::I64(i) => write!(fmt, "{}", i),
            Bson::TimeStamp(TimeStamp { time, increment }) => {
                write!(fmt, "Timestamp({}, {})", time, increment)
            }
            Bson::Binary(Binary { subtype, ref bytes }) => write!(
                fmt,
                "BinData({}, 0x{})",
                u8::from(subtype),
                hex::encode(bytes)
            ),
            Bson::ObjectId(ref id) => write!(fmt, "ObjectId(\"{}\")", id),
            Bson::UtcDatetime(date_time) => write!(fmt, "Date(\"{}\")", date_time),
            Bson::Symbol(ref sym) => write!(fmt, "Symbol(\"{}\")", sym),
            #[cfg(feature = "decimal128")]
            Bson::Decimal128(ref d) => write!(fmt, "Decimal128({})", d),
            Bson::Undefined => write!(fmt, "undefined"),
            Bson::MinKey => write!(fmt, "MinKey"),
            Bson::MaxKey => write!(fmt, "MaxKey"),
            Bson::DbPointer(DbPointer {
                ref namespace,
                ref id,
            }) => write!(fmt, "DBPointer({}, {})", namespace, id),
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

impl From<&str> for Bson {
    fn from(s: &str) -> Bson {
        Bson::String(s.to_owned())
    }
}

impl From<String> for Bson {
    fn from(a: String) -> Bson {
        Bson::String(a)
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

impl From<Regex> for Bson {
    fn from(regex: Regex) -> Bson {
        Bson::Regex(regex)
    }
}

impl From<JavaScriptCodeWithScope> for Bson {
    fn from(code_with_scope: JavaScriptCodeWithScope) -> Bson {
        Bson::JavaScriptCodeWithScope(code_with_scope)
    }
}

impl From<Binary> for Bson {
    fn from(binary: Binary) -> Bson {
        Bson::Binary(binary)
    }
}

impl<T> From<&T> for Bson
where
    T: Clone + Into<Bson>,
{
    fn from(t: &T) -> Bson {
        t.clone().into()
    }
}

impl<T> From<Vec<T>> for Bson
where
    T: Into<Bson>,
{
    fn from(v: Vec<T>) -> Bson {
        Bson::Array(v.into_iter().map(|val| val.into()).collect())
    }
}

impl<T> From<&[T]> for Bson
where
    T: Clone + Into<Bson>,
{
    fn from(s: &[T]) -> Bson {
        Bson::Array(s.iter().cloned().map(|val| val.into()).collect())
    }
}

impl<T: Into<Bson>> ::std::iter::FromIterator<T> for Bson {
    /// # Examples
    ///
    /// ```
    /// use std::iter::FromIterator;
    /// use bson::Bson;
    ///
    /// let x: Bson = Bson::from_iter(vec!["lorem", "ipsum", "dolor"]);
    /// // or
    /// let x: Bson = vec!["lorem", "ipsum", "dolor"].into_iter().collect();
    /// ```
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Bson::Array(iter.into_iter().map(Into::into).collect())
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
        Bson::ObjectId(a)
    }
}

impl From<DateTime<Utc>> for Bson {
    fn from(a: DateTime<Utc>) -> Bson {
        Bson::UtcDatetime(a)
    }
}

impl From<DbPointer> for Bson {
    fn from(a: DbPointer) -> Bson {
        Bson::DbPointer(a)
    }
}

impl From<Value> for Bson {
    fn from(a: Value) -> Bson {
        match a {
            Value::Number(x) => x
                .as_i64()
                .map(Bson::from)
                .or_else(|| x.as_u64().map(Bson::from))
                .or_else(|| x.as_f64().map(Bson::from))
                .unwrap_or_else(|| panic!("Invalid number value: {}", x)),
            Value::String(x) => x.into(),
            Value::Bool(x) => x.into(),
            Value::Array(x) => Bson::Array(x.into_iter().map(Bson::from).collect()),
            Value::Object(x) => {
                Bson::from_extended_document(x.into_iter().map(|(k, v)| (k, v.into())).collect())
            }
            Value::Null => Bson::Null,
        }
    }
}

impl From<Bson> for Value {
    fn from(bson: Bson) -> Self {
        match bson {
            Bson::FloatingPoint(v) => json!(v),
            Bson::String(v) => json!(v),
            Bson::Array(v) => json!(v),
            Bson::Document(v) => json!(v),
            Bson::Boolean(v) => json!(v),
            Bson::Null => Value::Null,
            Bson::Regex(Regex { pattern, options }) => json!({
                "$regex": pattern,
                "$options": options
            }),
            Bson::JavaScriptCode(code) => json!({ "$code": code }),
            Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope { code, scope }) => json!({
                "$code": code,
                "scope": scope
            }),
            Bson::I32(v) => v.into(),
            Bson::I64(v) => v.into(),
            Bson::TimeStamp(TimeStamp { time, increment }) => json!({
                "t": time,
                "i": increment
            }),
            Bson::Binary(Binary { subtype, ref bytes }) => {
                let tval: u8 = From::from(subtype);
                json!({
                    "type": tval,
                    "$binary": hex::encode(bytes),
                })
            }
            Bson::ObjectId(v) => json!({"$oid": v.to_string()}),
            Bson::UtcDatetime(v) => json!({
                "$date": {
                    "$numberLong": (v.timestamp() * 1000) + ((v.nanosecond() / 1_000_000) as i64)
                }
            }),
            // FIXME: Don't know what is the best way to encode Symbol type
            Bson::Symbol(v) => json!({ "$symbol": v }),
            #[cfg(feature = "decimal128")]
            Bson::Decimal128(ref v) => json!({ "$numberDecimal": v.to_string() }),
            Bson::Undefined => json!({ "$undefined": true }),
            Bson::MinKey => json!({ "$minKey": 1 }),
            Bson::MaxKey => json!({ "$maxKey": 1 }),
            Bson::DbPointer(DbPointer {
                ref namespace,
                ref id,
            }) => json!({ "$dbPointer": { "$ref": namespace, "$id": id.to_string() } }),
        }
    }
}

impl Bson {
    /// Get the `ElementType` of this value.
    pub fn element_type(&self) -> ElementType {
        match *self {
            Bson::FloatingPoint(..) => ElementType::FloatingPoint,
            Bson::String(..) => ElementType::Utf8String,
            Bson::Array(..) => ElementType::Array,
            Bson::Document(..) => ElementType::EmbeddedDocument,
            Bson::Boolean(..) => ElementType::Boolean,
            Bson::Null => ElementType::NullValue,
            Bson::Regex(..) => ElementType::RegularExpression,
            Bson::JavaScriptCode(..) => ElementType::JavaScriptCode,
            Bson::JavaScriptCodeWithScope(..) => ElementType::JavaScriptCodeWithScope,
            Bson::I32(..) => ElementType::Integer32Bit,
            Bson::I64(..) => ElementType::Integer64Bit,
            Bson::TimeStamp(..) => ElementType::TimeStamp,
            Bson::Binary(..) => ElementType::Binary,
            Bson::ObjectId(..) => ElementType::ObjectId,
            Bson::UtcDatetime(..) => ElementType::UtcDatetime,
            Bson::Symbol(..) => ElementType::Symbol,
            #[cfg(feature = "decimal128")]
            Bson::Decimal128(..) => ElementType::Decimal128Bit,
            Bson::Undefined => ElementType::Undefined,
            Bson::MaxKey => ElementType::MaxKey,
            Bson::MinKey => ElementType::MinKey,
            Bson::DbPointer(..) => ElementType::DbPointer,
        }
    }

    /// Converts to extended format.
    /// This function mainly used for [extended JSON format](https://docs.mongodb.com/manual/reference/mongodb-extended-json/).
    #[doc(hidden)]
    pub fn to_extended_document(&self) -> Document {
        match *self {
            Bson::Regex(Regex {
                ref pattern,
                ref options,
            }) => {
                doc! {
                    "$regex": pattern.clone(),
                    "$options": options.clone(),
                }
            }
            Bson::JavaScriptCode(ref code) => {
                doc! {
                    "$code": code.clone(),
                }
            }
            Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope {
                ref code,
                ref scope,
            }) => {
                doc! {
                    "$code": code.clone(),
                    "$scope": scope.clone(),
                }
            }
            Bson::TimeStamp(TimeStamp { time, increment }) => {
                doc! {
                    "t": time,
                    "i": increment
                }
            }
            Bson::Binary(Binary { subtype, ref bytes }) => {
                let tval: u8 = From::from(subtype);
                doc! {
                    "$binary": hex::encode(bytes),
                    "type": tval as i64,
                }
            }
            Bson::ObjectId(ref v) => {
                doc! {
                    "$oid": v.to_string(),
                }
            }
            Bson::UtcDatetime(ref v) => {
                doc! {
                    "$date": {
                        "$numberLong": (v.timestamp() * 1000) + v.nanosecond() as i64 / 1_000_000,
                    }
                }
            }
            Bson::Symbol(ref v) => {
                doc! {
                    "$symbol": v.to_owned(),
                }
            }
            #[cfg(feature = "decimal128")]
            Bson::Decimal128(ref v) => {
                doc! {
                    "$numberDecimal": (v.to_string())
                }
            }
            Bson::Undefined => {
                doc! {
                    "$undefined": true,
                }
            }
            Bson::MinKey => {
                doc! {
                    "$minKey": 1,
                }
            }
            Bson::MaxKey => {
                doc! {
                    "$maxKey": 1,
                }
            }
            Bson::DbPointer(DbPointer {
                ref namespace,
                ref id,
            }) => {
                doc! {
                    "$dbPointer": {
                        "$ref": namespace,
                        "$id": id.to_string()
                    }
                }
            }
            _ => panic!("Attempted conversion of invalid data type: {}", self),
        }
    }

    /// Converts from extended format.
    /// This function is mainly used for [extended JSON format](https://docs.mongodb.com/manual/reference/mongodb-extended-json/).
    #[cfg(feature = "decimal128")]
    #[doc(hidden)]
    pub fn from_extended_document(values: Document) -> Bson {
        if values.len() == 2 {
            if let (Ok(pat), Ok(opt)) = (values.get_str("$regex"), values.get_str("$options")) {
                return Bson::Regex(Regex {
                    pattern: pat.to_owned(),
                    options: opt.to_owned(),
                });
            } else if let (Ok(code), Ok(scope)) =
                (values.get_str("$code"), values.get_document("$scope"))
            {
                return Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope {
                    code: code.to_owned(),
                    scope: scope.to_owned(),
                });
            } else if let (Ok(time), Ok(increment)) = (values.get_i32("t"), values.get_i32("i")) {
                if time >= 0 && increment >= 0 {
                    return Bson::TimeStamp(TimeStamp {
                        time: time as u32,
                        increment: increment as u32,
                    });
                }
            } else if let (Ok(time), Ok(increment)) = (values.get_i64("t"), values.get_i64("i")) {
                if time >= 0
                    && time <= std::u32::MAX as i64
                    && increment >= 0
                    && increment <= std::u32::MAX as i64
                {
                    return Bson::TimeStamp(TimeStamp {
                        time: time as u32,
                        increment: increment as u32,
                    });
                }
            } else if let (Ok(hex), Ok(t)) = (values.get_str("$binary"), values.get_i64("type")) {
                let ttype = t as u8;
                return Bson::Binary(Binary {
                    subtype: From::from(ttype),
                    bytes: hex::decode(hex.as_bytes())
                        .expect("$binary value is not a valid Hex encoded bytes"),
                });
            }
        } else if values.len() == 1 {
            if let Ok(code) = values.get_str("$code") {
                return Bson::JavaScriptCode(code.to_owned());
            } else if let Ok(hex) = values.get_str("$oid") {
                return Bson::ObjectId(oid::ObjectId::with_string(hex).unwrap());
            } else if let Ok(long) = values
                .get_document("$date")
                .and_then(|inner| inner.get_i64("$numberLong"))
            {
                return Bson::UtcDatetime(
                    Utc.timestamp(long / 1000, ((long % 1000) * 1_000_000) as u32),
                );
            } else if let Ok(sym) = values.get_str("$symbol") {
                return Bson::Symbol(sym.to_owned());
            } else if let Ok(dec) = values.get_str("$numberDecimal") {
                return Bson::Decimal128(dec.parse::<Decimal128>().unwrap());
            } else if let Ok(undefined) = values.get_bool("$undefined") {
                if undefined {
                    return Bson::Undefined;
                }
            } else if let Ok(min) = values.get_i64("$minKey") {
                if min == 1 {
                    return Bson::MinKey;
                }
            } else if let Ok(max) = values.get_i64("$maxKey") {
                if max == 1 {
                    return Bson::MaxKey;
                }
            } else if let Ok(db_pointer) = values.get_document("$dbPointer") {
                if let (Ok(namespace), Ok(id)) =
                    (db_pointer.get_str("$ref"), db_pointer.get_str("$id"))
                {
                    return Bson::DbPointer(DbPointer {
                        namespace: namespace.to_owned(),
                        id: oid::ObjectId::with_string(id).unwrap(),
                    });
                }
            }
        }

        Bson::Document(values)
    }

    /// Converts from extended format.
    /// This function is mainly used for [extended JSON format](https://docs.mongodb.com/manual/reference/mongodb-extended-json/).
    #[cfg(not(feature = "decimal128"))]
    #[doc(hidden)]
    pub fn from_extended_document(values: Document) -> Bson {
        if values.len() == 2 {
            if let (Ok(pat), Ok(opt)) = (values.get_str("$regex"), values.get_str("$options")) {
                return Bson::Regex(Regex {
                    pattern: pat.to_owned(),
                    options: opt.to_owned(),
                });
            } else if let (Ok(code), Ok(scope)) =
                (values.get_str("$code"), values.get_document("$scope"))
            {
                return Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope {
                    code: code.to_owned(),
                    scope: scope.to_owned(),
                });
            } else if let (Ok(time), Ok(increment)) = (values.get_i32("t"), values.get_i32("i")) {
                if time >= 0 && increment >= 0 {
                    return Bson::TimeStamp(TimeStamp {
                        time: time as u32,
                        increment: increment as u32,
                    });
                }
            } else if let (Ok(time), Ok(increment)) = (values.get_i64("t"), values.get_i64("i")) {
                if time >= 0
                    && time <= std::u32::MAX as i64
                    && increment >= 0
                    && increment <= std::u32::MAX as i64
                {
                    return Bson::TimeStamp(TimeStamp {
                        time: time as u32,
                        increment: increment as u32,
                    });
                }
            } else if let (Ok(hex), Ok(t)) = (values.get_str("$binary"), values.get_i64("type")) {
                let ttype = t as u8;
                return Bson::Binary(Binary {
                    subtype: From::from(ttype),
                    bytes: hex::decode(hex.as_bytes())
                        .expect("$binary value is not a valid Hex encoded bytes"),
                });
            }
        } else if values.len() == 1 {
            if let Ok(code) = values.get_str("$code") {
                return Bson::JavaScriptCode(code.to_owned());
            } else if let Ok(hex) = values.get_str("$oid") {
                return Bson::ObjectId(oid::ObjectId::with_string(hex).unwrap());
            } else if let Ok(long) = values
                .get_document("$date")
                .and_then(|inner| inner.get_i64("$numberLong"))
            {
                return Bson::UtcDatetime(
                    Utc.timestamp(long / 1000, ((long % 1000) * 1_000_000) as u32),
                );
            } else if let Ok(sym) = values.get_str("$symbol") {
                return Bson::Symbol(sym.to_owned());
            } else if let Ok(undefined) = values.get_bool("$undefined") {
                if undefined {
                    return Bson::Undefined;
                }
            } else if let Ok(min) = values.get_i64("$minKey") {
                if min == 1 {
                    return Bson::MinKey;
                }
            } else if let Ok(max) = values.get_i64("$maxKey") {
                if max == 1 {
                    return Bson::MaxKey;
                }
            } else if let Ok(db_pointer) = values.get_document("$dbPointer") {
                if let (Ok(namespace), Ok(id)) =
                    (db_pointer.get_str("$ref"), db_pointer.get_str("$id"))
                {
                    return Bson::DbPointer(DbPointer {
                        namespace: namespace.to_owned(),
                        id: oid::ObjectId::with_string(id).unwrap(),
                    });
                }
            }
        }

        Bson::Document(values)
    }
}

/// Value helpers
impl Bson {
    /// If `Bson` is `FloatingPoint`, return its value. Returns `None` otherwise
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Bson::FloatingPoint(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `String`, return its value. Returns `None` otherwise
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Bson::String(ref s) => Some(s),
            _ => None,
        }
    }

    /// If `Bson` is `String`, return a mutable reference to its value. Returns `None` otherwise
    pub fn as_str_mut(&mut self) -> Option<&mut str> {
        match *self {
            Bson::String(ref mut s) => Some(s),
            _ => None,
        }
    }

    /// If `Bson` is `Array`, return its value. Returns `None` otherwise
    pub fn as_array(&self) -> Option<&Array> {
        match *self {
            Bson::Array(ref v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Array`, return a mutable reference to its value. Returns `None` otherwise
    pub fn as_array_mut(&mut self) -> Option<&mut Array> {
        match *self {
            Bson::Array(ref mut v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Document`, return its value. Returns `None` otherwise
    pub fn as_document(&self) -> Option<&Document> {
        match *self {
            Bson::Document(ref v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Document`, return a mutable reference to its value. Returns `None` otherwise
    pub fn as_document_mut(&mut self) -> Option<&mut Document> {
        match *self {
            Bson::Document(ref mut v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Boolean`, return its value. Returns `None` otherwise
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Bson::Boolean(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `I32`, return its value. Returns `None` otherwise
    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            Bson::I32(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `I64`, return its value. Returns `None` otherwise
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Bson::I64(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Objectid`, return its value. Returns `None` otherwise
    pub fn as_object_id(&self) -> Option<&oid::ObjectId> {
        match *self {
            Bson::ObjectId(ref v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Objectid`, return a mutable reference to its value. Returns `None` otherwise
    pub fn as_object_id_mut(&mut self) -> Option<&mut oid::ObjectId> {
        match *self {
            Bson::ObjectId(ref mut v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `UtcDateTime`, return its value. Returns `None` otherwise
    pub fn as_utc_date_time(&self) -> Option<&DateTime<Utc>> {
        match *self {
            Bson::UtcDatetime(ref v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `UtcDateTime`, return a mutable reference to its value. Returns `None`
    /// otherwise
    pub fn as_utc_date_time_mut(&mut self) -> Option<&mut DateTime<Utc>> {
        match *self {
            Bson::UtcDatetime(ref mut v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Symbol`, return its value. Returns `None` otherwise
    pub fn as_symbol(&self) -> Option<&str> {
        match *self {
            Bson::Symbol(ref v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Symbol`, return a mutable reference to its value. Returns `None` otherwise
    pub fn as_symbol_mut(&mut self) -> Option<&mut str> {
        match *self {
            Bson::Symbol(ref mut v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `TimeStamp`, return its value. Returns `None` otherwise
    pub fn as_timestamp(&self) -> Option<TimeStamp> {
        match *self {
            Bson::TimeStamp(timestamp) => Some(timestamp),
            _ => None,
        }
    }

    /// If `Bson` is `Null`, return its value. Returns `None` otherwise
    pub fn as_null(&self) -> Option<()> {
        match *self {
            Bson::Null => Some(()),
            _ => None,
        }
    }

    pub fn as_db_pointer(&self) -> Option<&DbPointer> {
        match self {
            Bson::DbPointer(ref db_pointer) => Some(db_pointer),
            _ => None,
        }
    }
}

/// Represents a BSON timestamp value.
#[derive(Debug, Eq, PartialEq, Clone, Copy, Hash)]
pub struct TimeStamp {
    /// The number of seconds since the Unix epoch.
    pub time: u32,

    /// An incrementing value to order timestamps with the same number of seconds in the `time`
    /// field.
    pub increment: u32,
}

impl TimeStamp {
    pub(crate) fn to_le_i64(self) -> i64 {
        let upper = (self.time.to_le() as u64) << 32;
        let lower = self.increment.to_le() as u64;

        (upper | lower) as i64
    }

    pub(crate) fn from_le_i64(val: i64) -> Self {
        let ts = val.to_le();

        TimeStamp {
            time: ((ts as u64) >> 32) as u32,
            increment: (ts & 0xFFFF_FFFF) as u32,
        }
    }
}

/// `DateTime` representation in struct for serde serialization
///
/// Just a helper for convenience
///
/// ```rust,ignore
/// use serde::{Serialize, Deserialize};
/// use bson::UtcDateTime;
///
/// #[derive(Serialize, Deserialize)]
/// struct Foo {
///     date_time: UtcDateTime,
/// }
/// ```
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
pub struct UtcDateTime(pub DateTime<Utc>);

impl Deref for UtcDateTime {
    type Target = DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for UtcDateTime {
    fn deref_mut(&mut self) -> &mut DateTime<Utc> {
        &mut self.0
    }
}

impl From<UtcDateTime> for DateTime<Utc> {
    fn from(utc: UtcDateTime) -> Self {
        utc.0
    }
}

impl From<DateTime<Utc>> for UtcDateTime {
    fn from(x: DateTime<Utc>) -> Self {
        UtcDateTime(x)
    }
}

/// Represents a BSON regular expression value.
#[derive(Debug, Clone, PartialEq)]
pub struct Regex {
    /// The regex pattern to match.
    pub pattern: String,

    /// The options for the regex.
    ///
    /// Options are identified by characters, which must be stored in
    /// alphabetical order. Valid options are 'i' for case insensitive matching, 'm' for
    /// multiline matching, 'x' for verbose mode, 'l' to make \w, \W, etc. locale dependent,
    /// 's' for dotall mode ('.' matches everything), and 'u' to make \w, \W, etc. match
    /// unicode.
    pub options: String,
}

/// Represents a BSON code with scope value.
#[derive(Debug, Clone, PartialEq)]
pub struct JavaScriptCodeWithScope {
    pub code: String,
    pub scope: Document,
}

/// Represents a BSON binary value.
#[derive(Debug, Clone, PartialEq)]
pub struct Binary {
    /// The subtype of the bytes.
    pub subtype: BinarySubtype,

    /// The binary bytes.
    pub bytes: Vec<u8>,
}

/// Represents a DBPointer. (Deprecated)
#[derive(Debug, Clone, PartialEq)]
pub struct DbPointer {
    pub(crate) namespace: String,
    pub(crate) id: oid::ObjectId,
}
