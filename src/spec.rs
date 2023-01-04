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

//! Constants derived from the [BSON Specification Version 1.1](http://bsonspec.org/spec.html).

use std::convert::From;

const ELEMENT_TYPE_FLOATING_POINT: u8 = 0x01;
const ELEMENT_TYPE_UTF8_STRING: u8 = 0x02;
const ELEMENT_TYPE_EMBEDDED_DOCUMENT: u8 = 0x03;
const ELEMENT_TYPE_ARRAY: u8 = 0x04;
const ELEMENT_TYPE_BINARY: u8 = 0x05;
const ELEMENT_TYPE_UNDEFINED: u8 = 0x06; // Deprecated
const ELEMENT_TYPE_OBJECT_ID: u8 = 0x07;
const ELEMENT_TYPE_BOOLEAN: u8 = 0x08;
const ELEMENT_TYPE_UTC_DATETIME: u8 = 0x09;
const ELEMENT_TYPE_NULL_VALUE: u8 = 0x0A;
const ELEMENT_TYPE_REGULAR_EXPRESSION: u8 = 0x0B;
const ELEMENT_TYPE_DBPOINTER: u8 = 0x0C; // Deprecated
const ELEMENT_TYPE_JAVASCRIPT_CODE: u8 = 0x0D;
const ELEMENT_TYPE_SYMBOL: u8 = 0x0E; // Deprecated
const ELEMENT_TYPE_JAVASCRIPT_CODE_WITH_SCOPE: u8 = 0x0F;
const ELEMENT_TYPE_32BIT_INTEGER: u8 = 0x10;
const ELEMENT_TYPE_TIMESTAMP: u8 = 0x11;
const ELEMENT_TYPE_64BIT_INTEGER: u8 = 0x12;
#[allow(unused)]
const ELEMENT_TYPE_128BIT_DECIMAL: u8 = 0x13;
const ELEMENT_TYPE_MINKEY: u8 = 0xFF;
const ELEMENT_TYPE_MAXKEY: u8 = 0x7F;

const BINARY_SUBTYPE_GENERIC: u8 = 0x00;
const BINARY_SUBTYPE_FUNCTION: u8 = 0x01;
const BINARY_SUBTYPE_BINARY_OLD: u8 = 0x02;
const BINARY_SUBTYPE_UUID_OLD: u8 = 0x03;
const BINARY_SUBTYPE_UUID: u8 = 0x04;
const BINARY_SUBTYPE_MD5: u8 = 0x05;
const BINARY_SUBTYPE_ENCRYPTED: u8 = 0x06;
const BINARY_SUBTYPE_COLUMN: u8 = 0x07;
const BINARY_SUBTYPE_USER_DEFINED: u8 = 0x80;

/// All available BSON element types.
///
/// Not all element types are representable by the [`Bson`](crate::Bson) type.
#[repr(u8)]
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum ElementType {
    /// 64-bit binary floating point
    Double = ELEMENT_TYPE_FLOATING_POINT,
    /// UTF-8 string
    String = ELEMENT_TYPE_UTF8_STRING,
    /// Embedded document
    EmbeddedDocument = ELEMENT_TYPE_EMBEDDED_DOCUMENT,
    /// Array
    Array = ELEMENT_TYPE_ARRAY,
    /// Binary data
    Binary = ELEMENT_TYPE_BINARY,
    /// Deprecated. Undefined (value)
    Undefined = ELEMENT_TYPE_UNDEFINED,
    /// [ObjectId](http://dochub.mongodb.org/core/objectids)
    ObjectId = ELEMENT_TYPE_OBJECT_ID,
    /// Bool value
    Boolean = ELEMENT_TYPE_BOOLEAN,
    /// UTC datetime
    DateTime = ELEMENT_TYPE_UTC_DATETIME,
    /// Null value
    Null = ELEMENT_TYPE_NULL_VALUE,
    /// Regular expression - The first cstring is the regex pattern, the second is the regex
    /// options string. Options are identified by characters, which must be stored in
    /// alphabetical order. Valid options are 'i' for case insensitive matching, 'm' for
    /// multiline matching, 'x' for verbose mode, 'l' to make \w, \W, etc. locale dependent,
    /// 's' for dotall mode ('.' matches everything), and 'u' to make \w, \W, etc. match
    /// unicode.
    RegularExpression = ELEMENT_TYPE_REGULAR_EXPRESSION,
    /// Deprecated.
    DbPointer = ELEMENT_TYPE_DBPOINTER,
    /// JavaScript code
    JavaScriptCode = ELEMENT_TYPE_JAVASCRIPT_CODE,
    /// Deprecated.
    Symbol = ELEMENT_TYPE_SYMBOL,
    /// JavaScript code w/ scope
    JavaScriptCodeWithScope = ELEMENT_TYPE_JAVASCRIPT_CODE_WITH_SCOPE,
    /// 32-bit integer
    Int32 = ELEMENT_TYPE_32BIT_INTEGER,
    /// Timestamp
    Timestamp = ELEMENT_TYPE_TIMESTAMP,
    /// 64-bit integer
    Int64 = ELEMENT_TYPE_64BIT_INTEGER,
    /// [128-bit decimal floating point](https://github.com/mongodb/specifications/blob/master/source/bson-decimal128/decimal128.rst)
    Decimal128 = ELEMENT_TYPE_128BIT_DECIMAL,
    MaxKey = ELEMENT_TYPE_MAXKEY,
    MinKey = ELEMENT_TYPE_MINKEY,
}

