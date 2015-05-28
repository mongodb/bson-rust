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

//! BSON Specification Version 1.0
/// http://bsonspec.org/spec.html

use std::convert::From;

pub const ELEMENT_TYPE_FLOATING_POINT               : u8 = 0x01;
pub const ELEMENT_TYPE_UTF8_STRING                  : u8 = 0x02;
pub const ELEMENT_TYPE_EMBEDDED_DOCUMENT            : u8 = 0x03;
pub const ELEMENT_TYPE_ARRAY                        : u8 = 0x04;
pub const ELEMENT_TYPE_BINARY                       : u8 = 0x05;
pub const ELEMENT_TYPE_UNDEFINED                    : u8 = 0x06; // Deprecated
pub const ELEMENT_TYPE_OBJECT_ID                    : u8 = 0x07;
pub const ELEMENT_TYPE_BOOLEAN                      : u8 = 0x08;
pub const ELEMENT_TYPE_UTC_DATETIME                 : u8 = 0x09;
pub const ELEMENT_TYPE_NULL_VALUE                   : u8 = 0x0A;
pub const ELEMENT_TYPE_REGULAR_EXPRESSION           : u8 = 0x0B;
pub const ELEMENT_TYPE_DBPOINTER                    : u8 = 0x0C; // Deprecated
pub const ELEMENT_TYPE_JAVASCRIPT_CODE              : u8 = 0x0D;
pub const ELEMENT_TYPE_DEPRECATED                   : u8 = 0x0E;
pub const ELEMENT_TYPE_JAVASCRIPT_CODE_WITH_SCOPE   : u8 = 0x0F;
pub const ELEMENT_TYPE_32BIT_INTEGER                : u8 = 0x10;
pub const ELEMENT_TYPE_TIMESTAMP                    : u8 = 0x11;
pub const ELEMENT_TYPE_64BIT_INTEGER                : u8 = 0x12;
pub const ELEMENT_TYPE_MINKEY                       : u8 = 0xFF;
pub const ELEMENT_TYPE_MAXKEY                       : u8 = 0x7F;

pub const BINARY_SUBTYPE_GENERIC                    : u8 = 0x00;
pub const BINARY_SUBTYPE_FUNCTION                   : u8 = 0x01;
pub const BINARY_SUBTYPE_BINARY_OLD                 : u8 = 0x02;
pub const BINARY_SUBTYPE_UUID_OLD                   : u8 = 0x03;
pub const BINARY_SUBTYPE_UUID                       : u8 = 0x04;
pub const BINARY_SUBTYPE_MD5                        : u8 = 0x05;

#[repr(u8)]
#[derive(Debug, Eq, PartialEq)]
pub enum ElementType {
    FloatingPoint               = ELEMENT_TYPE_FLOATING_POINT,
    Utf8String                  = ELEMENT_TYPE_UTF8_STRING,
    EmbeddedDocument            = ELEMENT_TYPE_EMBEDDED_DOCUMENT,
    Array                       = ELEMENT_TYPE_ARRAY,
    Binary                      = ELEMENT_TYPE_BINARY,
    #[warn(deprecated)]
    Undefined                   = ELEMENT_TYPE_UNDEFINED,
    ObjectId                    = ELEMENT_TYPE_OBJECT_ID,
    Boolean                     = ELEMENT_TYPE_BOOLEAN,
    UtcDatetime                 = ELEMENT_TYPE_UTC_DATETIME,
    NullValue                   = ELEMENT_TYPE_NULL_VALUE,
    RegularExpression           = ELEMENT_TYPE_REGULAR_EXPRESSION,
    #[warn(deprecated)]
    DbPointer                   = ELEMENT_TYPE_DBPOINTER,
    JavaScriptCode              = ELEMENT_TYPE_JAVASCRIPT_CODE,
    Deprecated                  = ELEMENT_TYPE_DEPRECATED,
    JavaScriptCodeWithScope     = ELEMENT_TYPE_JAVASCRIPT_CODE_WITH_SCOPE,
    Integer32Bit                = ELEMENT_TYPE_32BIT_INTEGER,
    TimeStamp                   = ELEMENT_TYPE_TIMESTAMP,
    Integer64Bit                = ELEMENT_TYPE_64BIT_INTEGER,

