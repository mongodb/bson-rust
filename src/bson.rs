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

use chrono::{Datelike, SecondsFormat, TimeZone, Utc};
use serde_json::{json, Value};

pub use crate::document::Document;
use crate::{
    oid::{self, ObjectId},
    spec::{BinarySubtype, ElementType},
    Decimal128,
};

/// Possible BSON value types.
#[derive(Clone, Debug, PartialEq)]
pub enum Bson {
    /// 64-bit binary floating point
    Double(f64),
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
    RegularExpression(Regex),
    /// JavaScript code
    JavaScriptCode(String),
    /// JavaScript code w/ scope
    JavaScriptCodeWithScope(JavaScriptCodeWithScope),
    /// 32-bit signed integer
    Int32(i32),
    /// 64-bit signed integer
    Int64(i64),
    /// Timestamp
    Timestamp(Timestamp),
    /// Binary data
    Binary(Binary),
    /// [ObjectId](http://dochub.mongodb.org/core/objectids)
    ObjectId(oid::ObjectId),
    /// UTC datetime
    DateTime(chrono::DateTime<Utc>),
    /// Symbol (Deprecated)
    Symbol(String),
    /// [128-bit decimal floating point](https://github.com/mongodb/specifications/blob/master/source/bson-decimal128/decimal128.rst)
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
            Bson::Double(f) => write!(fmt, "{}", f),
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
            Bson::RegularExpression(Regex {
                ref pattern,
                ref options,
            }) => write!(fmt, "/{}/{}", pattern, options),
            Bson::JavaScriptCode(ref code)
            | Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope { ref code, .. }) => {
                fmt.write_str(&code)
            }
            Bson::Int32(i) => write!(fmt, "{}", i),
            Bson::Int64(i) => write!(fmt, "{}", i),
            Bson::Timestamp(Timestamp { time, increment }) => {
                write!(fmt, "Timestamp({}, {})", time, increment)
            }
            Bson::Binary(Binary { subtype, ref bytes }) => write!(
                fmt,
                "BinData({:#x}, {})",
                u8::from(subtype),
                base64::encode(bytes)
            ),
            Bson::ObjectId(ref id) => write!(fmt, "ObjectId(\"{}\")", id),
            Bson::DateTime(date_time) => write!(fmt, "Date(\"{}\")", date_time),
            Bson::Symbol(ref sym) => write!(fmt, "Symbol(\"{}\")", sym),
            Bson::Decimal128(ref d) => write!(fmt, "{}", d),
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
        Bson::Double(a as f64)
    }
}

