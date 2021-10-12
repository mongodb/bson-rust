use std::convert::{TryFrom, TryInto};

use super::{Error, RawArray, RawDocumentRef, Result};
use crate::{
    oid::{self, ObjectId},
    spec::{BinarySubtype, ElementType},
    Bson,
    DbPointer,
    Decimal128,
    Timestamp,
};

/// A BSON value referencing raw bytes stored elsewhere.
#[derive(Clone, Copy)]
pub enum RawBson<'a> {
    /// 64-bit binary floating point
    Double(f64),
    /// UTF-8 string
    String(&'a str),
    /// Array
    Array(&'a RawArray),
    /// Embedded document
    Document(&'a RawDocumentRef),
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

// #[derive(Clone, Copy, Debug)]
// pub struct RawBson<'a> {
//     element_type: ElementType,
//     data: &'a [u8],
// }

impl<'a> RawBson<'a> {
    // pub(super) fn new(element_type: ElementType, data: &'a [u8]) -> RawBson<'a> {
    //     RawBson { element_type, data }
    // }

    /// Gets the type of the value.
    pub fn element_type(self) -> ElementType {
        // self.element_type
        todo!()
    }

    // /// Gets a reference to the raw bytes of the value.
    // pub fn as_bytes(self) -> &'a [u8] {
    //     self.data
    // }

    // fn validate_type(self, expected: ElementType) -> Result<()> {
    //     if self.element_type != expected {
    //         return Err(Error {
    //             key: None,
    //             kind: ErrorKind::UnexpectedType {
    //                 actual: self.element_type,
    //                 expected,
    //             },
    //         });
    //     }
    //     Ok(())
    // }

    /// Gets the f64 that's referenced or returns an error if the value isn't a BSON double.
    pub fn as_f64(self) -> Option<f64> {
        match self {
            RawBson::Double(d) => Some(d),
            _ => None,
        }
    }

    /// If `Bson` is `String`, return its value as a `&str`. Returns `None` otherwise
    pub fn as_str(self) -> Option<&'a str> {
        match self {
            RawBson::String(s) => Some(s),
            _ => None,
        }
    }

    /// If `Bson` is `Array`, return its value. Returns `None` otherwise
    pub fn as_array(self) -> Option<&'a RawArray> {
        match self {
            RawBson::Array(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Document`, return its value. Returns `None` otherwise
    pub fn as_document(self) -> Option<&'a RawDocumentRef> {
        match self {
            RawBson::Document(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Bool`, return its value. Returns `None` otherwise
    pub fn as_bool(self) -> Option<bool> {
        match self {
            RawBson::Boolean(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `I32`, return its value. Returns `None` otherwise
    pub fn as_i32(self) -> Option<i32> {
        match self {
            RawBson::Int32(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `I64`, return its value. Returns `None` otherwise
    pub fn as_i64(self) -> Option<i64> {
        match self {
            RawBson::Int64(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Objectid`, return its value. Returns `None` otherwise
    pub fn as_object_id(self) -> Option<oid::ObjectId> {
        match self {
            RawBson::ObjectId(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Binary`, return its value. Returns `None` otherwise
    pub fn as_binary(self) -> Option<RawBinary<'a>> {
        match self {
            RawBson::Binary(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Regex`, return its value. Returns `None` otherwise
    pub fn as_regex(self) -> Option<RawRegex<'a>> {
        match self {
            RawBson::RegularExpression(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `DateTime`, return its value. Returns `None` otherwise
    pub fn as_datetime(self) -> Option<crate::DateTime> {
        match self {
            RawBson::DateTime(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Symbol`, return its value. Returns `None` otherwise
    pub fn as_symbol(self) -> Option<&'a str> {
        match self {
            RawBson::Symbol(v) => Some(v),
            _ => None,
        }
    }

    /// If `Bson` is `Timestamp`, return its value. Returns `None` otherwise
    pub fn as_timestamp(self) -> Option<Timestamp> {
        match self {
            RawBson::Timestamp(timestamp) => Some(timestamp),
            _ => None,
        }
    }

    /// If `Bson` is `Null`, return its value. Returns `None` otherwise
    pub fn as_null(self) -> Option<()> {
        match self {
            RawBson::Null => Some(()),
            _ => None,
        }
    }

    pub fn as_db_pointer(self) -> Option<u8> {
        // match self {
        //     Bson::DbPointer(db_pointer) => Some(db_pointer),
        //     _ => None,
        // }
        todo!()
    }

    /// If `Bson` is `JavaScriptCode`, return its value. Returns `None` otherwise
    pub fn as_javascript(self) -> Option<&'a str> {
        match self {
            RawBson::JavaScriptCode(s) => Some(s),
            _ => None,
        }
    }

    /// If `Bson` is `JavaScriptCodeWithScope`, return its value. Returns `None` otherwise
    pub fn as_javascript_with_scope(self) -> Option<RawJavaScriptCodeWithScope<'a>> {
        match self {
            RawBson::JavaScriptCodeWithScope(s) => Some(s),
            _ => None,
        }
    }
}

// impl<'a> RawBson<'a> {

//     /// Gets the string that's referenced or returns an error if the value isn't a BSON string.
//     pub fn as_str(self) -> Result<&'a str> {
//         self.validate_type(ElementType::String)?;
//         read_lenencoded(self.data)
//     }

//     /// Gets the document that's referenced or returns an error if the value isn't a BSON
// document.     pub fn as_document(self) -> Result<&'a RawDocumentRef> {
//         self.validate_type(ElementType::EmbeddedDocument)?;
//         RawDocumentRef::new(self.data)
//     }

//     /// Gets the array that's referenced or returns an error if the value isn't a BSON array.
//     pub fn as_array(self) -> Result<&'a RawArray> {
//         self.validate_type(ElementType::Array)?;
//         RawArray::new(self.data)
//     }

//     /// Gets the BSON binary value that's referenced or returns an error if the value a BSON
// binary.     pub fn as_binary(self) -> Result<RawBinary<'a>> {
//         self.validate_type(ElementType::Binary)?;

//         let length = i32_from_slice(&self.data[0..4])?;
//         let subtype = BinarySubtype::from(self.data[4]);
//         if self.data.len() as i32 != length + 5 {
//             return Err(Error {
//                 key: None,
//                 kind: ErrorKind::MalformedValue {
//                     message: "binary bson has wrong declared length".into(),
//                 },
//             });
//         }
//         let data = match subtype {
//             BinarySubtype::BinaryOld => {
//                 if length < 4 {
//                     return Err(Error::new_without_key(ErrorKind::MalformedValue {
//                         message: "old binary subtype has no inner declared length".into(),
//                     }));
//                 }
//                 let oldlength = i32_from_slice(&self.data[5..9])?;
//                 if oldlength + 4 != length {
//                     return Err(Error::new_without_key(ErrorKind::MalformedValue {
//                         message: "old binary subtype has wrong inner declared length".into(),
//                     }));
//                 }
//                 &self.data[9..]
//             }
//             _ => &self.data[5..],
//         };
//         Ok(RawBinary::new(subtype, data))
//     }

//     /// Gets the ObjectId that's referenced or returns an error if the value isn't a BSON
// ObjectId.     pub fn as_object_id(self) -> Result<ObjectId> {
//         self.validate_type(ElementType::ObjectId)?;
//         Ok(ObjectId::from_bytes(self.data.try_into().map_err(
//             |_| {
//                 Error::new_without_key(ErrorKind::MalformedValue {
//                     message: "object id should be 12 bytes long".into(),
//                 })
//             },
//         )?))
//     }

//     /// Gets the boolean that's referenced or returns an error if the value isn't a BSON boolean.
//     pub fn as_bool(self) -> Result<bool> {
//         self.validate_type(ElementType::Boolean)?;
//         if self.data.len() != 1 {
//             Err(Error::new_without_key(ErrorKind::MalformedValue {
//                 message: "boolean has length != 1".into(),
//             }))
//         } else {
//             read_bool(self.data).map_err(|e| {
//                 Error::new_without_key(ErrorKind::MalformedValue {
//                     message: e.to_string(),
//                 })
//             })
//         }
//     }

//     /// Gets the DateTime that's referenced or returns an error if the value isn't a BSON
// DateTime.     pub fn as_datetime(self) -> Result<DateTime> {
//         self.validate_type(ElementType::DateTime)?;
//         let millis = i64_from_slice(self.data)?;
//         Ok(DateTime::from_millis(millis))
//     }

//     /// Gets the regex that's referenced or returns an error if the value isn't a BSON regex.
//     pub fn as_regex(self) -> Result<RawRegex<'a>> {
//         self.validate_type(ElementType::RegularExpression)?;
//         RawRegex::new(self.data)
//     }

//     /// Gets the BSON JavaScript code that's referenced or returns an error if the value isn't
// BSON     /// JavaScript code.
//     pub fn as_javascript(self) -> Result<&'a str> {
//         self.validate_type(ElementType::JavaScriptCode)?;
//         read_lenencoded(self.data)
//     }

//     /// Gets the symbol that's referenced or returns an error if the value isn't a BSON symbol.
//     pub fn as_symbol(self) -> Result<&'a str> {
//         self.validate_type(ElementType::Symbol)?;
//         read_lenencoded(self.data)
//     }

//     /// Gets the BSON JavaScript code with scope that's referenced or returns an error if the
// value     /// isn't BSON JavaScript code with scope.
//     pub fn as_javascript_with_scope(self) -> Result<RawJavaScriptCodeWithScope<'a>> {
//         self.validate_type(ElementType::JavaScriptCodeWithScope)?;
//         let length = i32_from_slice(&self.data[..4])?;

//         if (self.data.len() as i32) != length {
//             return Err(Error::new_without_key(ErrorKind::MalformedValue {
//                 message: format!("TODO: Java"),
//             }));
//         }

//         let code = read_lenencoded(&self.data[4..])?;
//         let scope = RawDocumentRef::new(&self.data[9 + code.len()..])?;

//         Ok(RawJavaScriptCodeWithScope { code, scope })
//     }

//     /// Gets the timestamp that's referenced or returns an error if the value isn't a BSON
//     /// timestamp.
//     pub fn as_timestamp(self) -> Result<RawTimestamp<'a>> {
//         self.validate_type(ElementType::Timestamp)?;
//         assert_eq!(self.data.len(), 8);
//         Ok(RawTimestamp { data: self.data })
//     }

//     /// Gets the i32 that's referenced or returns an error if the value isn't a BSON int32.
//     pub fn as_i32(self) -> Result<i32> {
//         self.validate_type(ElementType::Int32)?;
//         i32_from_slice(self.data)
//     }

//     /// Gets the i64 that's referenced or returns an error if the value isn't a BSON int64.
//     pub fn as_i64(self) -> Result<i64> {
//         self.validate_type(ElementType::Int64)?;
//         i64_from_slice(self.data)
//     }

//     /// Gets the decimal that's referenced or returns an error if the value isn't a BSON
// Decimal128.     pub fn as_decimal128(self) -> Result<Decimal128> {
//         self.validate_type(ElementType::Decimal128)?;
//         let bytes: [u8; 128 / 8] = self.data.try_into().map_err(|_| {
//             Error::new_without_key(ErrorKind::MalformedValue {
//                 message: format!("decimal128 value has invalid length: {}", self.data.len()),
//             })
//         })?;
//         Ok(Decimal128::from_bytes(bytes))
//     }

//     /// Gets the null value that's referenced or returns an error if the value isn't a BSON null.
//     pub fn as_null(self) -> Result<()> {
//         self.validate_type(ElementType::Null)
//     }
// }

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
                let RawBinary { subtype, data } = rawbson;
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
    pub(super) data: &'a [u8],
}

impl<'a> RawBinary<'a> {
    /// Gets the subtype of the binary value.
    pub fn subtype(self) -> BinarySubtype {
        self.subtype
    }

    /// Gets the contained bytes of the binary value.
    pub fn as_bytes(self) -> &'a [u8] {
        self.data
    }
}

/// A BSON regex referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug)]
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
#[derive(Clone, Copy, Debug)]
pub struct RawJavaScriptCodeWithScope<'a> {
    pub(crate) code: &'a str,
    pub(crate) scope: &'a RawDocumentRef,
}

impl<'a> RawJavaScriptCodeWithScope<'a> {
    /// Gets the code in the value.
    pub fn code(self) -> &'a str {
        self.code
    }

    /// Gets the scope in the value.
    pub fn scope(self) -> &'a RawDocumentRef {
        self.scope
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RawDbPointer<'a> {
    pub(crate) namespace: &'a str,
    pub(crate) id: ObjectId,
}
