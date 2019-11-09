use std::convert::TryInto;
use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};

use crate::bson::Bson;
use crate::{ordered};
#[cfg(feature = "decimal128")]
use crate::decimal128::Decimal128;
use crate::spec::{BinarySubtype, ElementType};
use crate::{oid, ValueAccessError, ValueAccessResult};

#[derive(Clone, Copy)]
pub struct RawBsonDoc<'a> {
    data: &'a [u8],
}

/// Error to indicate that either a value was empty or it contained an unexpected
/// type, for use with the direct getters.
#[derive(Debug, PartialEq)]
pub enum RawValueAccessError<'a> {
    /// Cannot find the expected field with the specified key
    NotPresent,
    /// Found a Bson value with the specified key, but not with the expected type
    UnexpectedType,
    /// Found a value where a utf-8 string was expected, but it was not valid
    /// utf-8.  The error value includes the raw bytes as a &[u8]
    EncodingError(&'a [u8]),
}

type RawValueAccessResult<'a, T> = Result<T, RawValueAccessError<'a>>;

impl<'a> From<RawValueAccessError<'a>> for ValueAccessError {
    fn from(src: RawValueAccessError<'a>) -> ValueAccessError {
        match src {
            RawValueAccessError::NotPresent => ValueAccessError::NotPresent,
            RawValueAccessError::UnexpectedType => ValueAccessError::UnexpectedType,
            RawValueAccessError::EncodingError(_) => ValueAccessError::UnexpectedType,
        }
    }
}

impl<'a> From<ValueAccessError> for RawValueAccessError<'a> {
    fn from(src: ValueAccessError) -> RawValueAccessError<'a> {
        match src {
            ValueAccessError::NotPresent => RawValueAccessError::NotPresent,
            ValueAccessError::UnexpectedType => RawValueAccessError::UnexpectedType,
        }
    }
}

impl<'a> RawBsonDoc<'a> {
    pub fn new(data: &'a [u8]) -> RawBsonDoc<'a> {
        let length = i32_from_slice(&data[..4]);
        assert_eq!(data.len() as i32, length); // Length is properly specified
        assert_eq!(*data.iter().last().unwrap(), 0); // Document is null terminated
        RawBsonDoc { data }
    }

    pub fn get(self, key: &str) -> ValueAccessResult<RawBson<'a>> {
        for (thiskey, bson) in self.into_iter() {
            if thiskey == key {
                return Ok(bson);
            }
        }
        Err(ValueAccessError::NotPresent)
    }

    pub fn get_f64(self, key: &str) -> RawValueAccessResult<'a, f64> {
        self.get(key)?.as_f64()
    }

    pub fn get_str(self, key: &str) -> RawValueAccessResult<'a, &'a str> {
        self.get(key)?.as_str()
    }

    pub fn get_document(self, key: &str) -> RawValueAccessResult<'a, RawBsonDoc<'a>> {
        self.get(key)?.as_document()
    }

    pub fn get_array(self, key: &str) -> RawValueAccessResult<'a, RawBsonArray<'a>> {
        self.get(key)?.as_array()
    }

    pub fn get_binary(self, key: &str) -> RawValueAccessResult<'a, RawBsonBinary<'a>> {
        self.get(key)?.as_binary()
    }

    pub fn get_object_id(self, key: &str) -> RawValueAccessResult<'a, oid::ObjectId> {
        self.get(key)?.as_object_id()
    }

    pub fn get_bool(self, key: &str) -> RawValueAccessResult<'a, bool> {
        self.get(key)?.as_bool()
    }

    pub fn get_utc_date_time(self, key: &str) -> RawValueAccessResult<'a, DateTime<Utc>> {
        self.get(key)?.as_utc_date_time()
    }

    pub fn get_null(self, key: &str) -> RawValueAccessResult<'a, ()> {
        self.get(key)?.as_null()
    }

    pub fn get_regex(self, key: &str) -> RawValueAccessResult<'a, RawBsonRegex<'a>> {
        self.get(key)?.as_regex()
    }

    pub fn get_javascript(self, key: &str) -> RawValueAccessResult<'a, &'a str> {
        self.get(key)?.as_javascript()
    }

    pub fn get_symbol(self, key: &str) -> RawValueAccessResult<'a, &'a str> {
        self.get(key)?.as_symbol()
    }

    pub fn get_javascript_with_scope(self, key: &str) -> RawValueAccessResult<'a, (&'a str, RawBsonDoc<'a>)> {
        self.get(key)?.as_javascript_with_scope()
    }

    pub fn get_i32(self, key: &str) -> RawValueAccessResult<'a, i32> {
        self.get(key)?.as_i32()
    }

    pub fn get_timestamp(self, key: &str) -> RawValueAccessResult<'a, u64> {
        self.get(key)?.as_timestamp()
    }

    pub fn get_i64(self, key: &str) -> RawValueAccessResult<'a, i64> {
        self.get(key)?.as_i64()
    }

    pub fn as_bytes(self) -> &'a [u8] {
        self.data
    }
}