impl From<f64> for Bson {
    fn from(a: f64) -> Bson {
        Bson::Double(a)
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
        Bson::RegularExpression(regex)
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

impl From<Timestamp> for Bson {
    fn from(ts: Timestamp) -> Bson {
        Bson::Timestamp(ts)
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
        Bson::Int32(a)
    }
}

impl From<i64> for Bson {
    fn from(a: i64) -> Bson {
        Bson::Int64(a)
    }
}

impl From<u32> for Bson {
    fn from(a: u32) -> Bson {
        Bson::Int32(a as i32)
    }
}

impl From<u64> for Bson {
    fn from(a: u64) -> Bson {
        Bson::Int64(a as i64)
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

impl From<chrono::DateTime<Utc>> for Bson {
    fn from(a: chrono::DateTime<Utc>) -> Bson {
        Bson::DateTime(a)
    }
}

impl From<DbPointer> for Bson {
    fn from(a: DbPointer) -> Bson {
        Bson::DbPointer(a)
    }
}

/// This will create the [relaxed Extended JSON v2](https://docs.mongodb.com/manual/reference/mongodb-extended-json/) representation of the provided [`Bson`](../enum.Bson.html).
impl From<Bson> for Value {
    fn from(bson: Bson) -> Self {
        bson.into_relaxed_extjson()
    }
}

impl Bson {
    /// Converts the Bson value into its [relaxed extended JSON representation](https://docs.mongodb.com/manual/reference/mongodb-extended-json/).
    ///
    /// Note: extended json encoding for `Decimal128` values is not supported without the
    /// "decimal128" feature flag. If this method is called on a case which contains a
    /// `Decimal128` value, it will panic.
    pub fn into_relaxed_extjson(self) -> Value {
        match self {
            Bson::Double(v) if v.is_nan() => {
                let s = if v.is_sign_negative() { "-NaN" } else { "NaN" };

                json!({ "$numberDouble": s })
            }
            Bson::Double(v) if v.is_infinite() => {
                let s = if v.is_sign_negative() {
                    "-Infinity"
                } else {
                    "Infinity"
                };

                json!({ "$numberDouble": s })
            }
            Bson::Double(v) => json!(v),
            Bson::String(v) => json!(v),
            Bson::Array(v) => json!(v),
            Bson::Document(v) => {
                Value::Object(v.into_iter().map(|(k, v)| (k, Value::from(v))).collect())
            }
            Bson::Boolean(v) => json!(v),
            Bson::Null => Value::Null,
            Bson::RegularExpression(Regex { pattern, options }) => {
                let mut chars: Vec<_> = options.chars().collect();
                chars.sort();

                let options: String = chars.into_iter().collect();

                json!({
                    "$regularExpression": {
                        "pattern": pattern,
                        "options": options,
                    }
                })
            }
            Bson::JavaScriptCode(code) => json!({ "$code": code }),
            Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope { code, scope }) => json!({
                "$code": code,
                "$scope": scope,
            }),
            Bson::Int32(v) => v.into(),
            Bson::Int64(v) => v.into(),
            Bson::Timestamp(Timestamp { time, increment }) => json!({
                "$timestamp": {
                    "t": time,
                    "i": increment,
                }
            }),
            Bson::Binary(Binary { subtype, ref bytes }) => {
                let tval: u8 = From::from(subtype);
                json!({
                    "$binary": {
                        "base64": base64::encode(bytes),
                        "subType": hex::encode([tval]),
                    }
                })
            }
            Bson::ObjectId(v) => json!({"$oid": v.to_hex()}),
            Bson::DateTime(v) if v.timestamp_millis() >= 0 && v.year() <= 99999 => {
                let seconds_format = if v.timestamp_subsec_millis() == 0 {
                    SecondsFormat::Secs
                } else {
                    SecondsFormat::Millis
                };

                json!({
                    "$date": v.to_rfc3339_opts(seconds_format, true),
                })
            }
            Bson::DateTime(v) => json!({
                "$date": { "$numberLong": v.timestamp_millis().to_string() },
            }),
            Bson::Symbol(v) => json!({ "$symbol": v }),
            #[cfg(feature = "decimal128")]
            Bson::Decimal128(ref v) => json!({ "$numberDecimal": v.to_string() }),
            #[cfg(not(feature = "decimal128"))]
            Bson::Decimal128(_) => panic!(
                "Decimal128 extended JSON not implemented yet. Use the decimal128 feature to \
                 enable experimental support for it."
            ),
            Bson::Undefined => json!({ "$undefined": true }),
            Bson::MinKey => json!({ "$minKey": 1 }),
            Bson::MaxKey => json!({ "$maxKey": 1 }),
            Bson::DbPointer(DbPointer {
                ref namespace,
                ref id,
            }) => json!({
                "$dbPointer": {
                    "$ref": namespace,
                    "$id": {
                        "$oid": id.to_hex()
                    }
                }
            }),
        }
    }

    /// Converts the Bson value into its [canonical extended JSON representation](https://docs.mongodb.com/manual/reference/mongodb-extended-json/).
    ///
    /// Note: extended json encoding for `Decimal128` values is not supported without the
    /// "decimal128" feature flag. If this method is called on a case which contains a
    /// `Decimal128` value, it will panic.
    pub fn into_canonical_extjson(self) -> Value {
        match self {
            Bson::Int32(i) => json!({ "$numberInt": i.to_string() }),
            Bson::Int64(i) => json!({ "$numberLong": i.to_string() }),
            Bson::Double(f) if f.is_normal() => {
                let mut s = f.to_string();
                if f.fract() == 0.0 {
                    s.push_str(".0");
                }

                json!({ "$numberDouble": s })
            }
            Bson::Double(f) if f == 0.0 => {
                let s = if f.is_sign_negative() { "-0.0" } else { "0.0" };

                json!({ "$numberDouble": s })
            }
            Bson::DateTime(date) => {
                json!({ "$date": { "$numberLong": date.timestamp_millis().to_string() } })
            }
            Bson::Array(arr) => {
                Value::Array(arr.into_iter().map(Bson::into_canonical_extjson).collect())
            }
            Bson::Document(arr) => Value::Object(
                arr.into_iter()
                    .map(|(k, v)| (k, v.into_canonical_extjson()))
                    .collect(),
            ),
            Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope { code, scope }) => json!({
                "$code": code,
                "$scope": Bson::Document(scope).into_canonical_extjson(),
            }),

            other => other.into_relaxed_extjson(),
        }
    }

    /// Get the `ElementType` of this value.
    pub fn element_type(&self) -> ElementType {
        match *self {
            Bson::Double(..) => ElementType::Double,
            Bson::String(..) => ElementType::String,
            Bson::Array(..) => ElementType::Array,
            Bson::Document(..) => ElementType::EmbeddedDocument,
            Bson::Boolean(..) => ElementType::Boolean,
            Bson::Null => ElementType::Null,
            Bson::RegularExpression(..) => ElementType::RegularExpression,
            Bson::JavaScriptCode(..) => ElementType::JavaScriptCode,
            Bson::JavaScriptCodeWithScope(..) => ElementType::JavaScriptCodeWithScope,
            Bson::Int32(..) => ElementType::Int32,
            Bson::Int64(..) => ElementType::Int64,
            Bson::Timestamp(..) => ElementType::Timestamp,
            Bson::Binary(..) => ElementType::Binary,
            Bson::ObjectId(..) => ElementType::ObjectId,
            Bson::DateTime(..) => ElementType::DateTime,
            Bson::Symbol(..) => ElementType::Symbol,
            Bson::Decimal128(..) => ElementType::Decimal128,
            Bson::Undefined => ElementType::Undefined,
            Bson::MaxKey => ElementType::MaxKey,
            Bson::MinKey => ElementType::MinKey,
            Bson::DbPointer(..) => ElementType::DbPointer,
        }
    }

    /// Converts to extended format.
    /// This function mainly used for [extended JSON format](https://docs.mongodb.com/manual/reference/mongodb-extended-json/).
    // TODO RUST-426: Investigate either removing this from the serde implementation or unifying
    // with the extended JSON implementation.
    pub(crate) fn to_extended_document(&self) -> Document {
        match *self {
            Bson::RegularExpression(Regex {
                ref pattern,
                ref options,
            }) => {
                let mut chars: Vec<_> = options.chars().collect();
                chars.sort();

                let options: String = chars.into_iter().collect();

                doc! {
                    "$regularExpression": {
                        "pattern": pattern.clone(),
                        "options": options,
                    }
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
            Bson::Timestamp(Timestamp { time, increment }) => {
                doc! {
                    "$timestamp": {
                        "t": time,
                        "i": increment,
                    }
                }
            }
            Bson::Binary(Binary { subtype, ref bytes }) => {
                let tval: u8 = From::from(subtype);
                doc! {
                    "$binary": {
                        "base64": base64::encode(bytes),
                        "subType": hex::encode([tval]),
                    }
                }
            }
            Bson::ObjectId(ref v) => {
                doc! {
                    "$oid": v.to_string(),
                }
            }
            Bson::DateTime(v) if v.timestamp_millis() >= 0 && v.year() <= 99999 => {
                let seconds_format = if v.timestamp_subsec_millis() == 0 {
                    SecondsFormat::Secs
                } else {
                    SecondsFormat::Millis
                };

                doc! {
                    "$date": v.to_rfc3339_opts(seconds_format, true),
                }
            }
            Bson::DateTime(v) => doc! {
                "$date": { "$numberLong": v.timestamp_millis().to_string() },
            },
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
                        "$id": {
                            "$oid": id.to_string()
                        }
                    }
                }
            }
            _ => panic!("Attempted conversion of invalid data type: {}", self),
        }
    }

    pub(crate) fn from_extended_document(doc: Document) -> Bson {
        if doc.len() > 2 {
            return Bson::Document(doc);
        }

        let mut keys: Vec<_> = doc.keys().map(|s| s.as_str()).collect();
        keys.sort();

        match keys.as_slice() {
            ["$oid"] => {
                if let Ok(oid) = doc.get_str("$oid") {
                    if let Ok(oid) = ObjectId::with_string(oid) {
                        return Bson::ObjectId(oid);
                    }
                }
            }

            ["$symbol"] => {
                if let Ok(symbol) = doc.get_str("$symbol") {
                    return Bson::Symbol(symbol.into());
                }
            }

            ["$numberInt"] => {
                if let Ok(i) = doc.get_str("$numberInt") {
                    if let Ok(i) = i.parse() {
                        return Bson::Int32(i);
                    }
                }
            }

            ["$numberLong"] => {
                if let Ok(i) = doc.get_str("$numberLong") {
                    if let Ok(i) = i.parse() {
                        return Bson::Int64(i);
                    }
                }
            }

            ["$numberDouble"] => match doc.get_str("$numberDouble") {
                Ok("Infinity") => return Bson::Double(f64::INFINITY),
                Ok("-Infinity") => return Bson::Double(f64::NEG_INFINITY),
                Ok("NaN") => return Bson::Double(f64::NAN),
                Ok(other) => {
                    if let Ok(d) = other.parse() {
                        return Bson::Double(d);
                    }
                }
                _ => {}
            },

            #[cfg(feature = "decimal128")]
            ["$numberDecimal"] => {
                if let Ok(d) = doc.get_str("$numberDecimal") {
                    if let Ok(d) = d.parse() {
                        return Bson::Decimal128(d);
                    }
                }
            }

            ["$binary"] => {
                if let Some(binary) = Binary::from_extended_doc(&doc) {
                    return Bson::Binary(binary);
                }
            }

            ["$code"] => {
                if let Ok(code) = doc.get_str("$code") {
                    return Bson::JavaScriptCode(code.into());
                }
            }

            ["$code", "$scope"] => {
                if let Ok(code) = doc.get_str("$code") {
                    if let Ok(scope) = doc.get_document("$scope") {
                        return Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope {
                            code: code.into(),
                            scope: scope.clone(),
                        });
                    }
                }
            }

            ["$timestamp"] => {
                if let Ok(timestamp) = doc.get_document("$timestamp") {
                    if let Ok(t) = timestamp.get_i32("t") {
                        if let Ok(i) = timestamp.get_i32("i") {
                            return Bson::Timestamp(Timestamp {
                                time: t as u32,
                                increment: i as u32,
                            });
                        }
                    }

                    if let Ok(t) = timestamp.get_i64("t") {
                        if let Ok(i) = timestamp.get_i64("i") {
                            if t >= 0 && i >= 0 && t <= (u32::MAX as i64) && i <= (u32::MAX as i64)
                            {
                                return Bson::Timestamp(Timestamp {
                                    time: t as u32,
                                    increment: i as u32,
                                });
                            }
                        }
                    }
                }
            }

            ["$regularExpression"] => {
                if let Ok(regex) = doc.get_document("$regularExpression") {
                    if let Ok(pattern) = regex.get_str("pattern") {
                        if let Ok(options) = regex.get_str("options") {
                            let mut options: Vec<_> = options.chars().collect();
                            options.sort();

                            return Bson::RegularExpression(Regex {
                                pattern: pattern.into(),
                                options: options.into_iter().collect(),
                            });
                        }
                    }
                }
            }

            ["$dbPointer"] => {
                if let Ok(db_pointer) = doc.get_document("$dbPointer") {
                    if let Ok(ns) = db_pointer.get_str("$ref") {
                        if let Ok(id) = db_pointer.get_object_id("$id") {
                            return Bson::DbPointer(DbPointer {
                                namespace: ns.into(),
                                id: id.clone(),
                            });
                        }
                    }
                }
            }

            ["$date"] => {
                if let Ok(date) = doc.get_i64("$date") {
                    return Bson::DateTime(DateTime::from_i64(date).into());
                }

                if let Ok(date) = doc.get_str("$date") {
                    if let Ok(date) = chrono::DateTime::parse_from_rfc3339(date) {
                        return Bson::DateTime(date.into());
                    }
                }
            }

            ["$minKey"] => {
                let min_key = doc.get("$minKey");

                if min_key == Some(&Bson::Int32(1)) || min_key == Some(&Bson::Int64(1)) {
                    return Bson::MinKey;
                }
            }

            ["$maxKey"] => {
                let max_key = doc.get("$maxKey");

                if max_key == Some(&Bson::Int32(1)) || max_key == Some(&Bson::Int64(1)) {
                    return Bson::MaxKey;
                }
            }

            ["$undefined"] => {
                if doc.get("$undefined") == Some(&Bson::Boolean(true)) {
                    return Bson::Undefined;
                }
            }

            _ => {}
        };

        Bson::Document(
            doc.into_iter()
                .map(|(k, v)| {
                    let v = match v {
                        Bson::Document(v) => Bson::from_extended_document(v),
                        other => other,
                    };

                    (k, v)
                })
                .collect(),
        )
    }
}

