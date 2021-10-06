use std::convert::{TryFrom, TryInto};

// use chrono::{DateTime, TimeZone, Utc};
use crate::{DateTime, Decimal128};

#[cfg(feature = "decimal128")]
use super::d128_from_slice;
use super::{
    i32_from_slice,
    i64_from_slice,
    read_lenencoded,
    read_nullterminated,
    u32_from_slice,
    Error,
    RawArray,
    RawDocumentRef,
    Result,
};
use crate::{
    oid::ObjectId,
    spec::{BinarySubtype, ElementType},
    Bson,
};

/// A BSON value referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug)]
pub struct RawBson<'a> {
    element_type: ElementType,
    data: &'a [u8],
}

impl<'a> RawBson<'a> {
    pub(super) fn new(element_type: ElementType, data: &'a [u8]) -> RawBson<'a> {
        RawBson { element_type, data }
    }

    /// Gets the type of the value.
    pub fn element_type(self) -> ElementType {
        self.element_type
    }

    /// Gets a reference to the raw bytes of the value.
    pub fn as_bytes(self) -> &'a [u8] {
        self.data
    }

    fn validate_type(self, expected: ElementType) -> Result<()> {
        if self.element_type != expected {
            return Err(Error::UnexpectedType {
                actual: self.element_type,
                expected,
            });
        }
        Ok(())
    }

    /// Gets the f64 that's referenced or returns an error if the value isn't a BSON double.
    pub fn as_f64(self) -> Result<f64> {
        self.validate_type(ElementType::Double)?;
        Ok(f64::from_bits(u64::from_le_bytes(
            self.data.try_into().map_err(|_| Error::MalformedValue {
                message: "f64 should be 8 bytes long".into(),
            })?,
        )))
    }

