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
    convert::TryFrom,
    fmt::{self, Debug, Display, Formatter},
    hash::Hash,
    ops::Index,
};

pub use crate::document::Document;
use crate::{oid, raw::CString, spec::ElementType, Binary, Decimal128};

/// Possible BSON value types.
#[derive(Clone, Default, PartialEq)]
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
    #[default]
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
    DateTime(crate::DateTime),
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

impl Hash for Bson {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            Bson::Double(double) => {
                if *double == 0.0_f64 {
                    // There are 2 zero representations, +0 and -0, which
                    // compare equal but have different bits. We use the +0 hash
                    // for both so that hash(+0) == hash(-0).
                    0.0_f64.to_bits().hash(state);
                } else {
                    double.to_bits().hash(state);
                }
            }
            Bson::String(x) => x.hash(state),
            Bson::Array(x) => x.hash(state),
            Bson::Document(x) => x.hash(state),
            Bson::Boolean(x) => x.hash(state),
            Bson::RegularExpression(x) => x.hash(state),
            Bson::JavaScriptCode(x) => x.hash(state),
            Bson::JavaScriptCodeWithScope(x) => x.hash(state),
            Bson::Int32(x) => x.hash(state),
            Bson::Int64(x) => x.hash(state),
            Bson::Timestamp(x) => x.hash(state),
            Bson::Binary(x) => x.hash(state),
            Bson::ObjectId(x) => x.hash(state),
            Bson::DateTime(x) => x.hash(state),
            Bson::Symbol(x) => x.hash(state),
            Bson::Decimal128(x) => x.hash(state),
            Bson::DbPointer(x) => x.hash(state),
            Bson::Null | Bson::Undefined | Bson::MaxKey | Bson::MinKey => (),
        }
    }
}

impl Eq for Bson {}

impl Display for Bson {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Bson::Double(f) => write!(fmt, "{}", f),
            Bson::String(ref s) => write!(fmt, "\"{}\"", s),
            Bson::Array(ref vec) => {
                fmt.write_str("[")?;

                let indent = fmt.width().unwrap_or(2);
                let indent_str = " ".repeat(indent);

                let mut first = true;
                for bson in vec {
                    if !first {
                        fmt.write_str(", ")?;
                    }
                    if fmt.alternate() {
                        write!(fmt, "\n{indent_str}")?;
                        match bson {
                            Bson::Array(_arr) => {
                                let new_indent = indent + 2;
                                write!(fmt, "{bson:#new_indent$}")?;
                            }
                            Bson::Document(ref doc) => {
                                let new_indent = indent + 2;
                                write!(fmt, "{doc:#new_indent$}")?;
                            }
                            _ => {
                                write!(fmt, "{}", bson)?;
                            }
                        }
                    } else {
                        write!(fmt, "{}", bson)?;
                    }
                    first = false;
                }
                if fmt.alternate() && !vec.is_empty() {
                    let closing_bracket_indent_str = " ".repeat(indent - 2);
                    write!(fmt, "\n{closing_bracket_indent_str}]")
                } else {
                    fmt.write_str("]")
                }
            }
            Bson::Document(ref doc) => {
                if fmt.alternate() {
                    write!(fmt, "{doc:#}")
                } else {
                    write!(fmt, "{doc}")
                }
            }
            Bson::Boolean(b) => write!(fmt, "{}", b),
            Bson::Null => write!(fmt, "null"),
            Bson::RegularExpression(ref x) => write!(fmt, "{}", x),
            Bson::JavaScriptCode(ref code)
            | Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope { ref code, .. }) => {
                fmt.write_str(code)
            }
            Bson::Int32(i) => write!(fmt, "{}", i),
            Bson::Int64(i) => write!(fmt, "{}", i),
            Bson::Timestamp(ref x) => write!(fmt, "{}", x),
            Bson::Binary(ref x) => write!(fmt, "{}", x),
            Bson::ObjectId(ref id) => write!(fmt, "ObjectId(\"{}\")", id),
            Bson::DateTime(date_time) => write!(fmt, "DateTime(\"{}\")", date_time),
            Bson::Symbol(ref sym) => write!(fmt, "Symbol(\"{}\")", sym),
            Bson::Decimal128(ref d) => write!(fmt, "{}", d),
            Bson::Undefined => write!(fmt, "undefined"),
            Bson::MinKey => write!(fmt, "MinKey"),
            Bson::MaxKey => write!(fmt, "MaxKey"),
            Bson::DbPointer(DbPointer {
                ref namespace,
                ref id,
            }) => write!(fmt, "DbPointer({}, {})", namespace, id),
        }
    }
}