impl<'a> From<RawBsonDoc<'a>> for ordered::OrderedDocument {
    fn from(rawdoc: RawBsonDoc<'a>) -> ordered::OrderedDocument {
        rawdoc.into_iter().map(|(k, v)| (k.to_owned(), v.into())).collect()
    }
}
impl<'a> IntoIterator for RawBsonDoc<'a> {
    type IntoIter = RawBsonDocIterator<'a>;
    type Item = (&'a str, RawBson<'a>);

    fn into_iter(self) -> RawBsonDocIterator<'a> {
        RawBsonDocIterator { doc: self, offset: 4 }
    }
}

pub struct RawBsonDocIterator<'a> {
    doc: RawBsonDoc<'a>,
    offset: usize,
}

impl<'a> Iterator for RawBsonDocIterator<'a> {
    type Item = (&'a str, RawBson<'a>);

    fn next(&mut self) -> Option<(&'a str, RawBson<'a>)> {
        let key = {
            let mut splits = self.doc.data[self.offset + 1..].split(|x| *x == 0);
            splits.next()?
        };
        let valueoffset = self.offset + 1 + key.len() + 1; // type specifier + key + \0
        let element_type = ElementType::from(self.doc.data[self.offset])?;
        let nextoffset = valueoffset
            + match element_type {
                ElementType::FloatingPoint => 8,
                ElementType::Utf8String => 4 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize,
                ElementType::EmbeddedDocument => i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize,
                ElementType::Array => i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize,
                ElementType::Binary => 5 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize,
                ElementType::Undefined => 0,
                ElementType::ObjectId => 12,
                ElementType::Boolean => 1,
                ElementType::UtcDatetime => 8,
                ElementType::NullValue => 0,
                ElementType::RegularExpression => {
                    let mut splits = self.doc.data[valueoffset..].splitn(3, |x| *x == 0);
                    let regex = splits.next()?;
                    let options = splits.next()?;
                    regex.len() + options.len() + 2
                }
                ElementType::DbPointer => {
                    4 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize + 12
                }
                ElementType::JavaScriptCode => {
                    4 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize
                }
                ElementType::Symbol => 4 + i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize,
                ElementType::JavaScriptCodeWithScope => {
                    i32_from_slice(&self.doc.data[valueoffset..valueoffset + 4]) as usize
                }
                ElementType::Integer32Bit => 4,
                ElementType::TimeStamp => 8,
                ElementType::Integer64Bit => 8,
                #[cfg(feature = "decimal128")]
                ElementType::Decimal128Bit => 16,
                ElementType::MaxKey => 0,
                ElementType::MinKey => 0,
            };

        self.offset = nextoffset;
        Some((
            std::str::from_utf8(key).unwrap(),
            RawBson::new(element_type, &self.doc.data[valueoffset..nextoffset]),
        ))
    }
}

#[derive(Clone, Copy)]
pub struct RawBsonArray<'a> {
    doc: RawBsonDoc<'a>,
}