    /// Gets the string that's referenced or returns an error if the value isn't a BSON string.
    pub fn as_str(self) -> Result<&'a str> {
        self.validate_type(ElementType::String)?;
        read_lenencoded(self.data)
    }

    /// Gets the document that's referenced or returns an error if the value isn't a BSON document.
    pub fn as_document(self) -> Result<&'a RawDocumentRef> {
        self.validate_type(ElementType::EmbeddedDocument)?;
        RawDocumentRef::new(self.data)
    }

    /// Gets the array that's referenced or returns an error if the value isn't a BSON array.
    pub fn as_array(self) -> Result<&'a RawArray> {
        self.validate_type(ElementType::Array)?;
        RawArray::new(self.data)
    }

    /// Gets the BSON binary value that's referenced or returns an error if the value a BSON binary.
    pub fn as_binary(self) -> Result<RawBinary<'a>> {
        self.validate_type(ElementType::Binary)?;

        let length = i32_from_slice(&self.data[0..4])?;
        let subtype = BinarySubtype::from(self.data[4]); // TODO: This mishandles reserved values
        if self.data.len() as i32 != length + 5 {
            return Err(Error::MalformedValue {
                message: "binary bson has wrong declared length".into(),
            });
        }
        let data = match subtype {
            BinarySubtype::BinaryOld => {
                if length < 4 {
                    return Err(Error::MalformedValue {
                        message: "old binary subtype has no inner declared length".into(),
                    });
                }
                let oldlength = i32_from_slice(&self.data[5..9])?;
                if oldlength + 4 != length {
                    return Err(Error::MalformedValue {
                        message: "old binary subtype has wrong inner declared length".into(),
                    });
                }
                &self.data[9..]
            }
            _ => &self.data[5..],
        };
        Ok(RawBinary::new(subtype, data))
    }

    /// Gets the ObjectId that's referenced or returns an error if the value isn't a BSON ObjectId.
    pub fn as_object_id(self) -> Result<ObjectId> {
        self.validate_type(ElementType::ObjectId)?;
        Ok(ObjectId::from_bytes(self.data.try_into().map_err(
            |_| Error::MalformedValue {
                message: "object id should be 12 bytes long".into(),
            },
        )?))
    }

    /// Gets the boolean that's referenced or returns an error if the value isn't a BSON boolean.
    pub fn as_bool(self) -> Result<bool> {
        self.validate_type(ElementType::Boolean)?;
        if self.data.len() != 1 {
            Err(Error::MalformedValue {
                message: "boolean has length != 1".into(),
            })
        } else {
            match self.data[0] {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(Error::MalformedValue {
                    message: "boolean value was not 0 or 1".into(),
                }),
            }
        }
    }

    /// Gets the DateTime that's referenced or returns an error if the value isn't a BSON DateTime.
    pub fn as_datetime(self) -> Result<DateTime> {
        self.validate_type(ElementType::DateTime)?;
        let millis = i64_from_slice(self.data)?;
        Ok(DateTime::from_millis(millis))
    }

    /// Gets the regex that's referenced or returns an error if the value isn't a BSON regex.
    pub fn as_regex(self) -> Result<RawRegex<'a>> {
        self.validate_type(ElementType::RegularExpression)?;
        RawRegex::new(self.data)
    }

    /// Gets the BSON JavaScript code that's referenced or returns an error if the value isn't BSON
    /// JavaScript code.
    pub fn as_javascript(self) -> Result<&'a str> {
        self.validate_type(ElementType::JavaScriptCode)?;
        read_lenencoded(self.data)
    }

    /// Gets the symbol that's referenced or returns an error if the value isn't a BSON symbol.
    pub fn as_symbol(self) -> Result<&'a str> {
        self.validate_type(ElementType::Symbol)?;
        read_lenencoded(self.data)
    }

    /// Gets the BSON JavaScript code with scope that's referenced or returns an error if the value
    /// isn't BSON JavaScript code with scope.
    pub fn as_javascript_with_scope(self) -> Result<RawJavaScriptCodeWithScope<'a>> {
        self.validate_type(ElementType::JavaScriptCodeWithScope)?;
        let length = i32_from_slice(&self.data[..4])?;

        if (self.data.len() as i32) != length {
            return Err(Error::MalformedValue {
                message: "".to_string(),
            });
        }

        let code = read_lenencoded(&self.data[4..])?;
        let scope = RawDocumentRef::new(&self.data[9 + code.len()..])?;

        Ok(RawJavaScriptCodeWithScope { code, scope })
    }

    /// Gets the timestamp that's referenced or returns an error if the value isn't a BSON
    /// timestamp.
    pub fn as_timestamp(self) -> Result<RawTimestamp<'a>> {
        self.validate_type(ElementType::Timestamp)?;
        assert_eq!(self.data.len(), 8);
        Ok(RawTimestamp { data: self.data })
    }

    /// Gets the i32 that's referenced or returns an error if the value isn't a BSON int32.
    pub fn as_i32(self) -> Result<i32> {
        self.validate_type(ElementType::Int32)?;
        i32_from_slice(self.data)
    }

    /// Gets the i64 that's referenced or returns an error if the value isn't a BSON int64.
    pub fn as_i64(self) -> Result<i64> {
        self.validate_type(ElementType::Int64)?;
        i64_from_slice(self.data)
    }

    /// Gets the decimal that's referenced or returns an error if the value isn't a BSON Decimal128.
    pub fn as_decimal128(self) -> Result<Decimal128> {
        self.validate_type(ElementType::Decimal128)?;
        let bytes: [u8; 128 / 8] = self.data.try_into().map_err(|_| Error::MalformedValue {
            message: format!("decimal128 value has invalid length: {}", self.data.len()),
        })?;
        Ok(Decimal128::from_bytes(bytes))
    }

    pub fn as_null(self) -> Result<()> {
        self.validate_type(ElementType::Null)
    }
}

// TODO: finish implementation
impl<'a> TryFrom<RawBson<'a>> for Bson {
    type Error = Error;

