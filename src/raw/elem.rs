use std::{
    convert::{TryFrom, TryInto},
    time::Duration,
};

use chrono::{DateTime, TimeZone, Utc};

#[cfg(feature = "decimal128")]
use super::d128_from_slice;
use super::{
    i32_from_slice,
    i64_from_slice,
    read_lenencoded,
    read_nullterminated,
    u32_from_slice,
    RawArray,
    RawDocumentRef,
    RawError,
    RawResult,
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

    /// Gets the f64 that's referenced or returns an error if the value isn't a BSON double.
    pub fn as_f64(self) -> RawResult<f64> {
        if let ElementType::Double = self.element_type {
            Ok(f64::from_bits(u64::from_le_bytes(
                self.data
                    .try_into()
                    .map_err(|_| RawError::MalformedValue("f64 should be 8 bytes long".into()))?,
            )))
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the string that's referenced or returns an error if the value isn't a BSON string.
    pub fn as_str(self) -> RawResult<&'a str> {
        if let ElementType::String = self.element_type {
            read_lenencoded(self.data)
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the document that's referenced or returns an error if the value isn't a BSON document.
    pub fn as_document(self) -> RawResult<&'a RawDocumentRef> {
        if let ElementType::EmbeddedDocument = self.element_type {
            RawDocumentRef::new(self.data)
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the array that's referenced or returns an error if the value isn't a BSON array.
    pub fn as_array(self) -> RawResult<&'a RawArray> {
        if let ElementType::Array = self.element_type {
            RawArray::new(self.data)
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the BSON binary value that's referenced or returns an error if the value a BSON binary.
    pub fn as_binary(self) -> RawResult<RawBinary<'a>> {
        if let ElementType::Binary = self.element_type {
            let length = i32_from_slice(&self.data[0..4]);
            let subtype = BinarySubtype::from(self.data[4]); // TODO: This mishandles reserved values
            if self.data.len() as i32 != length + 5 {
                return Err(RawError::MalformedValue(
                    "binary bson has wrong declared length".into(),
                ));
            }
            let data = match subtype {
                BinarySubtype::BinaryOld => {
                    if length < 4 {
                        return Err(RawError::MalformedValue(
                            "old binary subtype has no inner declared length".into(),
                        ));
                    }
                    let oldlength = i32_from_slice(&self.data[5..9]);
                    if oldlength + 4 != length {
                        return Err(RawError::MalformedValue(
                            "old binary subtype has wrong inner declared length".into(),
                        ));
                    }
                    &self.data[9..]
                }
                _ => &self.data[5..],
            };
            Ok(RawBinary::new(subtype, data))
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the ObjectId that's referenced or returns an error if the value isn't a BSON ObjectId.
    pub fn as_object_id(self) -> RawResult<ObjectId> {
        if let ElementType::ObjectId = self.element_type {
            Ok(ObjectId::with_bytes(self.data.try_into().map_err(
                |_| RawError::MalformedValue("object id should be 12 bytes long".into()),
            )?))
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the boolean that's referenced or returns an error if the value isn't a BSON boolean.
    pub fn as_bool(self) -> RawResult<bool> {
        if let ElementType::Boolean = self.element_type {
            if self.data.len() != 1 {
                Err(RawError::MalformedValue("boolean has length != 1".into()))
            } else {
                match self.data[0] {
                    0 => Ok(false),
                    1 => Ok(true),
                    _ => Err(RawError::MalformedValue(
                        "boolean value was not 0 or 1".into(),
                    )),
                }
            }
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the DateTime that's referenced or returns an error if the value isn't a BSON DateTime.
    pub fn as_datetime(self) -> RawResult<DateTime<Utc>> {
        if let ElementType::DateTime = self.element_type {
            let millis = i64_from_slice(self.data);
            if millis >= 0 {
                let duration = Duration::from_millis(millis as u64);
                Ok(Utc.timestamp(
                    duration.as_secs().try_into().unwrap(),
                    duration.subsec_nanos(),
                ))
            } else {
                let duration = Duration::from_millis((-millis).try_into().unwrap());
                let mut secs: i64 = duration.as_secs().try_into().unwrap();
                secs *= -1;
                let mut nanos = duration.subsec_nanos();
                if nanos > 0 {
                    secs -= 1;
                    nanos = 1_000_000_000 - nanos;
                }
                Ok(Utc.timestamp(secs, nanos))
            }
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the regex that's referenced or returns an error if the value isn't a BSON regex.
    pub fn as_regex(self) -> RawResult<RawRegex<'a>> {
        if let ElementType::RegularExpression = self.element_type {
            RawRegex::new(self.data)
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the BSON JavaScript code that's referenced or returns an error if the value isn't BSON
    /// JavaScript code.
    pub fn as_javascript(self) -> RawResult<&'a str> {
        if let ElementType::JavaScriptCode = self.element_type {
            read_lenencoded(self.data)
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the symbol that's referenced or returns an error if the value isn't a BSON symbol.
    pub fn as_symbol(self) -> RawResult<&'a str> {
        if let ElementType::Symbol = self.element_type {
            read_lenencoded(self.data)
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the BSON JavaScript code with scope that's referenced or returns an error if the value
    /// isn't BSON JavaScript code with scope.
    pub fn as_javascript_with_scope(self) -> RawResult<(&'a str, &'a RawDocumentRef)> {
        if let ElementType::JavaScriptCodeWithScope = self.element_type {
            let length = i32_from_slice(&self.data[..4]);
            assert_eq!(self.data.len() as i32, length);

            let js = read_lenencoded(&self.data[4..])?;
            let doc = RawDocumentRef::new(&self.data[9 + js.len()..])?;

            Ok((js, doc))
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the timestamp that's referenced or returns an error if the value isn't a BSON
    /// timestamp.
    pub fn as_timestamp(self) -> RawResult<RawTimestamp<'a>> {
        if let ElementType::Timestamp = self.element_type {
            assert_eq!(self.data.len(), 8);
            Ok(RawTimestamp { data: self.data })
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the i32 that's referenced or returns an error if the value isn't a BSON int32.
    pub fn as_i32(self) -> RawResult<i32> {
        if let ElementType::Int32 = self.element_type {
            assert_eq!(self.data.len(), 4);
            Ok(i32_from_slice(self.data))
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the i64 that's referenced or returns an error if the value isn't a BSON int64.
    pub fn as_i64(self) -> RawResult<i64> {
        if let ElementType::Int64 = self.element_type {
            assert_eq!(self.data.len(), 8);
            Ok(i64_from_slice(self.data))
        } else {
            Err(RawError::UnexpectedType)
        }
    }

    /// Gets the decimal that's referenced or returns an error if the value isn't a BSON Decimal128.
    #[cfg(feature = "decimal128")]
    pub fn as_decimal128(self) -> RawResult<Decimal128> {
        if let ElementType::Decimal128 = self.element_type {
            assert_eq!(self.data.len(), 16);
            Ok(d128_from_slice(self.data))
        } else {
            Err(RawError::UnexpectedType)
        }
    }
}

impl<'a> TryFrom<RawBson<'a>> for Bson {
    type Error = RawError;

    fn try_from(rawbson: RawBson<'a>) -> RawResult<Bson> {
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
                let (js, scope) = rawbson.as_javascript_with_scope()?;
                Bson::JavaScriptCodeWithScope(crate::JavaScriptCodeWithScope {
                    code: String::from(js),
                    scope: scope.try_into()?,
                })
            }
            #[cfg(feature = "decimal128")]
            ElementType::Decimal128 => Bson::Decimal128(rawbson.as_decimal128()?),

            #[cfg(not(feature = "decimal128"))]
            ElementType::Decimal128 => return Err(RawError::UnexpectedType),
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
    pub fn new(data: &'a [u8]) -> RawResult<RawRegex<'a>> {
        let pattern = read_nullterminated(data)?;
        let opts = read_nullterminated(&data[pattern.len() + 1..])?;
        if pattern.len() + opts.len() == data.len() - 2 {
            Ok(RawRegex {
                pattern,
                options: opts,
            })
        } else {
            Err(RawError::MalformedValue(
                "expected two null-terminated strings".into(),
            ))
        }
    }

    pub fn pattern(self) -> &'a str {
        self.pattern
    }

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
    /// Return the time portion of the timestamp.
    pub fn time(&self) -> u32 {
        // RawBsonTimestamp can only be constructed with the correct data length, so this should
        // always succeed.
        u32_from_slice(&self.data[4..8])
    }

    /// Return the increment portion of the timestamp.
    pub fn increment(&self) -> u32 {
        // RawBsonTimestamp can only be constructed with the correct data length, so this should
        // always succeed.
        u32_from_slice(&self.data[0..4])
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