impl<'a> RawBsonArray<'a> {
    pub fn new(data: &'a [u8]) -> RawBsonArray<'a> {
        RawBsonArray::from_doc(RawBsonDoc::new(data))
    }

    pub fn from_doc(doc: RawBsonDoc<'a>) -> RawBsonArray<'a> {
        RawBsonArray { doc }
    }

    pub fn get(self, index: usize) -> RawValueAccessResult<'a, RawBson<'a>> {
        self.into_iter().nth(index).ok_or(RawValueAccessError::NotPresent)
    }

    pub fn get_f64(self, index: usize) -> RawValueAccessResult<'a, f64> {
        self.get(index)?.as_f64()
    }

    pub fn get_str(self, index: usize) -> RawValueAccessResult<'a, &'a str> {
        self.get(index)?.as_str()
    }

    pub fn get_document(self, index: usize) -> RawValueAccessResult<'a, RawBsonDoc<'a>> {
        self.get(index)?.as_document()
    }

    pub fn get_array(self, index: usize) -> RawValueAccessResult<'a, RawBsonArray<'a>> {
        self.get(index)?.as_array()
    }

    pub fn get_binary(self, index: usize) -> RawValueAccessResult<'a, RawBsonBinary<'a>> {
        self.get(index)?.as_binary()
    }

    pub fn get_object_id(self, index: usize) -> RawValueAccessResult<'a, oid::ObjectId> {
        self.get(index)?.as_object_id()
    }

    pub fn get_bool(self, index: usize) -> RawValueAccessResult<'a, bool> {
        self.get(index)?.as_bool()
    }

    pub fn get_utc_date_time(self, index: usize) -> RawValueAccessResult<'a, DateTime<Utc>> {
        self.get(index)?.as_utc_date_time()
    }

    pub fn get_null(self, index: usize) -> RawValueAccessResult<'a, ()> {
        self.get(index)?.as_null()
    }

    pub fn get_regex(self, index: usize) -> RawValueAccessResult<'a, RawBsonRegex<'a>> {
        self.get(index)?.as_regex()
    }

    pub fn get_javascript(self, index: usize) -> RawValueAccessResult<'a, &'a str> {
        self.get(index)?.as_javascript()
    }

    pub fn get_symbol(self, index: usize) -> RawValueAccessResult<'a, &'a str> {
        self.get(index)?.as_symbol()
    }

    pub fn get_javascript_with_scope(self, index: usize) -> RawValueAccessResult<'a, (&'a str, RawBsonDoc<'a>)> {
        self.get(index)?.as_javascript_with_scope()
    }

    pub fn get_i32(self, index: usize) -> RawValueAccessResult<'a, i32> {
        self.get(index)?.as_i32()
    }

    pub fn get_timestamp(self, index: usize) -> RawValueAccessResult<'a, u64> {
        self.get(index)?.as_timestamp()
    }

    pub fn get_i64(self, index: usize) -> RawValueAccessResult<'a, i64> {
        self.get(index)?.as_i64()
    }

    pub fn to_vec(self) -> Vec<RawBson<'a>> {
        self.into_iter().collect()
    }

    pub fn as_bytes(self) -> &'a [u8] {
        self.doc.as_bytes()
    }
}

impl<'a> From<RawBsonArray<'a>> for Vec<Bson> {
    fn from(arr: RawBsonArray) -> Vec<Bson> {
        arr.into_iter().map(Bson::from).collect()
    }
}
impl<'a> IntoIterator for RawBsonArray<'a> {
    type IntoIter = RawBsonArrayIterator<'a>;
    type Item = RawBson<'a>;

    fn into_iter(self) -> RawBsonArrayIterator<'a> {
        RawBsonArrayIterator {
            dociter: self.doc.into_iter(),
            index: 0,
        }
    }
}

pub struct RawBsonArrayIterator<'a> {
    dociter: RawBsonDocIterator<'a>,
    index: usize,
}

impl<'a> Iterator for RawBsonArrayIterator<'a> {
    type Item = RawBson<'a>;

    fn next(&mut self) -> Option<RawBson<'a>> {
        self.dociter.next().map(|(key, bson)| {
            assert_eq!(key.parse::<usize>().expect("key was not an integer"), self.index);
            self.index += 1;
            bson
        })
    }
}

#[derive(Clone, Copy)]
pub struct RawBsonBinary<'a> {
    subtype: BinarySubtype,
    data: &'a [u8],
}

impl<'a> RawBsonBinary<'a> {
    pub fn new(subtype: BinarySubtype, data: &'a [u8]) -> RawBsonBinary<'a> {
        RawBsonBinary { subtype, data }
    }

    pub fn subtype(self) -> BinarySubtype {
        self.subtype
    }

    pub fn as_bytes(self) -> &'a [u8] {
        self.data
    }
}

