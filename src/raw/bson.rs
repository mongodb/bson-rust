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
    RawBinaryRef,
    RawBsonRef,
    RawDbPointerRef,
    RawDocument,
    RawDocumentBuf,
    RawJavaScriptCodeWithScopeRef,
    RawRegexRef,
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
pub enum RawBson {
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
    JavaScriptCodeWithScope(RawJavaScriptCodeWithScope),
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

impl RawBson {
    /// Get the [`ElementType`] of this value.
    pub fn element_type(&self) -> ElementType {
        match *self {
            RawBson::Double(..) => ElementType::Double,
            RawBson::String(..) => ElementType::String,
            RawBson::Array(..) => ElementType::Array,
            RawBson::Document(..) => ElementType::EmbeddedDocument,
            RawBson::Boolean(..) => ElementType::Boolean,
            RawBson::Null => ElementType::Null,
            RawBson::RegularExpression(..) => ElementType::RegularExpression,
            RawBson::JavaScriptCode(..) => ElementType::JavaScriptCode,
            RawBson::JavaScriptCodeWithScope(..) => ElementType::JavaScriptCodeWithScope,
            RawBson::Int32(..) => ElementType::Int32,
            RawBson::Int64(..) => ElementType::Int64,
            RawBson::Timestamp(..) => ElementType::Timestamp,
            RawBson::Binary(..) => ElementType::Binary,
            RawBson::ObjectId(..) => ElementType::ObjectId,
            RawBson::DateTime(..) => ElementType::DateTime,
            RawBson::Symbol(..) => ElementType::Symbol,
            RawBson::Decimal128(..) => ElementType::Decimal128,
            RawBson::Undefined => ElementType::Undefined,
            RawBson::MaxKey => ElementType::MaxKey,
            RawBson::MinKey => ElementType::MinKey,
            RawBson::DbPointer(..) => ElementType::DbPointer,
        }
    }