impl ElementType {
    /// Attempt to convert from a `u8`.
    #[inline]
    pub fn from(tag: u8) -> Option<ElementType> {
        use self::ElementType::*;
        Some(match tag {
            ELEMENT_TYPE_FLOATING_POINT => Self::Double,
            ELEMENT_TYPE_UTF8_STRING => Self::String,
            ELEMENT_TYPE_EMBEDDED_DOCUMENT => EmbeddedDocument,
            ELEMENT_TYPE_ARRAY => Array,
            ELEMENT_TYPE_BINARY => Binary,
            ELEMENT_TYPE_UNDEFINED => Undefined,
            ELEMENT_TYPE_OBJECT_ID => ObjectId,
            ELEMENT_TYPE_BOOLEAN => Boolean,
            ELEMENT_TYPE_UTC_DATETIME => Self::DateTime,
            ELEMENT_TYPE_NULL_VALUE => Self::Null,
            ELEMENT_TYPE_REGULAR_EXPRESSION => RegularExpression,
            ELEMENT_TYPE_DBPOINTER => DbPointer,
            ELEMENT_TYPE_JAVASCRIPT_CODE => JavaScriptCode,
            ELEMENT_TYPE_SYMBOL => Symbol,
            ELEMENT_TYPE_JAVASCRIPT_CODE_WITH_SCOPE => JavaScriptCodeWithScope,
            ELEMENT_TYPE_32BIT_INTEGER => Int32,
            ELEMENT_TYPE_TIMESTAMP => Timestamp,
            ELEMENT_TYPE_64BIT_INTEGER => Int64,
            ELEMENT_TYPE_128BIT_DECIMAL => Decimal128,
            ELEMENT_TYPE_MAXKEY => MaxKey,
            ELEMENT_TYPE_MINKEY => MinKey,
            _ => return None,
        })
    }
}

/// The available binary subtypes, plus a user-defined slot.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[non_exhaustive]
pub enum BinarySubtype {
    Generic,
    Function,
    BinaryOld,
    UuidOld,
    Uuid,
    Md5,
    Encrypted,
    Column,
    UserDefined(u8),
    Reserved(u8),
}

impl From<BinarySubtype> for u8 {
    #[inline]
    fn from(t: BinarySubtype) -> u8 {
        match t {
            BinarySubtype::Generic => BINARY_SUBTYPE_GENERIC,
            BinarySubtype::Function => BINARY_SUBTYPE_FUNCTION,
            BinarySubtype::BinaryOld => BINARY_SUBTYPE_BINARY_OLD,
            BinarySubtype::UuidOld => BINARY_SUBTYPE_UUID_OLD,
            BinarySubtype::Uuid => BINARY_SUBTYPE_UUID,
            BinarySubtype::Md5 => BINARY_SUBTYPE_MD5,
            BinarySubtype::Encrypted => BINARY_SUBTYPE_ENCRYPTED,
            BinarySubtype::Column => BINARY_SUBTYPE_COLUMN,
            BinarySubtype::UserDefined(x) => x,
            BinarySubtype::Reserved(x) => x,
        }
    }
}

impl From<u8> for BinarySubtype {
    #[inline]
    fn from(t: u8) -> BinarySubtype {
        match t {
            BINARY_SUBTYPE_GENERIC => BinarySubtype::Generic,
            BINARY_SUBTYPE_FUNCTION => BinarySubtype::Function,
            BINARY_SUBTYPE_BINARY_OLD => BinarySubtype::BinaryOld,
            BINARY_SUBTYPE_UUID_OLD => BinarySubtype::UuidOld,
            BINARY_SUBTYPE_UUID => BinarySubtype::Uuid,
            BINARY_SUBTYPE_MD5 => BinarySubtype::Md5,
            BINARY_SUBTYPE_ENCRYPTED => BinarySubtype::Encrypted,
            BINARY_SUBTYPE_COLUMN => BinarySubtype::Column,
            _ if t < BINARY_SUBTYPE_USER_DEFINED => BinarySubtype::Reserved(t),
            _ => BinarySubtype::UserDefined(t),
        }
    }
}
