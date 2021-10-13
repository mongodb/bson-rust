use std::convert::{TryFrom, TryInto};

use super::{Error, RawArr, RawDoc, Result};
use crate::{
    oid::{self, ObjectId},
    spec::{BinarySubtype, ElementType},
    Bson,
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
    Array(&'a RawArr),
    /// Embedded document
    Document(&'a RawDoc),
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

    /// Gets the [`crate::raw::RawArr`] that's referenced or returns `None` if the referenced value
    /// isn't a BSON array.
    pub fn as_array(self) -> Option<&'a RawArr> {
        match self {
            RawBson::Array(v) => Some(v),
            _ => None,
        }
    }

    /// Gets the [`crate::raw::RawDoc`] that's referenced or returns `None` if the referenced value
    /// isn't a BSON document.
    pub fn as_document(self) -> Option<&'a RawDoc> {
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
    pub(super) subtype: BinarySubtype,
    pub(super) bytes: &'a [u8],
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

/// A BSON "code with scope" value referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawJavaScriptCodeWithScope<'a> {
    pub(crate) code: &'a str,
    pub(crate) scope: &'a RawDoc,
}

impl<'a> RawJavaScriptCodeWithScope<'a> {
    /// Gets the code in the value.
    pub fn code(self) -> &'a str {
        self.code
    }

    /// Gets the scope in the value.
    pub fn scope(self) -> &'a RawDoc {
        self.scope
    }
}

/// A BSON DB pointer value referencing raw bytes stored elesewhere.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawDbPointer<'a> {
    pub(crate) namespace: &'a str,
    pub(crate) id: ObjectId,
}
