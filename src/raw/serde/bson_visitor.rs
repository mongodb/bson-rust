use std::{borrow::Cow, convert::TryFrom};

use serde::{
    de::{Error as SerdeError, Visitor},
};
use serde_bytes::ByteBuf;

use crate::{
    de::convert_unsigned_to_signed_raw,
    oid::ObjectId,
    raw::{RAW_ARRAY_NEWTYPE, RAW_DOCUMENT_NEWTYPE},
    spec::BinarySubtype,
    Binary,
    DateTime,
    DbPointer,
    Decimal128,
    RawArray,
    RawArrayBuf,
    RawBinaryRef,
    RawBson,
    RawBsonRef,
    RawDbPointerRef,
    RawDocument,
    RawDocumentBuf,
    RawJavaScriptCodeWithScope,
    RawJavaScriptCodeWithScopeRef,
    RawRegexRef,
    Regex,
    Timestamp,
};
use crate::extjson::models::{BorrowedBinaryBody, BorrowedDbPointerBody, BorrowedRegexBody, TimestampBody};

use super::{
    CowByteBuffer,
    CowStr,
    OwnedOrBorrowedRawBson,
    OwnedOrBorrowedRawDocument,
    SeededVisitor,
};

/// A visitor used to deserialize types backed by raw BSON.
pub(crate) struct OwnedOrBorrowedRawBsonVisitor;