    fn try_from(rawbson: RawBson<'a>) -> Result<Bson> {
        Ok(match rawbson.element_type {
            ElementType::Double => Bson::Double(rawbson.as_f64()?),
            ElementType::String => Bson::String(String::from(rawbson.as_str()?)),
            ElementType::EmbeddedDocument => {
                let rawdoc = rawbson.as_document()?;
                let doc = rawdoc.try_into()?;
                Bson::Document(doc)
            }
            ElementType::Array => {
                let rawarray = rawbson.as_array()?;
                let v = rawarray.try_into()?;
                Bson::Array(v)
            }
            ElementType::Binary => {
                let RawBinary { subtype, data } = rawbson.as_binary()?;
                Bson::Binary(crate::Binary {
                    subtype,
                    bytes: data.to_vec(),
                })
            }
            ElementType::ObjectId => Bson::ObjectId(rawbson.as_object_id()?),
            ElementType::Boolean => Bson::Boolean(rawbson.as_bool()?),
            ElementType::DateTime => Bson::DateTime(rawbson.as_datetime()?),
            ElementType::Null => Bson::Null,
            ElementType::RegularExpression => {
                let rawregex = rawbson.as_regex()?;
                Bson::RegularExpression(crate::Regex {
                    pattern: String::from(rawregex.pattern()),
                    options: String::from(rawregex.options()),
                })
            }
            ElementType::JavaScriptCode => {
                Bson::JavaScriptCode(String::from(rawbson.as_javascript()?))
            }
            ElementType::Int32 => Bson::Int32(rawbson.as_i32()?),
            ElementType::Timestamp => {
                // RawBson::as_timestamp() returns u64, but Bson::Timestamp expects i64
                let ts = rawbson.as_timestamp()?;
                Bson::Timestamp(crate::Timestamp {
                    time: ts.time(),
                    increment: ts.increment(),
                })
            }
            ElementType::Int64 => Bson::Int64(rawbson.as_i64()?),
            ElementType::Undefined => Bson::Null,
            ElementType::DbPointer => panic!("Uh oh. Maybe this should be a TryFrom"),
            ElementType::Symbol => Bson::Symbol(String::from(rawbson.as_symbol()?)),
            ElementType::JavaScriptCodeWithScope => {
                let RawJavaScriptCodeWithScope { code, scope } =
                    rawbson.as_javascript_with_scope()?;
                Bson::JavaScriptCodeWithScope(crate::JavaScriptCodeWithScope {
                    code: String::from(code),
                    scope: scope.try_into()?,
                })
            }
            ElementType::Decimal128 => Bson::Decimal128(rawbson.as_decimal128()?),
            ElementType::MaxKey => unimplemented!(),
            ElementType::MinKey => unimplemented!(),
        })
    }
}

/// A BSON binary value referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug)]
pub struct RawBinary<'a> {
    pub(super) subtype: BinarySubtype,
    pub(super) data: &'a [u8],
}

impl<'a> RawBinary<'a> {
    fn new(subtype: BinarySubtype, data: &'a [u8]) -> RawBinary<'a> {
        RawBinary { subtype, data }
    }

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
    pub(super) fn new(data: &'a [u8]) -> Result<RawRegex<'a>> {
        let pattern = read_nullterminated(data)?;
        let opts = read_nullterminated(&data[pattern.len() + 1..])?;
        if pattern.len() + opts.len() == data.len() - 2 {
            Ok(RawRegex {
                pattern,
                options: opts,
            })
        } else {
            Err(Error::MalformedValue {
                message: "expected two null-terminated strings".into(),
            })
        }
    }

    /// Gets the pattern portion of the regex.
    pub fn pattern(self) -> &'a str {
        self.pattern
    }

    /// Gets the options portion of the regex.
    pub fn options(self) -> &'a str {
        self.options
    }
}

/// A BSON timestamp referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RawTimestamp<'a> {
    data: &'a [u8],
}

impl<'a> RawTimestamp<'a> {
    /// Gets the time portion of the timestamp.
    pub fn time(&self) -> u32 {
        // RawBsonTimestamp can only be constructed with the correct data length, so this should
        // always succeed.
        u32_from_slice(&self.data[4..8]).unwrap()
    }

    /// Gets the increment portion of the timestamp.
    pub fn increment(&self) -> u32 {
        // RawBsonTimestamp can only be constructed with the correct data length, so this should
        // always succeed.
        u32_from_slice(&self.data[0..4]).unwrap()
    }
}

/// A BSON "code with scope" value referencing raw bytes stored elsewhere.
#[derive(Clone, Copy, Debug)]
pub struct RawJavaScriptCodeWithScope<'a> {
    code: &'a str,
    scope: &'a RawDocumentRef,
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