    MaxKey                      = ELEMENT_TYPE_MAXKEY,
    MinKey                      = ELEMENT_TYPE_MINKEY,
}

impl ElementType {
	#[inline]
	pub fn from(tag: u8) -> Option<ElementType> {
		use self::ElementType::*;
		Some(match tag {
			ELEMENT_TYPE_FLOATING_POINT => FloatingPoint,
			ELEMENT_TYPE_UTF8_STRING => Utf8String,
			ELEMENT_TYPE_EMBEDDED_DOCUMENT => EmbeddedDocument,
			ELEMENT_TYPE_ARRAY => Array,
			ELEMENT_TYPE_BINARY => Binary,
			ELEMENT_TYPE_UNDEFINED => Undefined,
			ELEMENT_TYPE_OBJECT_ID => ObjectId,
			ELEMENT_TYPE_BOOLEAN => Boolean,
			ELEMENT_TYPE_UTC_DATETIME => UtcDatetime,
			ELEMENT_TYPE_NULL_VALUE => NullValue,
			ELEMENT_TYPE_REGULAR_EXPRESSION => RegularExpression,
			ELEMENT_TYPE_DBPOINTER => DbPointer,
			ELEMENT_TYPE_JAVASCRIPT_CODE => JavaScriptCode,
			ELEMENT_TYPE_DEPRECATED => Deprecated,
			ELEMENT_TYPE_JAVASCRIPT_CODE_WITH_SCOPE => JavaScriptCodeWithScope,
			ELEMENT_TYPE_32BIT_INTEGER => Integer32Bit,
			ELEMENT_TYPE_TIMESTAMP => TimeStamp,
			ELEMENT_TYPE_64BIT_INTEGER => Integer64Bit,
			ELEMENT_TYPE_MAXKEY => MaxKey,
			ELEMENT_TYPE_MINKEY => MinKey,
			_ => return None
		})
	}
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum BinarySubtype {
    Generic,
    Function,
    BinaryOld,
    UuidOld,
    Uuid,
    Md5,
    UserDefined(u8),
}

impl From<BinarySubtype> for u8 {
    #[inline]
    fn from(t : BinarySubtype) -> u8 {
        match t {
            BinarySubtype::Generic         => BINARY_SUBTYPE_GENERIC,
            BinarySubtype::Function        => BINARY_SUBTYPE_FUNCTION,
            BinarySubtype::BinaryOld       => BINARY_SUBTYPE_BINARY_OLD,
            BinarySubtype::UuidOld         => BINARY_SUBTYPE_UUID_OLD,
            BinarySubtype::Uuid            => BINARY_SUBTYPE_UUID,
            BinarySubtype::Md5             => BINARY_SUBTYPE_MD5,
            BinarySubtype::UserDefined(x)  => x,
        }
    }
}

impl From<u8> for BinarySubtype {
    #[inline]
    fn from(t : u8) -> BinarySubtype {
        match t {
            BINARY_SUBTYPE_GENERIC      => BinarySubtype::Generic,
            BINARY_SUBTYPE_FUNCTION     => BinarySubtype::Function,
            BINARY_SUBTYPE_BINARY_OLD   => BinarySubtype::BinaryOld,
            BINARY_SUBTYPE_UUID_OLD     => BinarySubtype::UuidOld,
            BINARY_SUBTYPE_UUID         => BinarySubtype::Uuid,
            BINARY_SUBTYPE_MD5          => BinarySubtype::Md5,
            _                           => BinarySubtype::UserDefined(t),
        }
    }
}
