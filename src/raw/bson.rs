use std::convert::{TryFrom, TryInto};

use serde::{
    de::{MapAccess, Unexpected, Visitor},
    Deserialize,
};

use super::{Error, RawArray, RawDocument, Result};
use crate::{
    extjson,
    oid::{self, ObjectId},
    raw::{RAW_ARRAY_NEWTYPE, RAW_BSON_NEWTYPE, RAW_DOCUMENT_NEWTYPE},
    spec::{BinarySubtype, ElementType},
    Bson,
    DateTime,
    DbPointer,
    Decimal128,
    Timestamp,
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
}

impl<'de: 'a, 'a> Deserialize<'de> for RawBson<'a> {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::Error as SerdeError;

        struct RawBsonVisitor;

        impl<'de> Visitor<'de> for RawBsonVisitor {
            type Value = RawBson<'de>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "a raw BSON reference")
            }

            fn visit_borrowed_str<E>(self, v: &'de str) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RawBson::String(v))
            }

            fn visit_borrowed_bytes<E>(
                self,
                bytes: &'de [u8],
            ) -> std::result::Result<Self::Value, E>
            where
                E: SerdeError,
            {
                Ok(RawBson::Binary(RawBinary {
                    bytes,
                    subtype: BinarySubtype::Generic,
                }))
            }

            fn visit_i8<E>(self, v: i8) -> std::result::Result<Self::Value, E>
            where
                E: SerdeError,
            {
                Ok(RawBson::Int32(v.into()))
            }

            fn visit_i16<E>(self, v: i16) -> std::result::Result<Self::Value, E>
            where
                E: SerdeError,
            {
                Ok(RawBson::Int32(v.into()))
            }

            fn visit_i32<E>(self, v: i32) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RawBson::Int32(v))
            }

            fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RawBson::Int64(v))
            }

            fn visit_u8<E>(self, value: u8) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                crate::de::convert_unsigned_to_signed_raw(value.into())
            }

            fn visit_u16<E>(self, value: u16) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                crate::de::convert_unsigned_to_signed_raw(value.into())
            }

            fn visit_u32<E>(self, value: u32) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                crate::de::convert_unsigned_to_signed_raw(value.into())
            }

            fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                crate::de::convert_unsigned_to_signed_raw(value)
            }

            fn visit_bool<E>(self, v: bool) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RawBson::Boolean(v))
            }

            fn visit_f64<E>(self, v: f64) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RawBson::Double(v))
            }

            fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RawBson::Null)
            }

            fn visit_unit<E>(self) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RawBson::Null)
            }

            fn visit_newtype_struct<D>(
                self,
                deserializer: D,
            ) -> std::result::Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                deserializer.deserialize_any(self)
            }

            // use extjson for: ObjectId, datetime, timestamp, symbol, minkey, maxkey
            fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                let k = map.next_key::<&str>()?.ok_or_else(|| {
                    SerdeError::custom("expected a key when deserializing RawBson")
                })?;
                match k {
                    "$oid" => {
                        let oid: ObjectId = map.next_value()?;
                        Ok(RawBson::ObjectId(oid))
                    }
                    "$symbol" => {
                        let s: &str = map.next_value()?;
                        Ok(RawBson::Symbol(s))
                    }
                    "$numberDecimalBytes" => Ok(RawBson::Decimal128(map.next_value()?)),
                    "$regularExpression" => {
                        #[derive(Debug, Deserialize)]
                        struct BorrowedRegexBody<'a> {
                            pattern: &'a str,

                            options: &'a str,
                        }
                        let body: BorrowedRegexBody = map.next_value()?;
                        Ok(RawBson::RegularExpression(RawRegex {
                            pattern: body.pattern,
                            options: body.options,
                        }))
                    }
                    "$undefined" => {
                        let _: bool = map.next_value()?;
                        Ok(RawBson::Undefined)
                    }
                    "$binary" => {
                        #[derive(Debug, Deserialize)]
                        struct BorrowedBinaryBody<'a> {
                            base64: &'a [u8],

                            #[serde(rename = "subType")]
                            subtype: u8,
                        }

                        let v = map.next_value::<BorrowedBinaryBody>()?;

                        Ok(RawBson::Binary(RawBinary {
                            bytes: v.base64,
                            subtype: v.subtype.into(),
                        }))
                    }
                    "$date" => {
                        let v = map.next_value::<i64>()?;
                        Ok(RawBson::DateTime(DateTime::from_millis(v)))
                    }
                    "$timestamp" => {
                        let v = map.next_value::<extjson::models::TimestampBody>()?;
                        Ok(RawBson::Timestamp(Timestamp {
                            time: v.t,
                            increment: v.i,
                        }))
                    }
                    "$minKey" => {
                        let _ = map.next_value::<i32>()?;
                        Ok(RawBson::MinKey)
                    }
                    "$maxKey" => {
                        let _ = map.next_value::<i32>()?;
                        Ok(RawBson::MaxKey)
                    }
                    "$code" => {
                        let code = map.next_value::<&str>()?;
                        if let Some(key) = map.next_key::<&str>()? {
                            if key == "$scope" {
                                let scope = map.next_value::<&RawDocument>()?;
                                Ok(RawBson::JavaScriptCodeWithScope(
                                    RawJavaScriptCodeWithScope { code, scope },
                                ))
                            } else {
                                Err(SerdeError::unknown_field(key, &["$scope"]))
                            }
                        } else {
                            Ok(RawBson::JavaScriptCode(code))
                        }
                    }
                    "$dbPointer" => {
                        #[derive(Deserialize)]
                        struct BorrowedDbPointerBody<'a> {
                            #[serde(rename = "$ref")]
                            ns: &'a str,

                            #[serde(rename = "$id")]
                            id: ObjectId,
                        }

                        let body: BorrowedDbPointerBody = map.next_value()?;
                        Ok(RawBson::DbPointer(RawDbPointer {
                            namespace: body.ns,
                            id: body.id,
                        }))
                    }
                    RAW_DOCUMENT_NEWTYPE => {
                        let bson = map.next_value::<&[u8]>()?;
                        let doc = RawDocument::new(bson).map_err(SerdeError::custom)?;
                        Ok(RawBson::Document(doc))
                    }
                    RAW_ARRAY_NEWTYPE => {
                        let bson = map.next_value::<&[u8]>()?;
                        let doc = RawDocument::new(bson).map_err(SerdeError::custom)?;
                        Ok(RawBson::Array(RawArray::from_doc(doc)))
                    }
                    k => Err(SerdeError::custom(format!(
                        "can't deserialize RawBson from map, key={}",
                        k
                    ))),
                }
            }
        }

        deserializer.deserialize_newtype_struct(RAW_BSON_NEWTYPE, RawBsonVisitor)
    }
}