impl Debug for Bson {
    fn fmt(&self, fmt: &mut Formatter) -> fmt::Result {
        match *self {
            Bson::Double(f) => fmt.debug_tuple("Double").field(&f).finish(),
            Bson::String(ref s) => fmt.debug_tuple("String").field(s).finish(),
            Bson::Array(ref vec) => {
                write!(fmt, "Array(")?;
                Debug::fmt(vec, fmt)?;
                write!(fmt, ")")
            }
            Bson::Document(ref doc) => Debug::fmt(doc, fmt),
            Bson::Boolean(b) => fmt.debug_tuple("Boolean").field(&b).finish(),
            Bson::Null => write!(fmt, "Null"),
            Bson::RegularExpression(ref regex) => Debug::fmt(regex, fmt),
            Bson::JavaScriptCode(ref code) => {
                fmt.debug_tuple("JavaScriptCode").field(code).finish()
            }
            Bson::JavaScriptCodeWithScope(ref code) => Debug::fmt(code, fmt),
            Bson::Int32(i) => fmt.debug_tuple("Int32").field(&i).finish(),
            Bson::Int64(i) => fmt.debug_tuple("Int64").field(&i).finish(),
            Bson::Timestamp(ref t) => Debug::fmt(t, fmt),
            Bson::Binary(ref b) => Debug::fmt(b, fmt),
            Bson::ObjectId(ref id) => Debug::fmt(id, fmt),
            Bson::DateTime(ref date_time) => Debug::fmt(date_time, fmt),
            Bson::Symbol(ref sym) => fmt.debug_tuple("Symbol").field(sym).finish(),
            Bson::Decimal128(ref d) => Debug::fmt(d, fmt),
            Bson::Undefined => write!(fmt, "Undefined"),
            Bson::MinKey => write!(fmt, "MinKey"),
            Bson::MaxKey => write!(fmt, "MaxKey"),
            Bson::DbPointer(ref pointer) => Debug::fmt(pointer, fmt),
        }
    }
}

impl Index<&str> for Bson {
    type Output = Bson;

    fn index(&self, index: &str) -> &Self::Output {
        match *self {
            Bson::Document(ref doc) => match doc.get(index) {
                Some(v) => v,
                None => &Bson::Null,
            },
            _ => &Bson::Null,
        }
    }
}

