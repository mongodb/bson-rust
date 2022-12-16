use std::convert::{TryFrom, TryInto};

use serde::{ser::SerializeStruct, Deserialize, Serialize};
use serde_bytes::Bytes;

use super::{
    bson::RawBson,
    serde::{OwnedOrBorrowedRawBson, OwnedOrBorrowedRawBsonVisitor},
    Error,
    RawArray,
    RawDocument,
    Result,
};
use crate::{
    extjson,
    oid::{self, ObjectId},
    raw::{RawJavaScriptCodeWithScope, RAW_BSON_NEWTYPE},
    spec::{BinarySubtype, ElementType},
    Binary,
    Bson,
    DbPointer,
    Decimal128,
    RawArrayBuf,
    RawDocumentBuf,
    Regex,
    Timestamp,
};

/// A BSON value referencing raw bytes stored elsewhere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RawBsonRef<'a> {
    /// 64-bit binary floating point
    Double(f64),
    /// UTF-8 string
    String(&'a str),
    /// Array
    Array(&'a RawArray),
    /// Embedded document
    Document(&'a RawDocument),
    /// Boolean value
    Boolean(bool),
    /// Null value
    Null,
    /// Regular expression
    RegularExpression(RawRegexRef<'a>),
    /// JavaScript code
    JavaScriptCode(&'a str),
    /// JavaScript code w/ scope
    JavaScriptCodeWithScope(RawJavaScriptCodeWithScopeRef<'a>),
    /// 32-bit signed integer
    Int32(i32),
    /// 64-bit signed integer
    Int64(i64),
    /// Timestamp
    Timestamp(Timestamp),
    /// Binary data
    Binary(RawBinaryRef<'a>),
    /// [ObjectId](http://dochub.mongodb.org/core/objectids)
    ObjectId(oid::ObjectId),
    /// UTC datetime
    DateTime(crate::DateTime),
    /// Symbol (Deprecated)
    Symbol(&'a str),
    /// [128-bit decimal floating point](https://github.com/mongodb/specifications/blob/master/source/bson-decimal128/decimal128.rst)
    Decimal128(Decimal128),
    /// Undefined value (Deprecated)
    Undefined,
    /// Max key
    MaxKey,
    /// Min key
    MinKey,
    /// DBPointer (Deprecated)
    DbPointer(RawDbPointerRef<'a>),
}

impl<'a> RawBsonRef<'a> {
    /// Get the [`ElementType`] of this value.
    pub fn element_type(&self) -> ElementType {
        match *self {
            RawBsonRef::Double(..) => ElementType::Double,
            RawBsonRef::String(..) => ElementType::String,
            RawBsonRef::Array(..) => ElementType::Array,
            RawBsonRef::Document(..) => ElementType::EmbeddedDocument,
            RawBsonRef::Boolean(..) => ElementType::Boolean,
            RawBsonRef::Null => ElementType::Null,
            RawBsonRef::RegularExpression(..) => ElementType::RegularExpression,
            RawBsonRef::JavaScriptCode(..) => ElementType::JavaScriptCode,
            RawBsonRef::JavaScriptCodeWithScope(..) => ElementType::JavaScriptCodeWithScope,
            RawBsonRef::Int32(..) => ElementType::Int32,
            RawBsonRef::Int64(..) => ElementType::Int64,
            RawBsonRef::Timestamp(..) => ElementType::Timestamp,
            RawBsonRef::Binary(..) => ElementType::Binary,
            RawBsonRef::ObjectId(..) => ElementType::ObjectId,
            RawBsonRef::DateTime(..) => ElementType::DateTime,
            RawBsonRef::Symbol(..) => ElementType::Symbol,
            RawBsonRef::Decimal128(..) => ElementType::Decimal128,
            RawBsonRef::Undefined => ElementType::Undefined,
            RawBsonRef::MaxKey => ElementType::MaxKey,
            RawBsonRef::MinKey => ElementType::MinKey,
            RawBsonRef::DbPointer(..) => ElementType::DbPointer,
        }
    }

    /// Gets the `f64` that's referenced or returns [`None`] if the referenced value isn't a BSON
    /// double.
    pub fn as_f64(self) -> Option<f64> {
        match self {
            RawBsonRef::Double(d) => Some(d),
            _ => None,
        }
    }

    /// Gets the `&str` that's referenced or returns [`None`] if the referenced value isn't a BSON
    /// String.
    pub fn as_str(self) -> Option<&'a str> {
        match self {
            RawBsonRef::String(s) => Some(s),
            _ => None,
        }
    }

    /// Gets the [`RawArray`] that's referenced or returns [`None`] if the referenced value
    /// isn't a BSON array.
    pub fn as_array(self) -> Option<&'a RawArray> {
        match self {
            RawBsonRef::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the [`RawDocument`] that's referenced or returns [`None`] if the referenced value
    /// isn't a BSON document.
    pub fn as_document(self) -> Option<&'a RawDocument> {
        match self {
            RawBsonRef::Document(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the `bool` that's referenced or returns [`None`] if the referenced value isn't a BSON
    /// boolean.
    pub fn as_bool(self) -> Option<bool> {
        match self {
            RawBsonRef::Boolean(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the `i32` that's referenced or returns [`None`] if the referenced value isn't a BSON
    /// Int32.
    pub fn as_i32(self) -> Option<i32> {
        match self {
            RawBsonRef::Int32(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the `i64` that's referenced or returns [`None`] if the referenced value isn't a BSON
    /// Int64.
    pub fn as_i64(self) -> Option<i64> {
        match self {
            RawBsonRef::Int64(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the [`crate::oid::ObjectId`] that's referenced or returns [`None`] if the referenced
    /// value isn't a BSON ObjectID.
    pub fn as_object_id(self) -> Option<oid::ObjectId> {
        match self {
            RawBsonRef::ObjectId(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the [`RawBinaryRef`] that's referenced or returns [`None`] if the referenced value
    /// isn't a BSON binary.
    pub fn as_binary(self) -> Option<RawBinaryRef<'a>> {
        match self {
            RawBsonRef::Binary(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the [`RawRegexRef`] that's referenced or returns [`None`] if the referenced value isn't
    /// a BSON regular expression.
    pub fn as_regex(self) -> Option<RawRegexRef<'a>> {
        match self {
            RawBsonRef::RegularExpression(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the [`crate::DateTime`] that's referenced or returns [`None`] if the referenced value
    /// isn't a BSON datetime.
    pub fn as_datetime(self) -> Option<crate::DateTime> {
        match self {
            RawBsonRef::DateTime(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the symbol that's referenced or returns [`None`] if the referenced value isn't a BSON
    /// symbol.
    pub fn as_symbol(self) -> Option<&'a str> {
        match self {
            RawBsonRef::Symbol(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the [`crate::Timestamp`] that's referenced or returns [`None`] if the referenced value
    /// isn't a BSON timestamp.
    pub fn as_timestamp(self) -> Option<Timestamp> {
        match self {
            RawBsonRef::Timestamp(timestamp) => Some(timestamp),
            _ => None,
        }
    }

    /// Gets the null value that's referenced or returns [`None`] if the referenced value isn't a
    /// BSON null.
    pub fn as_null(self) -> Option<()> {
        match self {
            RawBsonRef::Null => Some(()),
            _ => None,
        }
    }

    /// Gets the [`RawDbPointerRef`] that's referenced or returns [`None`] if the referenced value
    /// isn't a BSON DB pointer.
    pub fn as_db_pointer(self) -> Option<RawDbPointerRef<'a>> {
        match self {
            RawBsonRef::DbPointer(d) => Some(d),
            _ => None,
        }
    }

    /// Gets the code that's referenced or returns [`None`] if the referenced value isn't a BSON
    /// JavaScript.
    pub fn as_javascript(self) -> Option<&'a str> {
        match self {
            RawBsonRef::JavaScriptCode(s) => Some(s),
            _ => None,
        }
    }

    /// Gets the [`RawJavaScriptCodeWithScope`] that's referenced or returns [`None`] if the
    /// referenced value isn't a BSON JavaScript with scope.
    pub fn as_javascript_with_scope(self) -> Option<RawJavaScriptCodeWithScopeRef<'a>> {
        match self {
            RawBsonRef::JavaScriptCodeWithScope(s) => Some(s),
            _ => None,
        }
    }

    /// Convert this [`RawBsonRef`] to the equivalent [`RawBson`].
    pub fn to_raw_bson(self) -> RawBson {
        match self {
            RawBsonRef::Double(d) => RawBson::Double(d),
            RawBsonRef::String(s) => RawBson::String(s.to_string()),
            RawBsonRef::Array(a) => RawBson::Array(a.to_owned()),
            RawBsonRef::Document(d) => RawBson::Document(d.to_owned()),
            RawBsonRef::Boolean(b) => RawBson::Boolean(b),
            RawBsonRef::Null => RawBson::Null,
            RawBsonRef::RegularExpression(re) => {
                RawBson::RegularExpression(Regex::new(re.pattern, re.options))
            }
            RawBsonRef::JavaScriptCode(c) => RawBson::JavaScriptCode(c.to_owned()),
            RawBsonRef::JavaScriptCodeWithScope(c_w_s) => {
                RawBson::JavaScriptCodeWithScope(RawJavaScriptCodeWithScope {
                    code: c_w_s.code.to_string(),
                    scope: c_w_s.scope.to_owned(),
                })
            }
            RawBsonRef::Int32(i) => RawBson::Int32(i),
            RawBsonRef::Int64(i) => RawBson::Int64(i),
            RawBsonRef::Timestamp(t) => RawBson::Timestamp(t),
            RawBsonRef::Binary(b) => RawBson::Binary(Binary {
                bytes: b.bytes.to_vec(),
                subtype: b.subtype,
            }),
            RawBsonRef::ObjectId(o) => RawBson::ObjectId(o),
            RawBsonRef::DateTime(dt) => RawBson::DateTime(dt),
            RawBsonRef::Symbol(s) => RawBson::Symbol(s.to_string()),
            RawBsonRef::Decimal128(d) => RawBson::Decimal128(d),
            RawBsonRef::Undefined => RawBson::Undefined,
            RawBsonRef::MaxKey => RawBson::MaxKey,
            RawBsonRef::MinKey => RawBson::MinKey,
            RawBsonRef::DbPointer(d) => RawBson::DbPointer(DbPointer {
                namespace: d.namespace.to_string(),
                id: d.id,
            }),
        }
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for RawBsonRef<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match deserializer
            .deserialize_newtype_struct(RAW_BSON_NEWTYPE, OwnedOrBorrowedRawBsonVisitor)?
        {
            OwnedOrBorrowedRawBson::Borrowed(b) => Ok(b),
            o => Err(serde::de::Error::custom(format!(
                "RawBson must be deserialized from borrowed content, instead got {:?}",
                o
            ))),
        }
    }
}

impl<'a> Serialize for RawBsonRef<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            RawBsonRef::Double(v) => serializer.serialize_f64(*v),
            RawBsonRef::String(v) => serializer.serialize_str(v),
            RawBsonRef::Array(v) => v.serialize(serializer),
            RawBsonRef::Document(v) => v.serialize(serializer),
            RawBsonRef::Boolean(v) => serializer.serialize_bool(*v),
            RawBsonRef::Null => serializer.serialize_unit(),
            RawBsonRef::Int32(v) => serializer.serialize_i32(*v),
            RawBsonRef::Int64(v) => serializer.serialize_i64(*v),
            RawBsonRef::ObjectId(oid) => oid.serialize(serializer),
            RawBsonRef::DateTime(dt) => dt.serialize(serializer),
            RawBsonRef::Binary(b) => b.serialize(serializer),
            RawBsonRef::JavaScriptCode(c) => {
                let mut state = serializer.serialize_struct("$code", 1)?;
                state.serialize_field("$code", c)?;
                state.end()
            }
            RawBsonRef::JavaScriptCodeWithScope(code_w_scope) => code_w_scope.serialize(serializer),
            RawBsonRef::DbPointer(dbp) => dbp.serialize(serializer),
            RawBsonRef::Symbol(s) => {
                let mut state = serializer.serialize_struct("$symbol", 1)?;
                state.serialize_field("$symbol", s)?;
                state.end()
            }
            RawBsonRef::RegularExpression(re) => re.serialize(serializer),
            RawBsonRef::Timestamp(t) => t.serialize(serializer),
            RawBsonRef::Decimal128(d) => d.serialize(serializer),
            RawBsonRef::Undefined => {
                let mut state = serializer.serialize_struct("$undefined", 1)?;
                state.serialize_field("$undefined", &true)?;
                state.end()
            }
            RawBsonRef::MaxKey => {
                let mut state = serializer.serialize_struct("$maxKey", 1)?;
                state.serialize_field("$maxKey", &1)?;
                state.end()
            }
            RawBsonRef::MinKey => {
                let mut state = serializer.serialize_struct("$minKey", 1)?;
                state.serialize_field("$minKey", &1)?;
                state.end()
            }
        }
    }
}

impl<'a> TryFrom<RawBsonRef<'a>> for Bson {
    type Error = Error;

    fn try_from(rawbson: RawBsonRef<'a>) -> Result<Bson> {
        rawbson.to_raw_bson().try_into()
    }
}

impl<'a> From<i32> for RawBsonRef<'a> {
    fn from(i: i32) -> Self {
        RawBsonRef::Int32(i)
    }
}

impl<'a> From<i64> for RawBsonRef<'a> {
    fn from(i: i64) -> Self {
        RawBsonRef::Int64(i)
    }
}

impl<'a> From<&'a str> for RawBsonRef<'a> {
    fn from(s: &'a str) -> Self {
        RawBsonRef::String(s)
    }
}

impl<'a> From<f64> for RawBsonRef<'a> {
    fn from(f: f64) -> Self {
        RawBsonRef::Double(f)
    }
}

impl<'a> From<bool> for RawBsonRef<'a> {
    fn from(b: bool) -> Self {
        RawBsonRef::Boolean(b)
    }
}

impl<'a> From<&'a RawDocumentBuf> for RawBsonRef<'a> {
    fn from(d: &'a RawDocumentBuf) -> Self {
        RawBsonRef::Document(d.as_ref())
    }
}

impl<'a> From<&'a RawDocument> for RawBsonRef<'a> {
    fn from(d: &'a RawDocument) -> Self {
        RawBsonRef::Document(d)
    }
}

impl<'a> From<&'a RawArray> for RawBsonRef<'a> {
    fn from(a: &'a RawArray) -> Self {
        RawBsonRef::Array(a)
    }
}

impl<'a> From<&'a RawArrayBuf> for RawBsonRef<'a> {
    fn from(a: &'a RawArrayBuf) -> Self {
        RawBsonRef::Array(a)
    }
}

impl<'a> From<crate::DateTime> for RawBsonRef<'a> {
    fn from(dt: crate::DateTime) -> Self {
        RawBsonRef::DateTime(dt)
    }
}

impl<'a> From<Timestamp> for RawBsonRef<'a> {
    fn from(ts: Timestamp) -> Self {
        RawBsonRef::Timestamp(ts)
    }
}

impl<'a> From<ObjectId> for RawBsonRef<'a> {
    fn from(oid: ObjectId) -> Self {
        RawBsonRef::ObjectId(oid)
    }
}

impl<'a> From<Decimal128> for RawBsonRef<'a> {
    fn from(d: Decimal128) -> Self {
        RawBsonRef::Decimal128(d)
    }
}

/// A BSON binary value referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawBinaryRef<'a> {
    /// The subtype of the binary value.
    pub subtype: BinarySubtype,

    /// The binary bytes.
    pub bytes: &'a [u8],
}

impl<'a> RawBinaryRef<'a> {
    /// Copy the contents into a [`Binary`].
    pub fn to_binary(&self) -> Binary {
        Binary {
            subtype: self.subtype,
            bytes: self.bytes.to_owned(),
        }
    }

    pub(crate) fn len(&self) -> i32 {
        match self.subtype {
            BinarySubtype::BinaryOld => self.bytes.len() as i32 + 4,
            _ => self.bytes.len() as i32,
        }
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for RawBinaryRef<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawBsonRef::deserialize(deserializer)? {
            RawBsonRef::Binary(b) => Ok(b),
            c => Err(serde::de::Error::custom(format!(
                "expected binary, but got {:?} instead",
                c
            ))),
        }
    }
}

impl<'a> Serialize for RawBinaryRef<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if let BinarySubtype::Generic = self.subtype {
            serializer.serialize_bytes(self.bytes)
        } else if !serializer.is_human_readable() {
            #[derive(Serialize)]
            struct BorrowedBinary<'a> {
                bytes: &'a Bytes,

                #[serde(rename = "subType")]
                subtype: u8,
            }

            let mut state = serializer.serialize_struct("$binary", 1)?;
            let body = BorrowedBinary {
                bytes: Bytes::new(self.bytes),
                subtype: self.subtype.into(),
            };
            state.serialize_field("$binary", &body)?;
            state.end()
        } else {
            let mut state = serializer.serialize_struct("$binary", 1)?;
            let body = extjson::models::BinaryBody {
                base64: base64::encode(self.bytes),
                subtype: hex::encode([self.subtype.into()]),
            };
            state.serialize_field("$binary", &body)?;
            state.end()
        }
    }
}

impl<'a> From<RawBinaryRef<'a>> for RawBsonRef<'a> {
    fn from(b: RawBinaryRef<'a>) -> Self {
        RawBsonRef::Binary(b)
    }
}

impl<'a> From<&'a Binary> for RawBsonRef<'a> {
    fn from(bin: &'a Binary) -> Self {
        bin.as_raw_binary().into()
    }
}

/// A BSON regex referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawRegexRef<'a> {
    /// The regex pattern to match.
    pub pattern: &'a str,

    /// The options for the regex.
    ///
    /// Options are identified by characters, which must be stored in
    /// alphabetical order. Valid options are 'i' for case insensitive matching, 'm' for
    /// multiline matching, 'x' for verbose mode, 'l' to make \w, \W, etc. locale dependent,
    /// 's' for dotall mode ('.' matches everything), and 'u' to make \w, \W, etc. match
    /// unicode.
    pub options: &'a str,
}

impl<'de: 'a, 'a> Deserialize<'de> for RawRegexRef<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawBsonRef::deserialize(deserializer)? {
            RawBsonRef::RegularExpression(b) => Ok(b),
            c => Err(serde::de::Error::custom(format!(
                "expected Regex, but got {:?} instead",
                c
            ))),
        }
    }
}

impl<'a> Serialize for RawRegexRef<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct BorrowedRegexBody<'a> {
            pattern: &'a str,
            options: &'a str,
        }

        let mut state = serializer.serialize_struct("$regularExpression", 1)?;
        let body = BorrowedRegexBody {
            pattern: self.pattern,
            options: self.options,
        };
        state.serialize_field("$regularExpression", &body)?;
        state.end()
    }
}

impl<'a> From<RawRegexRef<'a>> for RawBsonRef<'a> {
    fn from(re: RawRegexRef<'a>) -> Self {
        RawBsonRef::RegularExpression(re)
    }
}

/// A BSON "code with scope" value referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawJavaScriptCodeWithScopeRef<'a> {
    /// The JavaScript code.
    pub code: &'a str,

    /// The scope document containing variable bindings.
    pub scope: &'a RawDocument,
}

impl<'a> RawJavaScriptCodeWithScopeRef<'a> {
    pub(crate) fn len(self) -> i32 {
        4 + 4 + self.code.len() as i32 + 1 + self.scope.as_bytes().len() as i32
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for RawJavaScriptCodeWithScopeRef<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawBsonRef::deserialize(deserializer)? {
            RawBsonRef::JavaScriptCodeWithScope(b) => Ok(b),
            c => Err(serde::de::Error::custom(format!(
                "expected CodeWithScope, but got {:?} instead",
                c
            ))),
        }
    }
}

impl<'a> Serialize for RawJavaScriptCodeWithScopeRef<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("$codeWithScope", 2)?;
        state.serialize_field("$code", &self.code)?;
        state.serialize_field("$scope", &self.scope)?;
        state.end()
    }
}

impl<'a> From<RawJavaScriptCodeWithScopeRef<'a>> for RawBsonRef<'a> {
    fn from(code_w_scope: RawJavaScriptCodeWithScopeRef<'a>) -> Self {
        RawBsonRef::JavaScriptCodeWithScope(code_w_scope)
    }
}

/// A BSON DB pointer value referencing raw bytes stored elesewhere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawDbPointerRef<'a> {
    pub(crate) namespace: &'a str,
    pub(crate) id: ObjectId,
}

impl<'de: 'a, 'a> Deserialize<'de> for RawDbPointerRef<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawBsonRef::deserialize(deserializer)? {
            RawBsonRef::DbPointer(b) => Ok(b),
            c => Err(serde::de::Error::custom(format!(
                "expected DbPointer, but got {:?} instead",
                c
            ))),
        }
    }
}

impl<'a> Serialize for RawDbPointerRef<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        #[derive(Serialize)]
        struct BorrowedDbPointerBody<'a> {
            #[serde(rename = "$ref")]
            ref_ns: &'a str,

            #[serde(rename = "$id")]
            id: ObjectId,
        }

        let mut state = serializer.serialize_struct("$dbPointer", 1)?;
        let body = BorrowedDbPointerBody {
            ref_ns: self.namespace,
            id: self.id,
        };
        state.serialize_field("$dbPointer", &body)?;
        state.end()
    }
}