/// Value helpers
impl Bson {
    /// If `Bson` is `Double`, return its value as an `f64`. Returns `None` otherwise
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Bson::Double(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `String`, return its value as a `&str`. Returns `None` otherwise
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Bson::String(ref s) => Some(s),
            _ => None,
        }
    }

    /// If `Bson` is `String`, return a mutable reference to its value as a `str`. Returns `None`
    /// otherwise
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

    /// If `Bson` is `Bool`, return its value. Returns `None` otherwise
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Bson::Boolean(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `I32`, return its value. Returns `None` otherwise
    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            Bson::Int32(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `I64`, return its value. Returns `None` otherwise
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Bson::Int64(v) => Some(v),
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

    /// If `Bson` is `DateTime`, return its value. Returns `None` otherwise
    pub fn as_datetime(&self) -> Option<&chrono::DateTime<Utc>> {
        match *self {
            Bson::DateTime(ref v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `DateTime`, return a mutable reference to its value. Returns `None`
    /// otherwise
    pub fn as_datetime_mut(&mut self) -> Option<&mut chrono::DateTime<Utc>> {
        match *self {
            Bson::DateTime(ref mut v) => Some(v),
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

    /// If `Bson` is `Timestamp`, return its value. Returns `None` otherwise
    pub fn as_timestamp(&self) -> Option<Timestamp> {
        match *self {
            Bson::Timestamp(timestamp) => Some(timestamp),
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
pub struct Timestamp {
    /// The number of seconds since the Unix epoch.
    pub time: u32,

    /// An incrementing value to order timestamps with the same number of seconds in the `time`
    /// field.
    pub increment: u32,
}

impl Timestamp {
    pub(crate) fn to_le_i64(self) -> i64 {
        let upper = (self.time.to_le() as u64) << 32;
        let lower = self.increment.to_le() as u64;

        (upper | lower) as i64
    }

    pub(crate) fn from_le_i64(val: i64) -> Self {
        let ts = val.to_le();

        Timestamp {
            time: ((ts as u64) >> 32) as u32,
            increment: (ts & 0xFFFF_FFFF) as u32,
        }
    }
}

/// `DateTime` representation in struct for serde serialization
///
/// Just a helper for convenience
///
/// ```rust
/// use serde::{Serialize, Deserialize};
/// use bson::DateTime;
///
/// #[derive(Serialize, Deserialize)]
/// struct Foo {
///     date_time: DateTime,
/// }
/// ```
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Copy, Clone)]
pub struct DateTime(pub chrono::DateTime<Utc>);

impl DateTime {
    pub(crate) fn from_i64(date: i64) -> Self {
        let mut num_secs = date / 1000;
        let mut num_millis = date % 1000;

        // The chrono API only lets us create a DateTime with an i64 number of seconds
        // and a u32 number of nanoseconds. In the case of a negative timestamp, this
        // means that we need to turn the negative fractional part into a positive and
        // shift the number of seconds down. For example:
        //
        //     date       = -4300 ms
        //     num_secs   = date / 1000 = -4300 / 1000 = -4
        //     num_millis = date % 1000 = -4300 % 1000 = -300
        //
        // Since num_millis is less than 0:
        //     num_secs   = num_secs -1 = -4 - 1 = -5
        //     num_millis = num_nanos + 1000 = -300 + 1000 = 700
        //
        // Instead of -4 seconds and -300 milliseconds, we now have -5 seconds and +700
        // milliseconds, which expresses the same timestamp, but in a way we can create
        // a DateTime with.
        if num_millis < 0 {
            num_secs -= 1;
            num_millis += 1000;
        };

        Utc.timestamp(num_secs, num_millis as u32 * 1_000_000)
            .into()
    }
}

impl Deref for DateTime {
    type Target = chrono::DateTime<Utc>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for DateTime {
    fn deref_mut(&mut self) -> &mut chrono::DateTime<Utc> {
        &mut self.0
    }
}

impl From<DateTime> for chrono::DateTime<Utc> {
    fn from(utc: DateTime) -> Self {
        utc.0
    }
}

impl From<chrono::DateTime<Utc>> for DateTime {
    fn from(x: chrono::DateTime<Utc>) -> Self {
        DateTime(x)
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

impl Binary {
    fn from_extended_doc(doc: &Document) -> Option<Self> {
        let binary = doc.get_document("$binary").ok()?;
        let bytes = binary.get_str("base64").ok()?;
        let bytes = base64::decode(bytes).ok()?;
        let subtype = binary.get_str("subType").ok()?;
        let subtype = hex::decode(subtype).ok()?;

        if subtype.len() == 1 {
            Some(Self {
                bytes,
                subtype: subtype[0].into(),
            })
        } else {
            None
        }
    }
}

/// Represents a DBPointer. (Deprecated)
#[derive(Debug, Clone, PartialEq)]
pub struct DbPointer {
    pub(crate) namespace: String,
    pub(crate) id: oid::ObjectId,
}