    /// Gets the wrapped `f64` value or returns [`None`] if the value isn't a BSON
    /// double.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            RawBson::Double(d) => Some(*d),
            _ => None,
        }
    }

    /// Gets a reference to the [`String`] that's wrapped or returns [`None`] if the wrapped value
    /// isn't a BSON String.
    pub fn as_str(&self) -> Option<&'_ str> {
        match self {
            RawBson::String(s) => Some(s),
            _ => None,
        }
    }

    /// Gets a reference to the [`RawArrayBuf`] that's wrapped or returns [`None`] if the wrapped
    /// value isn't a BSON array.
    pub fn as_array(&self) -> Option<&'_ RawArray> {
        match self {
            RawBson::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Gets a mutable reference to the [`RawArrayBuf`] that's wrapped or returns [`None`] if the
    /// wrapped value isn't a BSON array.
    pub fn as_array_mut(&mut self) -> Option<&mut RawArrayBuf> {
        match self {
            RawBson::Array(ref mut v) => Some(v),
            _ => None,
        }
    }

    /// Gets a reference to the [`RawDocumentBuf`] that's wrapped or returns [`None`] if the wrapped
    /// value isn't a BSON document.
    pub fn as_document(&self) -> Option<&'_ RawDocument> {
        match self {
            RawBson::Document(v) => Some(v),
            _ => None,
        }
    }

    /// Gets a mutable reference to the [`RawDocumentBuf`] that's wrapped or returns [`None`] if the
    /// wrapped value isn't a BSON document.
    pub fn as_document_mut(&mut self) -> Option<&mut RawDocumentBuf> {
        match self {
            RawBson::Document(ref mut v) => Some(v),
            _ => None,
        }
    }

    /// Gets the wrapped `bool` value or returns [`None`] if the wrapped value isn't a BSON
    /// boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            RawBson::Boolean(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the wrapped `i32` value or returns [`None`] if the wrapped value isn't a BSON
    /// Int32.
    pub fn as_i32(&self) -> Option<i32> {
        match self {
            RawBson::Int32(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the wrapped `i64` value or returns [`None`] if the wrapped value isn't a BSON
    /// Int64.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            RawBson::Int64(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets the wrapped [`crate::oid::ObjectId`] value or returns [`None`] if the wrapped value
    /// isn't a BSON ObjectID.
    pub fn as_object_id(&self) -> Option<oid::ObjectId> {
        match self {
            RawBson::ObjectId(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets a reference to the [`Binary`] that's wrapped or returns [`None`] if the wrapped value
    /// isn't a BSON binary.
    pub fn as_binary(&self) -> Option<RawBinaryRef<'_>> {
        match self {
            RawBson::Binary(v) => Some(RawBinaryRef {
                bytes: v.bytes.as_slice(),
                subtype: v.subtype,
            }),
            _ => None,
        }
    }

    /// Gets a reference to the [`Regex`] that's wrapped or returns [`None`] if the wrapped value
    /// isn't a BSON regular expression.
    pub fn as_regex(&self) -> Option<RawRegexRef<'_>> {
        match self {
            RawBson::RegularExpression(v) => Some(RawRegexRef {
                pattern: v.pattern.as_str(),
                options: v.options.as_str(),
            }),
            _ => None,
        }
    }

    /// Gets the wrapped [`crate::DateTime`] value or returns [`None`] if the wrapped value isn't a
    /// BSON datetime.
    pub fn as_datetime(&self) -> Option<crate::DateTime> {
        match self {
            RawBson::DateTime(v) => Some(*v),
            _ => None,
        }
    }

    /// Gets a reference to the symbol that's wrapped or returns [`None`] if the wrapped value isn't
    /// a BSON Symbol.
    pub fn as_symbol(&self) -> Option<&'_ str> {
        match self {
            RawBson::Symbol(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the wrapped [`crate::Timestamp`] value or returns [`None`] if the wrapped value isn't a
    /// BSON datetime.
    pub fn as_timestamp(&self) -> Option<Timestamp> {
        match self {
            RawBson::Timestamp(timestamp) => Some(*timestamp),
            _ => None,
        }
    }

    /// Returns `Some(())` if this value is null, otherwise returns [`None`].
    pub fn as_null(&self) -> Option<()> {
        match self {
            RawBson::Null => Some(()),
            _ => None,
        }
    }

    /// Gets a reference to the [`crate::DbPointer`] that's wrapped or returns [`None`] if the
    /// wrapped value isn't a BSON DbPointer.
    pub fn as_db_pointer(&self) -> Option<RawDbPointerRef<'_>> {
        match self {
            RawBson::DbPointer(d) => Some(RawDbPointerRef {
                namespace: d.namespace.as_str(),
                id: d.id,
            }),
            _ => None,
        }
    }

    /// Gets a reference to the code that's wrapped or returns [`None`] if the wrapped value isn't a
    /// BSON JavaScript code.
    pub fn as_javascript(&self) -> Option<&'_ str> {
        match self {
            RawBson::JavaScriptCode(s) => Some(s),
            _ => None,
        }
    }

    /// Gets a reference to the [`RawJavaScriptCodeWithScope`] that's wrapped or returns [`None`]
    /// if the wrapped value isn't a BSON JavaScript code with scope value.
    pub fn as_javascript_with_scope(&self) -> Option<RawJavaScriptCodeWithScopeRef<'_>> {
        match self {
            RawBson::JavaScriptCodeWithScope(s) => Some(RawJavaScriptCodeWithScopeRef {
                code: s.code.as_str(),
                scope: &s.scope,
            }),
            _ => None,
        }
    }

    /// Gets a [`RawBsonRef`] value referencing this owned raw BSON value.
    pub fn as_raw_bson_ref(&self) -> RawBsonRef<'_> {
        match self {
            RawBson::Double(d) => RawBsonRef::Double(*d),
            RawBson::String(s) => RawBsonRef::String(s.as_str()),
            RawBson::Array(a) => RawBsonRef::Array(a),
            RawBson::Document(d) => RawBsonRef::Document(d),
            RawBson::Boolean(b) => RawBsonRef::Boolean(*b),
            RawBson::Null => RawBsonRef::Null,
            RawBson::RegularExpression(re) => RawBsonRef::RegularExpression(RawRegexRef {
                options: re.options.as_str(),
                pattern: re.pattern.as_str(),
            }),
            RawBson::JavaScriptCode(c) => RawBsonRef::JavaScriptCode(c.as_str()),
            RawBson::JavaScriptCodeWithScope(code_w_scope) => {
                RawBsonRef::JavaScriptCodeWithScope(RawJavaScriptCodeWithScopeRef {
                    code: code_w_scope.code.as_str(),
                    scope: code_w_scope.scope.as_ref(),
                })
            }
            RawBson::Int32(i) => RawBsonRef::Int32(*i),
            RawBson::Int64(i) => RawBsonRef::Int64(*i),
            RawBson::Timestamp(ts) => RawBsonRef::Timestamp(*ts),
            RawBson::Binary(b) => RawBsonRef::Binary(RawBinaryRef {
                bytes: b.bytes.as_slice(),
                subtype: b.subtype,
            }),
            RawBson::ObjectId(oid) => RawBsonRef::ObjectId(*oid),
            RawBson::DateTime(dt) => RawBsonRef::DateTime(*dt),
            RawBson::Symbol(s) => RawBsonRef::Symbol(s.as_str()),
            RawBson::Decimal128(d) => RawBsonRef::Decimal128(*d),
            RawBson::Undefined => RawBsonRef::Undefined,
            RawBson::MaxKey => RawBsonRef::MaxKey,
            RawBson::MinKey => RawBsonRef::MinKey,
            RawBson::DbPointer(dbp) => RawBsonRef::DbPointer(RawDbPointerRef {
                namespace: dbp.namespace.as_str(),
                id: dbp.id,
            }),
        }
    }
}

impl From<i32> for RawBson {
    fn from(i: i32) -> Self {
        RawBson::Int32(i)
    }
}

impl From<i64> for RawBson {
    fn from(i: i64) -> Self {
        RawBson::Int64(i)
    }
}

impl From<String> for RawBson {
    fn from(s: String) -> Self {
        RawBson::String(s)
    }
}

impl From<&str> for RawBson {
    fn from(s: &str) -> Self {
        RawBson::String(s.to_owned())
    }
}

impl From<f64> for RawBson {
    fn from(f: f64) -> Self {
        RawBson::Double(f)
    }
}

impl From<bool> for RawBson {
    fn from(b: bool) -> Self {
        RawBson::Boolean(b)
    }
}

impl From<RawDocumentBuf> for RawBson {
    fn from(d: RawDocumentBuf) -> Self {
        RawBson::Document(d)
    }
}

impl From<RawArrayBuf> for RawBson {
    fn from(a: RawArrayBuf) -> Self {
        RawBson::Array(a)
    }
}

impl From<crate::DateTime> for RawBson {
    fn from(dt: crate::DateTime) -> Self {
        RawBson::DateTime(dt)
    }
}

impl From<Timestamp> for RawBson {
    fn from(ts: Timestamp) -> Self {
        RawBson::Timestamp(ts)
    }
}

impl From<ObjectId> for RawBson {
    fn from(oid: ObjectId) -> Self {
        RawBson::ObjectId(oid)
    }
}

impl From<Decimal128> for RawBson {
    fn from(d: Decimal128) -> Self {
        RawBson::Decimal128(d)
    }
}

impl From<RawJavaScriptCodeWithScope> for RawBson {
    fn from(code_w_scope: RawJavaScriptCodeWithScope) -> Self {
        RawBson::JavaScriptCodeWithScope(code_w_scope)
    }
}

impl From<Binary> for RawBson {
    fn from(b: Binary) -> Self {
        RawBson::Binary(b)
    }
}

impl From<Regex> for RawBson {
    fn from(re: Regex) -> Self {
        RawBson::RegularExpression(re)
    }
}

impl From<DbPointer> for RawBson {
    fn from(d: DbPointer) -> Self {
        RawBson::DbPointer(d)
    }
}

impl<'de> Deserialize<'de> for RawBson {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match deserializer
            .deserialize_newtype_struct(RAW_BSON_NEWTYPE, OwnedOrBorrowedRawBsonVisitor)?
        {
            OwnedOrBorrowedRawBson::Owned(o) => Ok(o),
            OwnedOrBorrowedRawBson::Borrowed(b) => Ok(b.to_raw_bson()),
        }
    }
}

