use std::convert::{TryFrom, TryInto};

use serde::{Deserialize, Serialize};

use crate::{
    oid::{self, ObjectId},
    raw::RAW_BSON_NEWTYPE,
    spec::ElementType,
    Binary,
    Bson,
    DbPointer,
    Decimal128,
    RawArray,
    RawArrayBuf,
    RawBinary,
    RawBson,
    RawDbPointer,
    RawDocument,
    RawDocumentBuf,
    RawJavaScriptCodeWithScope,
    RawRegex,
    Regex,
    Timestamp,
};

use super::{
    serde::{OwnedOrBorrowedRawBson, OwnedOrBorrowedRawBsonVisitor},
    Error,
    Result,
};

/// A BSON value backed by owned raw BSON bytes.
#[derive(Debug, Clone, PartialEq)]
pub enum OwnedRawBson {
    /// 64-bit binary floating point
    Double(f64),
    /// UTF-8 string
    String(String),
    /// Array
    Array(RawArrayBuf),
    /// Embedded document
    Document(RawDocumentBuf),
    /// Boolean value
    Boolean(bool),
    /// Null value
    Null,
    /// Regular expression
    RegularExpression(Regex),
    /// JavaScript code
    JavaScriptCode(String),
    /// JavaScript code w/ scope
    JavaScriptCodeWithScope(OwnedRawJavaScriptCodeWithScope),
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

impl OwnedRawBson {
    /// Get the [`ElementType`] of this value.
    pub fn element_type(&self) -> ElementType {
        match *self {
            OwnedRawBson::Double(..) => ElementType::Double,
            OwnedRawBson::String(..) => ElementType::String,
            OwnedRawBson::Array(..) => ElementType::Array,
            OwnedRawBson::Document(..) => ElementType::EmbeddedDocument,
            OwnedRawBson::Boolean(..) => ElementType::Boolean,
            OwnedRawBson::Null => ElementType::Null,
            OwnedRawBson::RegularExpression(..) => ElementType::RegularExpression,
            OwnedRawBson::JavaScriptCode(..) => ElementType::JavaScriptCode,
            OwnedRawBson::JavaScriptCodeWithScope(..) => ElementType::JavaScriptCodeWithScope,
            OwnedRawBson::Int32(..) => ElementType::Int32,
            OwnedRawBson::Int64(..) => ElementType::Int64,
            OwnedRawBson::Timestamp(..) => ElementType::Timestamp,
            OwnedRawBson::Binary(..) => ElementType::Binary,
            OwnedRawBson::ObjectId(..) => ElementType::ObjectId,
            OwnedRawBson::DateTime(..) => ElementType::DateTime,
            OwnedRawBson::Symbol(..) => ElementType::Symbol,
            OwnedRawBson::Decimal128(..) => ElementType::Decimal128,
            OwnedRawBson::Undefined => ElementType::Undefined,
            OwnedRawBson::MaxKey => ElementType::MaxKey,
            OwnedRawBson::MinKey => ElementType::MinKey,
            OwnedRawBson::DbPointer(..) => ElementType::DbPointer,
        }
    }