impl<'de> Visitor<'de> for OwnedOrBorrowedRawBsonVisitor {
    type Value = OwnedOrBorrowedRawBson<'de>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a raw BSON value")
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(RawBsonRef::String(v).into())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(RawBson::String(v.to_string()).into())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(RawBson::String(v).into())
    }

    fn visit_borrowed_bytes<E>(self, bytes: &'de [u8]) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(RawBsonRef::Binary(RawBinaryRef {
            bytes,
            subtype: BinarySubtype::Generic,
        })
        .into())
    }

    fn visit_i8<E>(self, v: i8) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(RawBsonRef::Int32(v.into()).into())
    }

    fn visit_i16<E>(self, v: i16) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(RawBsonRef::Int32(v.into()).into())
    }

    fn visit_i32<E>(self, v: i32) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(RawBsonRef::Int32(v).into())
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(RawBsonRef::Int64(v).into())
    }

    fn visit_u8<E>(self, value: u8) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(convert_unsigned_to_signed_raw(value.into())?.into())
    }

    fn visit_u16<E>(self, value: u16) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(convert_unsigned_to_signed_raw(value.into())?.into())
    }

    fn visit_u32<E>(self, value: u32) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(convert_unsigned_to_signed_raw(value.into())?.into())
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(convert_unsigned_to_signed_raw(value)?.into())
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(RawBsonRef::Boolean(v).into())
    }

    fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(RawBsonRef::Double(v).into())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(RawBsonRef::Null.into())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(RawBsonRef::Null.into())
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: SerdeError,
    {
        Ok(RawBson::Binary(Binary {
            bytes: v,
            subtype: BinarySubtype::Generic,
        })
        .into())
    }

    fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut buffer = CowByteBuffer::new();
        let seeded_visitor = SeededVisitor::new(&mut buffer);
        seeded_visitor.visit_seq(seq)?;

        match OwnedOrBorrowedRawDocument::try_from(buffer).map_err(SerdeError::custom)? {
            OwnedOrBorrowedRawDocument::Borrowed(borrowed) => {
                let raw_array = RawArray::from_doc(borrowed);
                Ok(RawBsonRef::Array(raw_array).into())
            }
            OwnedOrBorrowedRawDocument::Owned(owned) => {
                let raw_array = RawArrayBuf::from_raw_document_buf(owned);
                Ok(RawBson::Array(raw_array).into())
            }
        }
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let first_key = match map.next_key::<CowStr>()? {
            Some(k) => k,
            None => return Ok(RawBson::Document(RawDocumentBuf::new()).into()),
        };

        match first_key.0.as_ref() {
            "$oid" => {
                let oid: ObjectId = map.next_value()?;
                Ok(RawBsonRef::ObjectId(oid).into())
            }
            "$symbol" => {
                let s: CowStr = map.next_value()?;
                match s.0 {
                    Cow::Borrowed(s) => Ok(RawBsonRef::Symbol(s).into()),
                    Cow::Owned(s) => Ok(RawBson::Symbol(s).into()),
                }
            }
            "$numberDecimalBytes" => {
                let bytes: ByteBuf = map.next_value()?;
                return Ok(RawBsonRef::Decimal128(Decimal128::deserialize_from_slice(
                    bytes.as_ref(),
                )?)
                .into());
            }
            "$regularExpression" => {
                let body: BorrowedRegexBody = map.next_value()?;

                match (body.pattern, body.options) {
                    (Cow::Borrowed(p), Cow::Borrowed(o)) => {
                        Ok(RawBsonRef::RegularExpression(RawRegexRef {
                            pattern: p,
                            options: o,
                        })
                        .into())
                    }
                    (p, o) => Ok(RawBson::RegularExpression(Regex {
                        pattern: p.into_owned(),
                        options: o.into_owned(),
                    })
                    .into()),
                }
            }
            "$undefined" => {
                let _: bool = map.next_value()?;
                Ok(RawBsonRef::Undefined.into())
            }
            "$binary" => {

                let v: BorrowedBinaryBody = map.next_value()?;

                if let Cow::Borrowed(bytes) = v.bytes {
                    Ok(RawBsonRef::Binary(RawBinaryRef {
                        bytes,
                        subtype: v.subtype.into(),
                    })
                    .into())
                } else {
                    Ok(RawBson::Binary(Binary {
                        bytes: v.bytes.into_owned(),
                        subtype: v.subtype.into(),
                    })
                    .into())
                }
            }
            "$date" => {
                let date: i64 = map.next_value()?;
                Ok(RawBsonRef::DateTime(DateTime::from_millis(date)).into())
            }
            "$timestamp" => {
                let timestamp: TimestampBody = map.next_value()?;
                Ok(RawBsonRef::Timestamp(Timestamp {
                    time: timestamp.t,
                    increment: timestamp.i,
                })
                .into())
            }
            "$minKey" => {
                let _ = map.next_value::<i32>()?;
                Ok(RawBsonRef::MinKey.into())
            }
            "$maxKey" => {
                let _ = map.next_value::<i32>()?;
                Ok(RawBsonRef::MaxKey.into())
            }
            "$code" => {
                let code = map.next_value::<CowStr>()?;
                if let Some(key) = map.next_key::<CowStr>()? {
                    if key.0.as_ref() == "$scope" {
                        let scope = map.next_value::<OwnedOrBorrowedRawDocument>()?;
                        match (code.0, scope) {
                            (Cow::Borrowed(code), OwnedOrBorrowedRawDocument::Borrowed(scope)) => {
                                Ok(RawBsonRef::JavaScriptCodeWithScope(
                                    RawJavaScriptCodeWithScopeRef { code, scope },
                                )
                                .into())
                            }
                            (code, scope) => Ok(RawBson::JavaScriptCodeWithScope(
                                RawJavaScriptCodeWithScope {
                                    code: code.into_owned(),
                                    scope: scope.into_owned(),
                                },
                            )
                            .into()),
                        }
                    } else {
                        Err(SerdeError::unknown_field(&key.0, &["$scope"]))
                    }
                } else if let Cow::Borrowed(code) = code.0 {
                    Ok(RawBsonRef::JavaScriptCode(code).into())
                } else {
                    Ok(RawBson::JavaScriptCode(code.0.into_owned()).into())
                }
            }
            "$dbPointer" => {
                let db_pointer: BorrowedDbPointerBody = map.next_value()?;
                if let Cow::Borrowed(ns) = db_pointer.ns.0 {
                    Ok(RawBsonRef::DbPointer(RawDbPointerRef {
                        namespace: ns,
                        id: db_pointer.id,
                    })
                    .into())
                } else {
                    Ok(RawBson::DbPointer(DbPointer {
                        namespace: db_pointer.ns.0.into_owned(),
                        id: db_pointer.id,
                    })
                    .into())
                }
            }
            RAW_DOCUMENT_NEWTYPE => {
                let bson = map.next_value::<&[u8]>()?;
                let doc = RawDocument::from_bytes(bson).map_err(SerdeError::custom)?;
                Ok(RawBsonRef::Document(doc).into())
            }
            RAW_ARRAY_NEWTYPE => {
                let bson = map.next_value::<&[u8]>()?;
                let doc = RawDocument::from_bytes(bson).map_err(SerdeError::custom)?;
                Ok(RawBsonRef::Array(RawArray::from_doc(doc)).into())
            }
            _ => {
                let mut buffer = CowByteBuffer::new();
                let seeded_visitor = SeededVisitor::new(&mut buffer);
                seeded_visitor.iterate_map(first_key, map)?;

                match OwnedOrBorrowedRawDocument::try_from(buffer).map_err(SerdeError::custom)? {
                    OwnedOrBorrowedRawDocument::Borrowed(borrowed) => {
                        Ok(RawBsonRef::Document(borrowed).into())
                    }
                    OwnedOrBorrowedRawDocument::Owned(owned) => Ok(RawBson::Document(owned).into()),
                }
            }
        }
    }
}