impl Serialize for RawBson {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_raw_bson_ref().serialize(serializer)
    }
}

impl TryFrom<RawBson> for Bson {
    type Error = Error;

    fn try_from(rawbson: RawBson) -> Result<Bson> {
        Ok(match rawbson {
            RawBson::Double(d) => Bson::Double(d),
            RawBson::String(s) => Bson::String(s),
            RawBson::Document(rawdoc) => Bson::Document(rawdoc.as_ref().try_into()?),
            RawBson::Array(rawarray) => Bson::Array(rawarray.as_ref().try_into()?),
            RawBson::Binary(rawbson) => Bson::Binary(rawbson),
            RawBson::ObjectId(rawbson) => Bson::ObjectId(rawbson),
            RawBson::Boolean(rawbson) => Bson::Boolean(rawbson),
            RawBson::DateTime(rawbson) => Bson::DateTime(rawbson),
            RawBson::Null => Bson::Null,
            RawBson::RegularExpression(rawregex) => Bson::RegularExpression(rawregex),
            RawBson::JavaScriptCode(rawbson) => Bson::JavaScriptCode(rawbson),
            RawBson::Int32(rawbson) => Bson::Int32(rawbson),
            RawBson::Timestamp(rawbson) => Bson::Timestamp(rawbson),
            RawBson::Int64(rawbson) => Bson::Int64(rawbson),
            RawBson::Undefined => Bson::Undefined,
            RawBson::DbPointer(rawbson) => Bson::DbPointer(rawbson),
            RawBson::Symbol(rawbson) => Bson::Symbol(rawbson),
            RawBson::JavaScriptCodeWithScope(rawbson) => {
                Bson::JavaScriptCodeWithScope(crate::JavaScriptCodeWithScope {
                    code: rawbson.code,
                    scope: rawbson.scope.try_into()?,
                })
            }
            RawBson::Decimal128(rawbson) => Bson::Decimal128(rawbson),
            RawBson::MaxKey => Bson::MaxKey,
            RawBson::MinKey => Bson::MinKey,
        })
    }
}