    /// Gets the wrapped `f64` value or returns `None` if the value isn't a BSON
    /// double.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            OwnedRawBson::Double(d) => Some(*d),
            _ => None,
        }
    }

    /// Gets a reference to the `String` that's wrapped or returns `None` if the wrapped value isn't
    /// a BSON String.
    pub fn as_str(&self) -> Option<&'_ str> {
        match self {
            OwnedRawBson::String(s) => Some(s),
            _ => None,
        }
    }

    /// Gets a reference to the [`RawArrayBuf`] that's wrapped or returns `None` if the wrapped
    /// value isn't a BSON array.
    pub fn as_array(&self) -> Option<&'_ RawArray> {
        match self {
            OwnedRawBson::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Gets a reference to the [`RawDocumentBuf`] that's wrapped or returns `None` if the wrapped
    /// value isn't a BSON document.
    pub fn as_document(&self) -> Option<&'_ RawDocument> {
        match self {
            OwnedRawBson::Document(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the wrapped `bool` value or returns `None` if the wrapped value isn't a BSON
    /// boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            OwnedRawBson::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the wrapped `i32` value or returns `None` if the wrapped value isn't a BSON
    /// Int32.
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            OwnedRawBson::Int32(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the wrapped `i64` value or returns `None` if the wrapped value isn't a BSON
    /// Int64.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            OwnedRawBson::Int64(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the wrapped [`crate::oid::ObjectId`] value or returns `None` if the wrapped value isn't
    /// a BSON ObjectID.
    pub fn as_object_id(&self) -> Option<oid::ObjectId> {
        match self {
            OwnedRawBson::ObjectId(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets a reference to the [`Binary`] that's wrapped or returns `None` if the wrapped value
    /// isn't a BSON binary.
    pub fn as_binary(&self) -> Option<RawBinary<'_>> {
        match self {
            OwnedRawBson::Binary(v) => Some(RawBinary {
                bytes: v.bytes.as_slice(),
                subtype: v.subtype,
            }),
            _ => None,
        }
    }

    /// Gets a reference to the [`Regex`] that's wrapped or returns `None` if the wrapped value
    /// isn't a BSON regular expression.
    pub fn as_regex(&self) -> Option<RawRegex<'_>> {
        match self {
            OwnedRawBson::RegularExpression(v) => Some(RawRegex {
                pattern: v.pattern.as_str(),
                options: v.options.as_str(),
            }),
            _ => None,
        }
    }

    /// Gets the wrapped [`crate::DateTime`] value or returns `None` if the wrapped value isn't a
    /// BSON datetime.
    pub fn as_datetime(&self) -> Option<crate::DateTime> {
        match self {
            OwnedRawBson::DateTime(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets a reference to the symbol that's wrapped or returns `None` if the wrapped value isn't a
    /// BSON Symbol.
    pub fn as_symbol(&self) -> Option<&'_ str> {
        match self {
            OwnedRawBson::Symbol(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the wrapped [`crate::Timestamp`] value or returns `None` if the wrapped value isn't a
    /// BSON datetime.
    pub fn as_timestamp(&self) -> Option<Timestamp> {
        match self {
            OwnedRawBson::Timestamp(timestamp) => Some(*timestamp),
            _ => None,
        }
    }

    /// Returns `Some(())` if this value is null, otherwise returns `None`.
    pub fn as_null(&self) -> Option<()> {
        match self {
            OwnedRawBson::Null => Some(()),
            _ => None,
        }
    }

    /// Gets a reference to the [`crate::DbPointer`] that's wrapped or returns `None` if the wrapped
    /// value isn't a BSON DbPointer.
    pub fn as_db_pointer(&self) -> Option<RawDbPointer<'_>> {
        match self {
            OwnedRawBson::DbPointer(d) => Some(RawDbPointer {
                namespace: d.namespace.as_str(),
                id: d.id,
            }),
            _ => None,
        }
    }

    /// Gets a reference to the code that's wrapped or returns `None` if the wrapped value isn't a
    /// BSON JavaScript code.
    pub fn as_javascript(&self) -> Option<&'_ str> {
        match self {
            OwnedRawBson::JavaScriptCode(s) => Some(s),
            _ => None,
        }
    }

    /// Gets a reference to the [`OwnedRawJavaScriptCodeWithScope`] that's wrapped or returns `None`
    /// if the wrapped value isn't a BSON JavaScript code with scope value.
    pub fn as_javascript_with_scope(&self) -> Option<RawJavaScriptCodeWithScope<'_>> {
        match self {
            OwnedRawBson::JavaScriptCodeWithScope(s) => Some(RawJavaScriptCodeWithScope {
                code: s.code.as_str(),
                scope: &s.scope,
            }),
            _ => None,
        }
    }

    /// Gets a [`RawBson`] value referencing this owned raw BSON value.
    pub fn as_raw_bson(&self) -> RawBson<'_> {
        match self {
            OwnedRawBson::Double(d) => RawBson::Double(*d),
            OwnedRawBson::String(s) => RawBson::String(s.as_str()),
            OwnedRawBson::Array(a) => RawBson::Array(a),
            OwnedRawBson::Document(d) => RawBson::Document(d),
            OwnedRawBson::Boolean(b) => RawBson::Boolean(*b),
            OwnedRawBson::Null => RawBson::Null,
            OwnedRawBson::RegularExpression(re) => RawBson::RegularExpression(RawRegex {
                options: re.options.as_str(),
                pattern: re.pattern.as_str(),
            }),
            OwnedRawBson::JavaScriptCode(c) => RawBson::JavaScriptCode(c.as_str()),
            OwnedRawBson::JavaScriptCodeWithScope(code_w_scope) => {
                RawBson::JavaScriptCodeWithScope(RawJavaScriptCodeWithScope {
                    code: code_w_scope.code.as_str(),
                    scope: code_w_scope.scope.as_ref(),
                })
            }
            OwnedRawBson::Int32(i) => RawBson::Int32(*i),
            OwnedRawBson::Int64(i) => RawBson::Int64(*i),
            OwnedRawBson::Timestamp(ts) => RawBson::Timestamp(*ts),
            OwnedRawBson::Binary(b) => RawBson::Binary(RawBinary {
                bytes: b.bytes.as_slice(),
                subtype: b.subtype,
            }),
            OwnedRawBson::ObjectId(oid) => RawBson::ObjectId(*oid),
            OwnedRawBson::DateTime(dt) => RawBson::DateTime(*dt),
            OwnedRawBson::Symbol(s) => RawBson::Symbol(s.as_str()),
            OwnedRawBson::Decimal128(d) => RawBson::Decimal128(*d),
            OwnedRawBson::Undefined => RawBson::Undefined,
            OwnedRawBson::MaxKey => RawBson::MaxKey,
            OwnedRawBson::MinKey => RawBson::MinKey,
            OwnedRawBson::DbPointer(dbp) => RawBson::DbPointer(RawDbPointer {
                namespace: dbp.namespace.as_str(),
                id: dbp.id,
            }),
        }
    }
}

impl From<i32> for OwnedRawBson {
    fn from(i: i32) -> Self {
        OwnedRawBson::Int32(i)
    }
}

impl From<i64> for OwnedRawBson {
    fn from(i: i64) -> Self {
        OwnedRawBson::Int64(i)
    }
}

impl From<String> for OwnedRawBson {
    fn from(s: String) -> Self {
        OwnedRawBson::String(s)
    }
}

impl From<&str> for OwnedRawBson {
    fn from(s: &str) -> Self {
        OwnedRawBson::String(s.to_owned())
    }
}

impl From<f64> for OwnedRawBson {
    fn from(f: f64) -> Self {
        OwnedRawBson::Double(f)
    }
}

impl From<bool> for OwnedRawBson {
    fn from(b: bool) -> Self {
        OwnedRawBson::Boolean(b)
    }
}

impl From<RawDocumentBuf> for OwnedRawBson {
    fn from(d: RawDocumentBuf) -> Self {
        OwnedRawBson::Document(d)
    }
}

impl From<RawArrayBuf> for OwnedRawBson {
    fn from(a: RawArrayBuf) -> Self {
        OwnedRawBson::Array(a)
    }
}

impl From<crate::DateTime> for OwnedRawBson {
    fn from(dt: crate::DateTime) -> Self {
        OwnedRawBson::DateTime(dt)
    }
}

impl From<Timestamp> for OwnedRawBson {
    fn from(ts: Timestamp) -> Self {
        OwnedRawBson::Timestamp(ts)
    }
}

impl From<ObjectId> for OwnedRawBson {
    fn from(oid: ObjectId) -> Self {
        OwnedRawBson::ObjectId(oid)
    }
}

impl From<Decimal128> for OwnedRawBson {
    fn from(d: Decimal128) -> Self {
        OwnedRawBson::Decimal128(d)
    }
}

impl From<OwnedRawJavaScriptCodeWithScope> for OwnedRawBson {
    fn from(code_w_scope: OwnedRawJavaScriptCodeWithScope) -> Self {
        OwnedRawBson::JavaScriptCodeWithScope(code_w_scope)
    }
}

impl From<Binary> for OwnedRawBson {
    fn from(b: Binary) -> Self {
        OwnedRawBson::Binary(b)
    }
}

impl From<Regex> for OwnedRawBson {
    fn from(re: Regex) -> Self {
        OwnedRawBson::RegularExpression(re)
    }
}

impl From<DbPointer> for OwnedRawBson {
    fn from(d: DbPointer) -> Self {
        OwnedRawBson::DbPointer(d)
    }
}

impl<'de> Deserialize<'de> for OwnedRawBson {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match deserializer
            .deserialize_newtype_struct(RAW_BSON_NEWTYPE, OwnedOrBorrowedRawBsonVisitor)?
        {
            OwnedOrBorrowedRawBson::Owned(o) => Ok(o),
            OwnedOrBorrowedRawBson::Borrowed(b) => Ok(b.to_owned_raw_bson()),
        }
    }
}

impl Serialize for OwnedRawBson {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_raw_bson().serialize(serializer)
    }
}

impl<'a> TryFrom<OwnedRawBson> for Bson {
    type Error = Error;

    fn try_from(rawbson: OwnedRawBson) -> Result<Bson> {
        Ok(match rawbson {
            OwnedRawBson::Double(d) => Bson::Double(d),
            OwnedRawBson::String(s) => Bson::String(s),
            OwnedRawBson::Document(rawdoc) => Bson::Document(rawdoc.as_ref().try_into()?),
            OwnedRawBson::Array(rawarray) => {
                let mut items = Vec::new();
                for v in rawarray.into_iter() {
                    let bson: Bson = v?.try_into()?;
                    items.push(bson);
                }
                Bson::Array(items)
            }
            OwnedRawBson::Binary(rawbson) => Bson::Binary(rawbson),
            OwnedRawBson::ObjectId(rawbson) => Bson::ObjectId(rawbson),
            OwnedRawBson::Boolean(rawbson) => Bson::Boolean(rawbson),
            OwnedRawBson::DateTime(rawbson) => Bson::DateTime(rawbson),
            OwnedRawBson::Null => Bson::Null,
            OwnedRawBson::RegularExpression(rawregex) => {
                Bson::RegularExpression(crate::Regex::new(rawregex.pattern, rawregex.options))
            }
            OwnedRawBson::JavaScriptCode(rawbson) => Bson::JavaScriptCode(rawbson),
            OwnedRawBson::Int32(rawbson) => Bson::Int32(rawbson),
            OwnedRawBson::Timestamp(rawbson) => Bson::Timestamp(rawbson),
            OwnedRawBson::Int64(rawbson) => Bson::Int64(rawbson),
            OwnedRawBson::Undefined => Bson::Undefined,
            OwnedRawBson::DbPointer(rawbson) => Bson::DbPointer(DbPointer {
                namespace: rawbson.namespace,
                id: rawbson.id,
            }),
            OwnedRawBson::Symbol(rawbson) => Bson::Symbol(rawbson),
            OwnedRawBson::JavaScriptCodeWithScope(rawbson) => {
                Bson::JavaScriptCodeWithScope(crate::JavaScriptCodeWithScope {
                    code: rawbson.code,
                    scope: rawbson.scope.try_into()?,
                })
            }
            OwnedRawBson::Decimal128(rawbson) => Bson::Decimal128(rawbson),
            OwnedRawBson::MaxKey => Bson::MaxKey,
            OwnedRawBson::MinKey => Bson::MinKey,
        })
    }
}

/// A BSON "code with scope" value backed by owned raw BSON.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnedRawJavaScriptCodeWithScope {
    /// The code value.
    pub code: String,

    /// The scope document.
    pub scope: RawDocumentBuf,
}

impl<'de> Deserialize<'de> for OwnedRawJavaScriptCodeWithScope {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match OwnedRawBson::deserialize(deserializer)? {
            OwnedRawBson::JavaScriptCodeWithScope(b) => Ok(b),
            c => Err(serde::de::Error::custom(format!(
                "expected CodeWithScope, but got {:?} instead",
                c
            ))),
        }
    }
}

impl Serialize for OwnedRawJavaScriptCodeWithScope {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let raw = RawJavaScriptCodeWithScope {
            code: self.code.as_str(),
            scope: self.scope.as_ref(),
        };

        raw.serialize(serializer)
    }
}