#[derive(Clone, Copy)]
pub struct RawBsonRegex<'a> {
    pattern: &'a [u8],
    opts: &'a [u8],
}

impl<'a> RawBsonRegex<'a> {
    pub fn new(data: &'a [u8]) -> RawBsonRegex<'a> {
        let mut splits = data.split(|x| *x == 0);
        let pattern = splits.next().expect("no pattern");
        let opts = splits.next().expect("no opts");
        RawBsonRegex { pattern, opts }
    }

    pub fn pattern(self) -> &'a str {
        std::str::from_utf8(self.pattern).expect("invalid utf8")
    }

    pub fn opts(self) -> &'a str {
        std::str::from_utf8(self.opts).expect("invalid utf8")
    }
}

#[derive(Clone, Copy)]
pub struct RawBson<'a> {
    element_type: ElementType,
    data: &'a [u8],
}

impl<'a> RawBson<'a> {
    // This is not public.  A RawBson object can only be created by the .get() method
    // on RawBsonDoc
    pub(super) fn new(element_type: ElementType, data: &'a [u8]) -> RawBson<'a> {
        RawBson { element_type, data }
    }

    pub fn element_type(self) -> ElementType {
        self.element_type
    }

    pub fn as_bytes(self) -> &'a [u8] {
        self.data
    }