impl TryFrom<Bson> for RawBson {
    type Error = Error;

    fn try_from(bson: Bson) -> Result<RawBson> {
        Ok(match bson {
            Bson::Double(d) => RawBson::Double(d),
            Bson::String(s) => RawBson::String(s),
            Bson::Document(doc) => RawBson::Document((&doc).try_into()?),
            Bson::Array(arr) => RawBson::Array(
                arr.into_iter()
                    .map(|b| -> Result<RawBson> { b.try_into() })
                    .collect::<Result<RawArrayBuf>>()?,
            ),
            Bson::Binary(bin) => RawBson::Binary(bin),
            Bson::ObjectId(id) => RawBson::ObjectId(id),
            Bson::Boolean(b) => RawBson::Boolean(b),
            Bson::DateTime(dt) => RawBson::DateTime(dt),
            Bson::Null => RawBson::Null,
            Bson::RegularExpression(regex) => RawBson::RegularExpression(regex),
            Bson::JavaScriptCode(s) => RawBson::JavaScriptCode(s),
            Bson::Int32(i) => RawBson::Int32(i),
            Bson::Timestamp(ts) => RawBson::Timestamp(ts),
            Bson::Int64(i) => RawBson::Int64(i),
            Bson::Undefined => RawBson::Undefined,
            Bson::DbPointer(p) => RawBson::DbPointer(p),
            Bson::Symbol(s) => RawBson::Symbol(s),
            Bson::JavaScriptCodeWithScope(jcws) => {
                RawBson::JavaScriptCodeWithScope(crate::RawJavaScriptCodeWithScope {
                    code: jcws.code,
                    scope: (&jcws.scope).try_into()?,
                })
            }
            Bson::Decimal128(d) => RawBson::Decimal128(d),
            Bson::MaxKey => RawBson::MaxKey,
            Bson::MinKey => RawBson::MinKey,
        })
    }
}

/// A BSON "code with scope" value backed by owned raw BSON.
#[derive(Debug, Clone, PartialEq)]
pub struct RawJavaScriptCodeWithScope {
    /// The code value.
    pub code: String,

    /// The scope document.
    pub scope: RawDocumentBuf,
}

impl<'de> Deserialize<'de> for RawJavaScriptCodeWithScope {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawBson::deserialize(deserializer)? {
            RawBson::JavaScriptCodeWithScope(b) => Ok(b),
            c => Err(serde::de::Error::custom(format!(
                "expected CodeWithScope, but got {:?} instead",
                c
            ))),
        }
    }
}

impl Serialize for RawJavaScriptCodeWithScope {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let raw = RawJavaScriptCodeWithScopeRef {
            code: self.code.as_str(),
            scope: self.scope.as_ref(),
        };

        raw.serialize(serializer)
    }
}