impl<'a> TryFrom<RawBson<'a>> for Bson {
    type Error = Error;

    fn try_from(rawbson: RawBson<'a>) -> Result<Bson> {
        Ok(match rawbson {
            RawBson::Double(d) => Bson::Double(d),
            RawBson::String(s) => Bson::String(s.to_string()),
            RawBson::Document(rawdoc) => {
                let doc = rawdoc.try_into()?;
                Bson::Document(doc)
            }
            RawBson::Array(rawarray) => {
                let mut items = Vec::new();
                for v in rawarray {
                    let bson: Bson = v?.try_into()?;
                    items.push(bson);
                }
                Bson::Array(items)
            }
            RawBson::Binary(rawbson) => {
                let RawBinary {
                    subtype,
                    bytes: data,
                } = rawbson;
                Bson::Binary(crate::Binary {
                    subtype,
                    bytes: data.to_vec(),
                })
            }
            RawBson::ObjectId(rawbson) => Bson::ObjectId(rawbson),
            RawBson::Boolean(rawbson) => Bson::Boolean(rawbson),
            RawBson::DateTime(rawbson) => Bson::DateTime(rawbson),
            RawBson::Null => Bson::Null,
            RawBson::RegularExpression(rawregex) => Bson::RegularExpression(crate::Regex::new(
                rawregex.pattern.to_string(),
                rawregex.options.to_string(),
            )),
            RawBson::JavaScriptCode(rawbson) => Bson::JavaScriptCode(rawbson.to_string()),
            RawBson::Int32(rawbson) => Bson::Int32(rawbson),
            RawBson::Timestamp(rawbson) => Bson::Timestamp(rawbson),
            RawBson::Int64(rawbson) => Bson::Int64(rawbson),
            RawBson::Undefined => Bson::Undefined,
            RawBson::DbPointer(rawbson) => Bson::DbPointer(DbPointer {
                namespace: rawbson.namespace.to_string(),
                id: rawbson.id,
            }),
            RawBson::Symbol(rawbson) => Bson::Symbol(rawbson.to_string()),
            RawBson::JavaScriptCodeWithScope(rawbson) => {
                Bson::JavaScriptCodeWithScope(crate::JavaScriptCodeWithScope {
                    code: rawbson.code.to_string(),
                    scope: rawbson.scope.try_into()?,
                })
            }
            RawBson::Decimal128(rawbson) => Bson::Decimal128(rawbson),
            RawBson::MaxKey => Bson::MaxKey,
            RawBson::MinKey => Bson::MinKey,
        })
    }
}

/// A BSON binary value referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawBinary<'a> {
    pub(crate) subtype: BinarySubtype,
    pub(crate) bytes: &'a [u8],
}

impl<'a> RawBinary<'a> {
    /// Gets the subtype of the binary value.
    pub fn subtype(self) -> BinarySubtype {
        self.subtype
    }

    /// Gets the contained bytes of the binary value.
    pub fn as_bytes(self) -> &'a [u8] {
        self.bytes
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

/// A BSON regex referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawRegex<'a> {
    pub(super) pattern: &'a str,
    pub(super) options: &'a str,
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