impl From<f32> for Bson {
    fn from(a: f32) -> Bson {
        Bson::Double(a.into())
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

impl From<crate::raw::CString> for Bson {
    fn from(a: crate::raw::CString) -> Bson {
        Bson::String(a.into_string())
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

impl<T> From<&mut T> for Bson
where
    for<'a> &'a T: Into<Bson>,
{
    fn from(t: &mut T) -> Bson {
        (&*t).into()
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
        if let Ok(i) = i32::try_from(a) {
            Bson::Int32(i)
        } else {
            Bson::Int64(a.into())
        }
    }
}

impl From<[u8; 12]> for Bson {
    fn from(a: [u8; 12]) -> Bson {
        Bson::ObjectId(oid::ObjectId::from_bytes(a))
    }
}

impl From<oid::ObjectId> for Bson {
    fn from(a: oid::ObjectId) -> Bson {
        Bson::ObjectId(a)
    }
}

#[cfg(feature = "time-0_3")]
impl From<time::OffsetDateTime> for Bson {
    fn from(a: time::OffsetDateTime) -> Bson {
        Bson::DateTime(crate::DateTime::from(a))
    }
}

#[cfg(feature = "chrono-0_4")]
impl<T: chrono::TimeZone> From<chrono::DateTime<T>> for Bson {
    fn from(a: chrono::DateTime<T>) -> Bson {
        Bson::DateTime(crate::DateTime::from(a))
    }
}

#[cfg(feature = "jiff-0_2")]
impl From<jiff::Timestamp> for Bson {
    fn from(a: jiff::Timestamp) -> Bson {
        Bson::DateTime(crate::DateTime::from(a))
    }
}

#[cfg(feature = "uuid-1")]
impl From<uuid::Uuid> for Bson {
    fn from(uuid: uuid::Uuid) -> Self {
        Bson::Binary(uuid.into())
    }
}

impl From<crate::DateTime> for Bson {
    fn from(dt: crate::DateTime) -> Self {
        Bson::DateTime(dt)
    }
}

impl From<DbPointer> for Bson {
    fn from(a: DbPointer) -> Bson {
        Bson::DbPointer(a)
    }
}

impl From<Decimal128> for Bson {
    fn from(d: Decimal128) -> Self {
        Bson::Decimal128(d)
    }
}

impl<T> From<Option<T>> for Bson
where
    T: Into<Bson>,
{
    fn from(a: Option<T>) -> Bson {
        match a {
            None => Bson::Null,
            Some(t) => t.into(),
        }
    }
}

impl Bson {
    /// Get the [`ElementType`] of this value.
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
    /// This function mainly used for [extended JSON format](https://www.mongodb.com/docs/manual/reference/mongodb-extended-json/).
    // TODO RUST-426: Investigate either removing this from the serde implementation or unifying
    // with the extended JSON implementation.
    #[cfg(feature = "serde")]
    pub(crate) fn into_extended_document(self, rawbson: bool) -> Document {
        match self {
            Bson::RegularExpression(Regex {
                ref pattern,
                ref options,
            }) => {
                let mut chars: Vec<_> = options.as_str().chars().collect();
                chars.sort_unstable();

                let options: String = chars.into_iter().collect();

                doc! {
                    "$regularExpression": {
                        "pattern": pattern,
                        "options": options,
                    }
                }
            }
            Bson::JavaScriptCode(ref code) => {
                doc! {
                    "$code": code,
                }
            }
            Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope { code, scope }) => {
                doc! {
                    "$code": code,
                    "$scope": scope,
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
            Bson::Binary(Binary { subtype, bytes }) => {
                let tval: u8 = From::from(subtype);
                if rawbson {
                    doc! {
                        "$binary": {
                            "bytes": Binary { subtype: crate::spec::BinarySubtype::Generic, bytes },
                            "subType": Bson::Int32(tval.into())
                        }
                    }
                } else {
                    doc! {
                        "$binary": {
                            "base64": crate::base64::encode(bytes),
                            "subType": hex::encode([tval]),
                        }
                    }
                }
            }
            Bson::ObjectId(ref v) => {
                doc! {
                    "$oid": v.to_string(),
                }
            }
            Bson::DateTime(v) if rawbson => doc! {
                "$date": v.timestamp_millis(),
            },
            Bson::DateTime(v) if v.timestamp_millis() >= 0 && v.to_time_0_3().year() <= 9999 => {
                doc! {
                    // Unwrap safety: timestamps in the guarded range can always be formatted.
                    "$date": v.try_to_rfc3339_string().unwrap(),
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

    #[cfg(feature = "serde")]
    pub(crate) fn from_extended_document(doc: Document) -> Bson {
        if doc.len() > 2 {
            return Bson::Document(doc);
        }

        let mut keys: Vec<_> = doc.keys().map(|s| s.as_str()).collect();
        keys.sort_unstable();

        match keys.as_slice() {
            ["$oid"] => {
                if let Ok(oid) = doc.get_str("$oid") {
                    if let Ok(oid) = crate::oid::ObjectId::parse_str(oid) {
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

            ["$numberDecimal"] => {
                if let Ok(d) = doc.get_str("$numberDecimal") {
                    if let Ok(d) = d.parse() {
                        return Bson::Decimal128(d);
                    }
                }
            }

            ["$numberDecimalBytes"] => {
                if let Ok(bytes) = doc.get_binary_generic("$numberDecimalBytes") {
                    if let Ok(b) = bytes.clone().try_into() {
                        return Bson::Decimal128(Decimal128 { bytes: b });
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
                            if let Ok(regex) = Regex::from_strings(pattern, options) {
                                return Bson::RegularExpression(regex);
                            }
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
                                id,
                            });
                        }
                    }
                }
            }

            ["$date"] => {
                if let Ok(date) = doc.get_i64("$date") {
                    return Bson::DateTime(crate::DateTime::from_millis(date));
                }

                if let Ok(date) = doc.get_str("$date") {
                    if let Ok(dt) = crate::DateTime::parse_rfc3339_str(date) {
                        return Bson::DateTime(dt);
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

    /// Method for converting a given [`Bson`] value to a [`serde::de::Unexpected`] for error
    /// reporting.
    #[cfg(feature = "serde")]
    pub(crate) fn as_unexpected(&self) -> serde::de::Unexpected {
        use serde::de::Unexpected;
        match self {
            Bson::Array(_) => Unexpected::Seq,
            Bson::Binary(b) => Unexpected::Bytes(b.bytes.as_slice()),
            Bson::Boolean(b) => Unexpected::Bool(*b),
            Bson::DbPointer(_) => Unexpected::Other("dbpointer"),
            Bson::Document(_) => Unexpected::Map,
            Bson::Double(f) => Unexpected::Float(*f),
            Bson::Int32(i) => Unexpected::Signed(*i as i64),
            Bson::Int64(i) => Unexpected::Signed(*i),
            Bson::JavaScriptCode(_) => Unexpected::Other("javascript code"),
            Bson::JavaScriptCodeWithScope(_) => Unexpected::Other("javascript code with scope"),
            Bson::MaxKey => Unexpected::Other("maxkey"),
            Bson::MinKey => Unexpected::Other("minkey"),
            Bson::Null => Unexpected::Unit,
            Bson::Undefined => Unexpected::Other("undefined"),
            Bson::ObjectId(_) => Unexpected::Other("objectid"),
            Bson::RegularExpression(_) => Unexpected::Other("regex"),
            Bson::String(s) => Unexpected::Str(s.as_str()),
            Bson::Symbol(_) => Unexpected::Other("symbol"),
            Bson::Timestamp(_) => Unexpected::Other("timestamp"),
            Bson::DateTime(_) => Unexpected::Other("datetime"),
            Bson::Decimal128(_) => Unexpected::Other("decimal128"),
        }
    }
}

/// Value helpers
impl Bson {
    /// If `self` is [`Double`](Bson::Double), return its value as an `f64`. Returns [`None`]
    /// otherwise.
    pub fn as_f64(&self) -> Option<f64> {
        match *self {
            Bson::Double(v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`String`](Bson::String), return its value as a `&str`. Returns [`None`]
    /// otherwise.
    pub fn as_str(&self) -> Option<&str> {
        match *self {
            Bson::String(ref s) => Some(s),
            _ => None,
        }
    }

    /// If `self` is [`String`](Bson::String), return a mutable reference to its value as a [`str`].
    /// Returns [`None`] otherwise.
    pub fn as_str_mut(&mut self) -> Option<&mut str> {
        match *self {
            Bson::String(ref mut s) => Some(s),
            _ => None,
        }
    }

    /// If `self` is [`Array`](Bson::Array), return its value. Returns [`None`] otherwise.
    pub fn as_array(&self) -> Option<&Array> {
        match *self {
            Bson::Array(ref v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`Array`](Bson::Array), return a mutable reference to its value. Returns
    /// [`None`] otherwise.
    pub fn as_array_mut(&mut self) -> Option<&mut Array> {
        match *self {
            Bson::Array(ref mut v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`Document`](Bson::Document), return its value. Returns [`None`] otherwise.
    pub fn as_document(&self) -> Option<&Document> {
        match *self {
            Bson::Document(ref v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`Document`](Bson::Document), return a mutable reference to its value. Returns
    /// [`None`] otherwise.
    pub fn as_document_mut(&mut self) -> Option<&mut Document> {
        match *self {
            Bson::Document(ref mut v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`Boolean`](Bson::Boolean), return its value. Returns [`None`] otherwise.
    pub fn as_bool(&self) -> Option<bool> {
        match *self {
            Bson::Boolean(v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`Int32`](Bson::Int32), return its value. Returns [`None`] otherwise.
    pub fn as_i32(&self) -> Option<i32> {
        match *self {
            Bson::Int32(v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`Int64`](Bson::Int64), return its value. Returns [`None`] otherwise.
    pub fn as_i64(&self) -> Option<i64> {
        match *self {
            Bson::Int64(v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`ObjectId`](Bson::ObjectId), return its value. Returns [`None`] otherwise.
    pub fn as_object_id(&self) -> Option<oid::ObjectId> {
        match *self {
            Bson::ObjectId(v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`ObjectId`](Bson::ObjectId), return a mutable reference to its value. Returns
    /// [`None`] otherwise.
    pub fn as_object_id_mut(&mut self) -> Option<&mut oid::ObjectId> {
        match *self {
            Bson::ObjectId(ref mut v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`DateTime`](Bson::DateTime), return its value. Returns [`None`] otherwise.
    pub fn as_datetime(&self) -> Option<&crate::DateTime> {
        match *self {
            Bson::DateTime(ref v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`DateTime`](Bson::DateTime), return a mutable reference to its value. Returns
    /// [`None`] otherwise.
    pub fn as_datetime_mut(&mut self) -> Option<&mut crate::DateTime> {
        match *self {
            Bson::DateTime(ref mut v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`Symbol`](Bson::Symbol), return its value. Returns [`None`] otherwise.
    pub fn as_symbol(&self) -> Option<&str> {
        match *self {
            Bson::Symbol(ref v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`Symbol`](Bson::Symbol), return a mutable reference to its value. Returns
    /// [`None`] otherwise.
    pub fn as_symbol_mut(&mut self) -> Option<&mut str> {
        match *self {
            Bson::Symbol(ref mut v) => Some(v),
            _ => None,
        }
    }

    /// If `self` is [`Timestamp`](Bson::Timestamp), return its value. Returns [`None`] otherwise.
    pub fn as_timestamp(&self) -> Option<Timestamp> {
        match *self {
            Bson::Timestamp(timestamp) => Some(timestamp),
            _ => None,
        }
    }

    /// If `self` is [`Null`](Bson::Null), return `()`. Returns [`None`] otherwise.
    pub fn as_null(&self) -> Option<()> {
        match *self {
            Bson::Null => Some(()),
            _ => None,
        }
    }

    /// If `self` is [`DbPointer`](Bson::DbPointer), return its value.  Returns [`None`] otherwise.
    pub fn as_db_pointer(&self) -> Option<&DbPointer> {
        match self {
            Bson::DbPointer(ref db_pointer) => Some(db_pointer),
            _ => None,
        }
    }
}

/// Represents a BSON timestamp value.
#[derive(Debug, Eq, Ord, PartialEq, PartialOrd, Clone, Copy, Hash)]
pub struct Timestamp {
    /// The number of seconds since the Unix epoch.
    pub time: u32,

    /// An incrementing value to order timestamps with the same number of seconds in the `time`
    /// field.
    pub increment: u32,
}

impl Display for Timestamp {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "Timestamp({}, {})", self.time, self.increment)
    }
}

impl Timestamp {
    pub(crate) fn to_le_bytes(self) -> [u8; 8] {
        let mut out = [0; 8];
        out[0..4].copy_from_slice(&self.increment.to_le_bytes());
        out[4..8].copy_from_slice(&self.time.to_le_bytes());
        out
    }

    pub(crate) fn from_le_bytes(bytes: [u8; 8]) -> Self {
        let mut inc_bytes = [0; 4];
        inc_bytes.copy_from_slice(&bytes[0..4]);
        let mut time_bytes = [0; 4];
        time_bytes.copy_from_slice(&bytes[4..8]);
        Self {
            increment: u32::from_le_bytes(inc_bytes),
            time: u32::from_le_bytes(time_bytes),
        }
    }
}

/// Represents a BSON regular expression value.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Regex {
    /// The regex pattern to match.
    pub pattern: CString,

    /// The options for the regex.
    ///
    /// Options are identified by characters, which must be stored in
    /// alphabetical order. Valid options are 'i' for case insensitive matching, 'm' for
    /// multiline matching, 'x' for verbose mode, 'l' to make \w, \W, etc. locale dependent,
    /// 's' for dotall mode ('.' matches everything), and 'u' to make \w, \W, etc. match
    /// unicode.
    pub options: CString,
}

impl Regex {
    #[cfg(any(test, feature = "serde"))]
    pub(crate) fn from_strings(
        pattern: impl AsRef<str>,
        options: impl AsRef<str>,
    ) -> crate::error::Result<Self> {
        let mut chars: Vec<_> = options.as_ref().chars().collect();
        chars.sort_unstable();
        let options: String = chars.into_iter().collect();
        Ok(Self {
            pattern: pattern.as_ref().to_string().try_into()?,
            options: options.try_into()?,
        })
    }
}

impl Display for Regex {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "/{}/{}", self.pattern, self.options)
    }
}

/// Represents a BSON code with scope value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JavaScriptCodeWithScope {
    /// The JavaScript code.
    pub code: String,

    /// The scope document containing variable bindings.
    pub scope: Document,
}

impl Display for JavaScriptCodeWithScope {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.write_str(&self.code)
    }
}

/// Represents a DBPointer. (Deprecated)
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DbPointer {
    pub(crate) namespace: String,
    pub(crate) id: oid::ObjectId,
}
