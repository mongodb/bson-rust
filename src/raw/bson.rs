use std::{
    convert::{TryFrom, TryInto},
    iter::FromIterator,
};

use serde::{de::Visitor, ser::SerializeStruct, Deserialize, Serialize};
use serde_bytes::{ByteBuf, Bytes};

use super::{
    owned_bson::OwnedRawBson,
    serde::{OwnedOrBorrowedRawBson, OwnedOrBorrowedRawBsonVisitor},
    Error, RawArray, RawDocument, Result,
};
use crate::{
    de::convert_unsigned_to_signed_raw,
    extjson,
    oid::{self, ObjectId},
    raw::{
        OwnedRawJavaScriptCodeWithScope, RAW_ARRAY_NEWTYPE, RAW_BSON_NEWTYPE, RAW_DOCUMENT_NEWTYPE,
    },
    spec::{BinarySubtype, ElementType},
    Binary, Bson, DateTime, DbPointer, Decimal128, JavaScriptCodeWithScope, RawArrayBuf,
    RawDocumentBuf, Regex, Timestamp,
};

/// A BSON value referencing raw bytes stored elsewhere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RawBson<'a> {
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
    RegularExpression(RawRegex<'a>),
    /// JavaScript code
    JavaScriptCode(&'a str),
    /// JavaScript code w/ scope
    JavaScriptCodeWithScope(RawJavaScriptCodeWithScope<'a>),
    /// 32-bit signed integer
    Int32(i32),
    /// 64-bit signed integer
    Int64(i64),
    /// Timestamp
    Timestamp(Timestamp),
    /// Binary data
    Binary(RawBinary<'a>),
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
    DbPointer(RawDbPointer<'a>),
}

impl<'a> RawBson<'a> {
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

    /// Gets the `f64` that's referenced or returns `None` if the referenced value isn't a BSON
    /// double.
    pub fn as_f64(self) -> Option<f64> {
        match self {
            RawBson::Double(d) => Some(d),
            _ => None,
        }
    }

    /// Gets the `&str` that's referenced or returns `None` if the referenced value isn't a BSON
    /// String.
    pub fn as_str(self) -> Option<&'a str> {
        match self {
            RawBson::String(s) => Some(s),
            _ => None,
        }
    }

    /// Gets the [`RawArray`] that's referenced or returns `None` if the referenced value
    /// isn't a BSON array.
    pub fn as_array(self) -> Option<&'a RawArray> {
        match self {
            RawBson::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the [`RawDocument`] that's referenced or returns `None` if the referenced value
    /// isn't a BSON document.
    pub fn as_document(self) -> Option<&'a RawDocument> {
        match self {
            RawBson::Document(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the `bool` that's referenced or returns `None` if the referenced value isn't a BSON
    /// boolean.
    pub fn as_bool(self) -> Option<bool> {
        match self {
            RawBson::Boolean(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the `i32` that's referenced or returns `None` if the referenced value isn't a BSON
    /// Int32.
    pub fn as_i32(self) -> Option<i32> {
        match self {
            RawBson::Int32(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the `i64` that's referenced or returns `None` if the referenced value isn't a BSON
    /// Int64.
    pub fn as_i64(self) -> Option<i64> {
        match self {
            RawBson::Int64(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the [`crate::oid::ObjectId`] that's referenced or returns `None` if the referenced
    /// value isn't a BSON ObjectID.
    pub fn as_object_id(self) -> Option<oid::ObjectId> {
        match self {
            RawBson::ObjectId(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the [`RawBinary`] that's referenced or returns `None` if the referenced value isn't a
    /// BSON binary.
    pub fn as_binary(self) -> Option<RawBinary<'a>> {
        match self {
            RawBson::Binary(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the [`RawRegex`] that's referenced or returns `None` if the referenced value isn't a
    /// BSON regular expression.
    pub fn as_regex(self) -> Option<RawRegex<'a>> {
        match self {
            RawBson::RegularExpression(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the [`crate::DateTime`] that's referenced or returns `None` if the referenced value
    /// isn't a BSON datetime.
    pub fn as_datetime(self) -> Option<crate::DateTime> {
        match self {
            RawBson::DateTime(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the symbol that's referenced or returns `None` if the referenced value isn't a BSON
    /// symbol.
    pub fn as_symbol(self) -> Option<&'a str> {
        match self {
            RawBson::Symbol(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the [`crate::Timestamp`] that's referenced or returns `None` if the referenced value
    /// isn't a BSON timestamp.
    pub fn as_timestamp(self) -> Option<Timestamp> {
        match self {
            RawBson::Timestamp(timestamp) => Some(timestamp),
            _ => None,
        }
    }

    /// Gets the null value that's referenced or returns `None` if the referenced value isn't a BSON
    /// null.
    pub fn as_null(self) -> Option<()> {
        match self {
            RawBson::Null => Some(()),
            _ => None,
        }
    }

    /// Gets the [`RawDbPointer`] that's referenced or returns `None` if the referenced value isn't
    /// a BSON DB pointer.
    pub fn as_db_pointer(self) -> Option<RawDbPointer<'a>> {
        match self {
            RawBson::DbPointer(d) => Some(d),
            _ => None,
        }
    }

    /// Gets the code that's referenced or returns `None` if the referenced value isn't a BSON
    /// JavaScript.
    pub fn as_javascript(self) -> Option<&'a str> {
        match self {
            RawBson::JavaScriptCode(s) => Some(s),
            _ => None,
        }
    }

    /// Gets the [`RawJavaScriptCodeWithScope`] that's referenced or returns `None` if the
    /// referenced value isn't a BSON JavaScript with scope.
    pub fn as_javascript_with_scope(self) -> Option<RawJavaScriptCodeWithScope<'a>> {
        match self {
            RawBson::JavaScriptCodeWithScope(s) => Some(s),
            _ => None,
        }
    }

    /// Convert this [`RawBson`] to the equivalent [`OwnedRawBson`].
    pub fn to_owned_raw_bson(self) -> OwnedRawBson {
        match self {
            RawBson::Double(d) => OwnedRawBson::Double(d),
            RawBson::String(s) => OwnedRawBson::String(s.to_string()),
            RawBson::Array(a) => OwnedRawBson::Array(a.to_owned()),
            RawBson::Document(d) => OwnedRawBson::Document(d.to_owned()),
            RawBson::Boolean(b) => OwnedRawBson::Boolean(b),
            RawBson::Null => OwnedRawBson::Null,
            RawBson::RegularExpression(re) => {
                OwnedRawBson::RegularExpression(Regex::new(re.pattern, re.options))
            }
            RawBson::JavaScriptCode(c) => OwnedRawBson::JavaScriptCode(c.to_owned()),
            RawBson::JavaScriptCodeWithScope(c_w_s) => {
                OwnedRawBson::JavaScriptCodeWithScope(OwnedRawJavaScriptCodeWithScope {
                    code: c_w_s.code.to_string(),
                    scope: c_w_s.scope.to_owned(),
                })
            }
            RawBson::Int32(i) => OwnedRawBson::Int32(i),
            RawBson::Int64(i) => OwnedRawBson::Int64(i),
            RawBson::Timestamp(t) => OwnedRawBson::Timestamp(t),
            RawBson::Binary(b) => OwnedRawBson::Binary(Binary {
                bytes: b.bytes.to_vec(),
                subtype: b.subtype
            }),
            RawBson::ObjectId(o) => OwnedRawBson::ObjectId(o),
            RawBson::DateTime(dt) => OwnedRawBson::DateTime(dt),
            RawBson::Symbol(s) => OwnedRawBson::Symbol(s.to_string()),
            RawBson::Decimal128(d) => OwnedRawBson::Decimal128(d),
            RawBson::Undefined => OwnedRawBson::Undefined,
            RawBson::MaxKey => OwnedRawBson::MaxKey,
            RawBson::MinKey => OwnedRawBson::MinKey,
            RawBson::DbPointer(d) => OwnedRawBson::DbPointer(DbPointer {
                namespace: d.namespace.to_string(),
                id: d.id
            }),
        }
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for RawBson<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match deserializer
            .deserialize_newtype_struct(RAW_BSON_NEWTYPE, OwnedOrBorrowedRawBsonVisitor)?
        {
            OwnedOrBorrowedRawBson::Borrowed(b) => Ok(b),
            _ => Err(serde::de::Error::custom(
                "RawBson must be deserialized from borrowed content",
            )),
        }
    }
}

impl<'a> Serialize for RawBson<'a> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            RawBson::Double(v) => serializer.serialize_f64(*v),
            RawBson::String(v) => serializer.serialize_str(v),
            RawBson::Array(v) => v.serialize(serializer),
            RawBson::Document(v) => v.serialize(serializer),
            RawBson::Boolean(v) => serializer.serialize_bool(*v),
            RawBson::Null => serializer.serialize_unit(),
            RawBson::Int32(v) => serializer.serialize_i32(*v),
            RawBson::Int64(v) => serializer.serialize_i64(*v),
            RawBson::ObjectId(oid) => oid.serialize(serializer),
            RawBson::DateTime(dt) => dt.serialize(serializer),
            RawBson::Binary(b) => b.serialize(serializer),
            RawBson::JavaScriptCode(c) => {
                let mut state = serializer.serialize_struct("$code", 1)?;
                state.serialize_field("$code", c)?;
                state.end()
            }
            RawBson::JavaScriptCodeWithScope(code_w_scope) => code_w_scope.serialize(serializer),
            RawBson::DbPointer(dbp) => dbp.serialize(serializer),
            RawBson::Symbol(s) => {
                let mut state = serializer.serialize_struct("$symbol", 1)?;
                state.serialize_field("$symbol", s)?;
                state.end()
            }
            RawBson::RegularExpression(re) => re.serialize(serializer),
            RawBson::Timestamp(t) => t.serialize(serializer),
            RawBson::Decimal128(d) => d.serialize(serializer),
            RawBson::Undefined => {
                let mut state = serializer.serialize_struct("$undefined", 1)?;
                state.serialize_field("$undefined", &true)?;
                state.end()
            }
            RawBson::MaxKey => {
                let mut state = serializer.serialize_struct("$maxKey", 1)?;
                state.serialize_field("$maxKey", &1)?;
                state.end()
            }
            RawBson::MinKey => {
                let mut state = serializer.serialize_struct("$minKey", 1)?;
                state.serialize_field("$minKey", &1)?;
                state.end()
            }
        }
    }
}

impl<'a> TryFrom<RawBson<'a>> for Bson {
    type Error = Error;

    fn try_from(rawbson: RawBson<'a>) -> Result<Bson> {
        rawbson.to_owned_raw_bson().try_into()
    }
}

impl<'a> From<i32> for RawBson<'a> {
    fn from(i: i32) -> Self {
        RawBson::Int32(i)
    }
}

impl<'a> From<i64> for RawBson<'a> {
    fn from(i: i64) -> Self {
        RawBson::Int64(i)
    }
}

impl<'a> From<&'a str> for RawBson<'a> {
    fn from(s: &'a str) -> Self {
        RawBson::String(s)
    }
}

impl<'a> From<f64> for RawBson<'a> {
    fn from(f: f64) -> Self {
        RawBson::Double(f)
    }
}

impl<'a> From<bool> for RawBson<'a> {
    fn from(b: bool) -> Self {
        RawBson::Boolean(b)
    }
}

impl<'a> From<&'a RawDocumentBuf> for RawBson<'a> {
    fn from(d: &'a RawDocumentBuf) -> Self {
        RawBson::Document(d.as_ref())
    }
}

impl<'a> From<&'a RawDocument> for RawBson<'a> {
    fn from(d: &'a RawDocument) -> Self {
        RawBson::Document(d)
    }
}

impl<'a> From<&'a RawArray> for RawBson<'a> {
    fn from(a: &'a RawArray) -> Self {
        RawBson::Array(a)
    }
}

impl<'a> From<&'a RawArrayBuf> for RawBson<'a> {
    fn from(a: &'a RawArrayBuf) -> Self {
        RawBson::Array(a)
    }
}

impl<'a> From<crate::DateTime> for RawBson<'a> {
    fn from(dt: crate::DateTime) -> Self {
        RawBson::DateTime(dt)
    }
}

impl<'a> From<Timestamp> for RawBson<'a> {
    fn from(ts: Timestamp) -> Self {
        RawBson::Timestamp(ts)
    }
}

impl<'a> From<ObjectId> for RawBson<'a> {
    fn from(oid: ObjectId) -> Self {
        RawBson::ObjectId(oid)
    }
}

impl<'a> From<Decimal128> for RawBson<'a> {
    fn from(d: Decimal128) -> Self {
        RawBson::Decimal128(d)
    }
}

/// A BSON binary value referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawBinary<'a> {
    /// The subtype of the binary value.
    pub subtype: BinarySubtype,

    /// The binary bytes.
    pub bytes: &'a [u8],
}

impl<'a> RawBinary<'a> {
    pub(crate) fn len(&self) -> i32 {
        match self.subtype {
            BinarySubtype::BinaryOld => self.bytes.len() as i32 + 4,
            _ => self.bytes.len() as i32,
        }
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for RawBinary<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawBson::deserialize(deserializer)? {
            RawBson::Binary(b) => Ok(b),
            c => Err(serde::de::Error::custom(format!(
                "expected binary, but got {:?} instead",
                c
            ))),
        }
    }
}

impl<'a> Serialize for RawBinary<'a> {
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

impl<'a> From<RawBinary<'a>> for RawBson<'a> {
    fn from(b: RawBinary<'a>) -> Self {
        RawBson::Binary(b)
    }
}

impl<'a> From<&'a Binary> for RawBson<'a> {
    fn from(bin: &'a Binary) -> Self {
        bin.as_raw_binary().into()
    }
}

/// A BSON regex referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawRegex<'a> {
    pub(crate) pattern: &'a str,
    pub(crate) options: &'a str,
}

impl<'a> RawRegex<'a> {
    /// Gets the pattern portion of the regex.
    pub fn pattern(self) -> &'a str {
        self.pattern
    }

    /// Gets the options portion of the regex.
    pub fn options(self) -> &'a str {
        self.options
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for RawRegex<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawBson::deserialize(deserializer)? {
            RawBson::RegularExpression(b) => Ok(b),
            c => Err(serde::de::Error::custom(format!(
                "expected Regex, but got {:?} instead",
                c
            ))),
        }
    }
}

impl<'a> Serialize for RawRegex<'a> {
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

impl<'a> From<RawRegex<'a>> for RawBson<'a> {
    fn from(re: RawRegex<'a>) -> Self {
        RawBson::RegularExpression(re)
    }
}

/// A BSON "code with scope" value referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawJavaScriptCodeWithScope<'a> {
    pub(crate) code: &'a str,

    pub(crate) scope: &'a RawDocument,
}

impl<'a> RawJavaScriptCodeWithScope<'a> {
    /// Gets the code in the value.
    pub fn code(self) -> &'a str {
        self.code
    }

    /// Gets the scope in the value.
    pub fn scope(self) -> &'a RawDocument {
        self.scope
    }

    pub(crate) fn len(self) -> i32 {
        4 + 4 + self.code.len() as i32 + 1 + self.scope.len() as i32
    }
}

impl<'de: 'a, 'a> Deserialize<'de> for RawJavaScriptCodeWithScope<'a> {
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

impl<'a> Serialize for RawJavaScriptCodeWithScope<'a> {
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

impl<'a> From<RawJavaScriptCodeWithScope<'a>> for RawBson<'a> {
    fn from(code_w_scope: RawJavaScriptCodeWithScope<'a>) -> Self {
        RawBson::JavaScriptCodeWithScope(code_w_scope)
    }
}

/// A BSON DB pointer value referencing raw bytes stored elesewhere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawDbPointer<'a> {
    pub(crate) namespace: &'a str,
    pub(crate) id: ObjectId,
}

impl<'de: 'a, 'a> Deserialize<'de> for RawDbPointer<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match RawBson::deserialize(deserializer)? {
            RawBson::DbPointer(b) => Ok(b),
            c => Err(serde::de::Error::custom(format!(
                "expected DbPointer, but got {:?} instead",
                c
            ))),
        }
    }
}

impl<'a> Serialize for RawDbPointer<'a> {
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