    pub fn as_f64(self) -> RawValueAccessResult<'a, f64> {
        if let ElementType::FloatingPoint = self.element_type {
            Ok(f64::from_bits(u64::from_le_bytes(
                self.data.try_into().expect("f64 should be 8 bytes long"),
            )))
        } else {
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    pub fn as_str(self) -> RawValueAccessResult<'a, &'a str> {
        if let ElementType::Utf8String = self.element_type {
            let length = i32_from_slice(&self.data[..4]);
            assert_eq!(self.data.len() as i32, length + 4);
            self.try_to_str(&self.data[4..4 + length as usize - 1])
        } else {
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    pub fn as_document(self) -> RawValueAccessResult<'a, RawBsonDoc<'a>> {
        if let ElementType::EmbeddedDocument = self.element_type {
            Ok(RawBsonDoc::new(self.data))
        } else {
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    pub fn as_array(self) -> RawValueAccessResult<'a, RawBsonArray<'a>> {
        if let ElementType::Array = self.element_type {
            Ok(RawBsonArray::new(self.data))
        } else {
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    pub fn as_binary(self) -> RawValueAccessResult<'a, RawBsonBinary<'a>> {
        if let ElementType::Binary = self.element_type {
            let length = i32_from_slice(&self.data[0..4]);
            let subtype = BinarySubtype::from(self.data[4]); // TODO: This mishandles reserved values
            assert_eq!(self.data.len() as i32, length + 5);
            let data = match subtype {
                BinarySubtype::BinaryOld => {
                    let oldlength = i32_from_slice(&self.data[5..9]);
                    assert_eq!(oldlength + 4, length);
                    &self.data[9..]
                }
                _ => &self.data[5..],
            };
            Ok(RawBsonBinary::new(subtype, data))
        } else {
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    pub fn as_object_id(self) -> RawValueAccessResult<'a, oid::ObjectId> {
        if let ElementType::ObjectId = self.element_type {
            Ok(oid::ObjectId::with_bytes(
                self.data.try_into().expect("object id should be 12 bytes long"),
            ))
        } else {
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    pub fn as_bool(self) -> RawValueAccessResult<'a, bool> {
        if let ElementType::Boolean = self.element_type {
            assert_eq!(self.data.len(), 1);
            match self.data[0] {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(RawValueAccessError::EncodingError(self.data)),
            }
        } else {
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    pub fn as_utc_date_time(self) -> RawValueAccessResult<'a, DateTime<Utc>> {
        if let ElementType::UtcDatetime = self.element_type {
            let millis = i64_from_slice(self.data);
            if millis >= 0 {
                let duration = Duration::from_millis(millis as u64);
                Ok(Utc.timestamp(duration.as_secs().try_into().unwrap(), duration.subsec_nanos()))
            } else {
                let duration = Duration::from_millis((-1 * millis).try_into().unwrap());
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
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    pub fn as_null(self) -> RawValueAccessResult<'a, ()> {
        if let ElementType::NullValue = self.element_type {
            Ok(())
        } else {
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    pub fn as_regex(self) -> RawValueAccessResult<'a, RawBsonRegex<'a>> {
        if let ElementType::RegularExpression = self.element_type {
            Ok(RawBsonRegex::new(self.data))
        } else {
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    pub fn as_javascript(self) -> RawValueAccessResult<'a, &'a str> {
        if let ElementType::JavaScriptCode = self.element_type {
            let length = i32_from_slice(&self.data[..4]);
            assert_eq!(self.data.len() as i32, length + 4);
            self.try_to_str(&self.data[4..4 + length as usize - 1])
        } else {
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    pub fn as_symbol(self) -> RawValueAccessResult<'a, &'a str> {
        if let ElementType::Symbol = self.element_type {
            let length = i32_from_slice(&self.data[..4]);
            assert_eq!(self.data.len() as i32, length + 4);
            self.try_to_str(&self.data[4..4 + length as usize - 1])
        } else {
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    pub fn as_javascript_with_scope(self) -> RawValueAccessResult<'a, (&'a str, RawBsonDoc<'a>)> {
        if let ElementType::JavaScriptCodeWithScope = self.element_type {
            let length = i32_from_slice(&self.data[..4]);
            assert_eq!(self.data.len() as i32, length);

            let js_len = i32_from_slice(&self.data[4..8]) as usize;
            let js = &self.data[8..8 + js_len - 1];
            let doc_offset = 8 + js_len;
            let doc_len = i32_from_slice(&self.data[doc_offset..doc_offset + 4]) as usize;
            assert_eq!(self.data.len(), doc_offset + doc_len);
            let doc = RawBsonDoc::new(&self.data[doc_offset..doc_offset + doc_len]);
            Ok((std::str::from_utf8(js).expect("js was not a string"), doc))
        } else {
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    pub fn as_i32(self) -> RawValueAccessResult<'a, i32> {
        if let ElementType::Integer32Bit = self.element_type {
            assert_eq!(self.data.len(), 4);
            Ok(i32_from_slice(self.data))
        } else {
            Err(RawValueAccessError::UnexpectedType)

        }
    }

    pub fn as_timestamp(self) -> RawValueAccessResult<'a, u64> {
        if let ElementType::TimeStamp = self.element_type {
            assert_eq!(self.data.len(), 8);
            Ok(u64_from_slice(self.data))
        } else {
            Err(RawValueAccessError::UnexpectedType)

        }
    }

    pub fn as_i64(self) -> RawValueAccessResult<'a, i64> {
        if let ElementType::Integer64Bit = self.element_type {
            assert_eq!(self.data.len(), 8);
            Ok(i64_from_slice(self.data))
        } else {
            Err(RawValueAccessError::UnexpectedType)

        }
    }

    #[cfg(feature="decimal128")]
    pub fn as_decimal128(self) -> RawValueAccessResult<'a, Decimal128> {
        if let ElementType::Decimal128Bit = self.element_type {
            assert_eq!(self.data.len(), 16);
            Ok(d128_from_slice(self.data))
        } else {
            Err(RawValueAccessError::UnexpectedType)
        }
    }

    fn try_to_str(&self, data: &'a [u8]) -> RawValueAccessResult<'a, &'a str> {
        match std::str::from_utf8(data) {
            Ok(s) => Ok(s),
            Err(_) => Err(RawValueAccessError::EncodingError(data))
        }

    }
}

impl<'a> From<RawBson<'a>> for Bson {
    fn from(rawbson: RawBson<'a>) -> Bson {
        match rawbson.element_type {
            ElementType::FloatingPoint => {
                Bson::FloatingPoint(rawbson.as_f64().expect("not an f64"))
            }
            ElementType::Utf8String => {
                Bson::String(String::from(rawbson.as_str().expect("not a string")))
            }
            ElementType::EmbeddedDocument => {
                let rawdoc = rawbson.as_document().expect("not a document");
                let doc = rawdoc.into();
                Bson::Document(doc)
            }
            ElementType::Array => {
                let rawarray = rawbson.as_array().expect("not an array");
                let v = rawarray.into();
                Bson::Array(v)
            }
            ElementType::Binary => {
                let RawBsonBinary { subtype, data } = rawbson.as_binary().expect("not binary");
                let data = match subtype {
                    BinarySubtype::BinaryOld => {
                        // Bson type includes the old binary length specifier in the data
                        let mut v = Vec::with_capacity(data.len() + 4);
                        v.extend_from_slice(&(data.len() as i32).to_le_bytes());
                        v.extend_from_slice(data);
                        v
                    }
                    _ => data.to_vec(),
                };
                Bson::Binary(subtype, data)
            }
            ElementType::ObjectId => {
                Bson::ObjectId(rawbson.as_object_id().expect("not an object_id"))
            }
            ElementType::Boolean => Bson::Boolean(rawbson.as_bool().expect("not a boolean")),
            ElementType::UtcDatetime => {
                Bson::UtcDatetime(rawbson.as_utc_date_time().expect("not a datetime"))
            }
            ElementType::NullValue => Bson::Null,
            ElementType::RegularExpression => {
                let rawregex = rawbson.as_regex().expect("not a regex");
                Bson::RegExp(
                    String::from(rawregex.pattern()),
                    String::from(rawregex.opts()),
                )
            }
            ElementType::JavaScriptCode => Bson::JavaScriptCode(String::from(
                rawbson.as_javascript().expect("not javascript"),
            )),
            ElementType::Integer32Bit => Bson::I32(rawbson.as_i32().expect("not int32")),
            ElementType::TimeStamp => {
                Bson::TimeStamp(
                    rawbson
                        .as_timestamp()
                        .expect("not timestamp")
                        .try_into()
                        .expect("Bson::Timestamp expects i64, but bson defines timestamp as u64, and no lossless conversion was possible")
                )
            },
            ElementType::Integer64Bit => Bson::I64(rawbson.as_i64().expect("not int32")),
            ElementType::Undefined => Bson::Null,
            ElementType::DbPointer => panic!("Uh oh. Maybe this should be a TryFrom"),
            ElementType::Symbol => Bson::Symbol(String::from(rawbson.as_symbol().expect("not a symbol"))),
            ElementType::JavaScriptCodeWithScope => {
                let (js, scope) = rawbson.as_javascript_with_scope().expect("not javascript with scope");
                Bson::JavaScriptCodeWithScope(String::from(js), scope.into())
            },
            #[cfg(feature = "decimal128")]
            ElementType::Decimal128Bit => Bson::Decimal128(rawbson.as_decimal128().expect("not a decimal 128")),
            ElementType::MaxKey => unimplemented!(),
            ElementType::MinKey => unimplemented!(),
        }
    }
}

// Given a 4 byte u8 slice, return an i32 calculated from the bytes in
// little endian order
//
// # Panics
//
// This function panics if given a slice that is not four bytes long.
fn i32_from_slice(val: &[u8]) -> i32 {
    i32::from_le_bytes(val.try_into().expect("i32 is four bytes"))
}

// Given an 8 byte u8 slice, return an i64 calculated from the bytes in
// little endian order
//
// # Panics
//
// This function panics if given a slice that is not eight bytes long.
fn i64_from_slice(val: &[u8]) -> i64 {
    i64::from_le_bytes(val.try_into().expect("i64 is eight bytes"))
}

// Given an 8 byte u8 slice, return a u64 calculated from the bytes in
// little endian order
//
// # Panics
//
// This function panics if given a slice that is not eight bytes long.
fn u64_from_slice(val: &[u8]) -> u64 {
    u64::from_le_bytes(val.try_into().expect("u64 is eight bytes"))
}

#[cfg(feature = "decimal128")]
fn d128_from_slice(val: &[u8]) -> Decimal128 {
    unsafe {
        Decimal128::from_raw_bytes_le(val.try_into().expect("d128 is eight bytes"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{doc, encode_document, Bson, Document};

    fn to_bytes(doc: &Document) -> Vec<u8> {
        let mut docbytes = Vec::new();
        encode_document(&mut docbytes, doc).unwrap();
        docbytes
    }

    #[test]
    fn string_from_document() {
        let docbytes = to_bytes(&doc! {
            "this": "first",
            "that": "second",
            "something": "else",
        });
        let rawdoc = RawBsonDoc::new(&docbytes);
        assert_eq!(rawdoc.get("that").unwrap().as_str().unwrap(), "second",);
    }

    #[test]
    fn nested_document() {
        let docbytes = to_bytes(&doc! {
            "outer": {
                "inner": "surprise",
            },
        });
        let rawdoc = RawBsonDoc::new(&docbytes);
        assert_eq!(
            rawdoc
                .get("outer")
                .expect("get doc")
                .as_document()
                .expect("as doc")
                .get("inner")
                .expect("get str")
                .as_str()
                .expect("as str"),
            "surprise",
        );
    }

    #[test]
    fn object_id() {
        let bson = super::RawBson::new(ElementType::ObjectId, &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]);
        assert_eq!(
            bson.as_object_id(),
            Ok(oid::ObjectId::with_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]))
        );
    }
    #[test]
    fn iterate() {
        let docbytes = to_bytes(&doc! {
            "apples": "oranges",
            "peanut butter": "chocolate",
            "easy as": {"do": 1, "re": 2, "mi": 3},
        });
        let rawdoc = RawBsonDoc::new(&docbytes);
        let mut dociter = rawdoc.into_iter();
        let next = dociter.next().expect("no result");
        assert_eq!(next.0, "apples");
        assert_eq!(next.1.as_str().expect("result was not a str"), "oranges");
        let next = dociter.next().expect("no result");
        assert_eq!(next.0, "peanut butter");
        assert_eq!(next.1.as_str().expect("result was not a str"), "chocolate");
        let next = dociter.next().expect("no result");
        assert_eq!(next.0, "easy as");
        let _doc = next.1.as_document().expect("result was a not a document");
        let next = dociter.next();
        assert!(next.is_none());
    }

    #[test]
    fn bson_types() {
        let docbytes = to_bytes(&doc! {
            "f64": 2.5,
            "string": "hello",
            "document": {},
            "array": ["binary", "serialized", "object", "notation"],
            "binary": (BinarySubtype::Generic, vec![1u8, 2, 3]),
            "object_id": oid::ObjectId::with_bytes([1, 2, 3, 4, 5,6,7,8,9,10, 11,12].try_into().unwrap()),
            "boolean": true,
            "datetime": Utc::now(),
            "null": Bson::Null,
            "regex": Bson::RegExp(String::from(r"end\s*$"), String::from("i")),
            "javascript": Bson::JavaScriptCode(String::from("console.log(console);")),
            "symbol": Bson::Symbol(String::from("artist-formerly-known-as")),
            "javascript_with_scope": Bson::JavaScriptCodeWithScope(String::from("console.log(msg);"), doc!{"ok": true}),
            "int32": 23i32,
            "timestamp": Bson::TimeStamp(3542578),
            "int64": 46i64,
            "end": "END",
        });

        let rawdoc = RawBsonDoc::new(&docbytes);
        let doc: Document = rawdoc.into();
        println!("{:#?}", doc);
        //assert!(false);
        assert_eq!(
            rawdoc
                .get("f64")
                .expect("no key f64")
                .as_f64()
                .expect("result was not a f64"),
            2.5,
        );
        assert_eq!(
            rawdoc
                .get("string")
                .expect("no key string")
                .as_str()
                .expect("result was not a string"),
            "hello",
        );
        let doc = rawdoc
            .get("document")
            .expect("no key document")
            .as_document()
            .expect("result was not a document");
        assert_eq!(doc.data, &[5, 0, 0, 0, 0]); // Empty document

        let array: RawBsonArray<'_> = rawdoc
            .get("array")
            .expect("no key array")
            .as_array()
            .expect("result was not an array");

        assert_eq!(array.get_str(0), Ok("binary"));
        assert_eq!(array.get_str(3), Ok("notation"));
        assert_eq!(array.get_str(4), Err(RawValueAccessError::NotPresent));

        let binary: RawBsonBinary<'_> = rawdoc
            .get("binary")
            .expect("no key binary")
            .as_binary()
            .expect("result was not a binary object");
        assert_eq!(binary.subtype, BinarySubtype::Generic);
        assert_eq!(binary.data, &[1, 2, 3]);

        let oid = rawdoc
            .get("object_id")
            .expect("no key object_id")
            .as_object_id()
            .expect("result was not an object id");
        assert_eq!(oid.to_hex(), "0102030405060708090a0b0c");

        let boolean = rawdoc
            .get("boolean")
            .expect("no key boolean")
            .as_bool()
            .expect("result was not boolean");

        assert_eq!(boolean, true);

        let _dt: DateTime<Utc> = rawdoc
            .get("datetime")
            .expect("no key datetime")
            .as_utc_date_time()
            .expect("was not utc_date_time");

        let null = rawdoc
            .get("null")
            .expect("no key null")
            .as_null()
            .expect("was not null");

        assert_eq!(null, ());

        let regex = rawdoc
            .get("regex")
            .expect("no key regex")
            .as_regex()
            .expect("was not regex");
        assert_eq!(regex.pattern(), r"end\s*$");
        assert_eq!(regex.opts(), "i");

        let js = rawdoc
            .get("javascript")
            .expect("no key javascript")
            .as_javascript()
            .expect("was not javascript");
        assert_eq!(js, "console.log(console);");

        let symbol = rawdoc
            .get("symbol")
            .expect("no key symbol")
            .as_symbol()
            .expect("was not symbol");
        assert_eq!(symbol, "artist-formerly-known-as");

        let (js, scopedoc) = rawdoc
            .get("javascript_with_scope")
            .expect("no key javascript_with-scope")
            .as_javascript_with_scope()
            .expect("was not javascript with scope");
        assert_eq!(js, "console.log(msg);");
        let (scope_key, scope_value_bson) = scopedoc.into_iter().next()
            .expect("no next value in scope");
        assert_eq!(scope_key, "ok");
        let scope_value = scope_value_bson.as_bool().expect("not a boolean");
        assert_eq!(scope_value, true);

        let int32 = rawdoc
            .get("int32")
            .expect("no key int32")
            .as_i32()
            .expect("was not int32");
        assert_eq!(int32, 23i32);

        let ts = rawdoc
            .get("timestamp")
            .expect("no key timestamp")
            .as_timestamp()
            .expect("was not a timestamp");

        assert_eq!(ts, 3542578);

        let int64 = rawdoc
            .get("int64")
            .expect("no key int64")
            .as_i64()
            .expect("was not int64");
        assert_eq!(int64, 46i64);

        let end = rawdoc.get("end").expect("no key end").as_str().expect("was not str");
        assert_eq!(end, "END");
    }

    #[test]
    fn into_bson_conversion() {
        let docbytes = to_bytes(&doc! {
            "f64": 2.5,
            "string": "hello",
            "document": {},
            "array": ["binary", "serialized", "object", "notation"],
            "object_id": oid::ObjectId::with_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
            "binary": (BinarySubtype::Generic, vec![1u8, 2, 3]),
            "boolean": false,
        });
        let rawbson = RawBson::new(ElementType::EmbeddedDocument, &docbytes);
        let b: Bson = rawbson.into();
        let doc = b.as_document().expect("not a document");
        assert_eq!(*doc.get("f64").expect("f64 not found"), Bson::FloatingPoint(2.5));
        assert_eq!(
            *doc.get("string").expect("string not found"),
            Bson::String(String::from("hello"))
        );
        assert_eq!(
            *doc.get("document").expect("document not found"),
            Bson::Document(doc! {})
        );
        assert_eq!(
            *doc.get("array").expect("array not found"),
            Bson::Array(
                vec!["binary", "serialized", "object", "notation"]
                    .into_iter()
                    .map(|s| Bson::String(String::from(s)))
                    .collect()
            )
        );
        assert_eq!(
            *doc.get("object_id").expect("object_id not found"),
            Bson::ObjectId(oid::ObjectId::with_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]))
        );
        assert_eq!(
            *doc.get("binary").expect("binary not found"),
            Bson::Binary(BinarySubtype::Generic, vec![1, 2, 3])
        );
        assert_eq!(*doc.get("boolean").expect("boolean not found"), Bson::Boolean(false));
    }
}
