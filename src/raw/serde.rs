use std::{borrow::Cow, fmt::Debug};

use serde::{de::Visitor, Deserialize};
use serde_bytes::ByteBuf;

use crate::{
    de::convert_unsigned_to_signed_raw,
    extjson,
    oid::ObjectId,
    raw::{RawJavaScriptCodeWithScope, RAW_ARRAY_NEWTYPE, RAW_DOCUMENT_NEWTYPE},
    spec::BinarySubtype,
    Binary,
    DateTime,
    DbPointer,
    Decimal128,
    RawArray,
    RawArrayBuf,
    RawBinaryRef,
    RawBsonRef,
    RawDbPointerRef,
    RawDocument,
    RawDocumentBuf,
    RawJavaScriptCodeWithScopeRef,
    RawRegexRef,
    Regex,
    Timestamp,
};

use super::{bson::RawBson, RAW_BSON_NEWTYPE};

/// A raw BSON value that may either be borrowed or owned.
///
/// This is used to consolidate the [`Serialize`] and [`Deserialize`] implementations for
/// [`RawBson`] and [`OwnedRawBson`].
pub(crate) enum OwnedOrBorrowedRawBson<'a> {
    Owned(RawBson),
    Borrowed(RawBsonRef<'a>),
}

impl<'a, 'de: 'a> Deserialize<'de> for OwnedOrBorrowedRawBson<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_newtype_struct(RAW_BSON_NEWTYPE, OwnedOrBorrowedRawBsonVisitor)
    }
}

impl<'a> Debug for OwnedOrBorrowedRawBson<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Owned(o) => o.fmt(f),
            Self::Borrowed(b) => b.fmt(f),
        }
    }
}

impl<'a> From<RawBsonRef<'a>> for OwnedOrBorrowedRawBson<'a> {
    fn from(b: RawBsonRef<'a>) -> Self {
        OwnedOrBorrowedRawBson::Borrowed(b)
    }
}

impl<'a> From<RawBson> for OwnedOrBorrowedRawBson<'a> {
    fn from(b: RawBson) -> Self {
        OwnedOrBorrowedRawBson::Owned(b)
    }
}

/// Wrapper around a `Cow<str>` to enable borrowed deserialization.
/// The default [`Deserialize`] impl for [`Cow`] always uses the owned version.
#[derive(Debug, Deserialize)]
struct CowStr<'a>(#[serde(borrow)] Cow<'a, str>);

/// Wrapper type that can deserialize either an owned or a borrowed raw BSON document.
#[derive(Debug)]
pub(crate) enum OwnedOrBorrowedRawDocument<'a> {
    Owned(RawDocumentBuf),
    Borrowed(&'a RawDocument),
}

impl<'a> OwnedOrBorrowedRawDocument<'a> {
    pub(crate) fn into_owned(self) -> RawDocumentBuf {
        match self {
            Self::Owned(o) => o,
            Self::Borrowed(b) => b.to_owned(),
        }
    }
}

impl<'a, 'de: 'a> Deserialize<'de> for OwnedOrBorrowedRawDocument<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match deserializer
            .deserialize_newtype_struct(RAW_DOCUMENT_NEWTYPE, OwnedOrBorrowedRawBsonVisitor)?
        {
            OwnedOrBorrowedRawBson::Borrowed(RawBsonRef::Document(d)) => Ok(Self::Borrowed(d)),
            OwnedOrBorrowedRawBson::Owned(RawBson::Document(d)) => Ok(Self::Owned(d)),

            // For non-BSON formats, RawDocument gets serialized as bytes, so we need to deserialize
            // from them here too. For BSON, the deserializier will return an error if it
            // sees the RAW_DOCUMENT_NEWTYPE but the next type isn't a document.
            OwnedOrBorrowedRawBson::Borrowed(RawBsonRef::Binary(b))
                if b.subtype == BinarySubtype::Generic =>
            {
                Ok(Self::Borrowed(
                    RawDocument::from_bytes(b.bytes).map_err(serde::de::Error::custom)?,
                ))
            }
            OwnedOrBorrowedRawBson::Owned(RawBson::Binary(b))
                if b.subtype == BinarySubtype::Generic =>
            {
                Ok(Self::Owned(
                    RawDocumentBuf::from_bytes(b.bytes).map_err(serde::de::Error::custom)?,
                ))
            }

            o => Err(serde::de::Error::custom(format!(
                "expected raw document, instead got {:?}",
                o
            ))),
        }
    }
}

/// Wrapper type that can deserialize either an owned or a borrowed raw BSON array.
#[derive(Debug)]
pub(crate) enum OwnedOrBorrowedRawArray<'a> {
    Owned(RawArrayBuf),
    Borrowed(&'a RawArray),
}

impl<'a> OwnedOrBorrowedRawArray<'a> {
    pub(crate) fn into_owned(self) -> RawArrayBuf {
        match self {
            Self::Owned(o) => o,
            Self::Borrowed(b) => b.to_owned(),
        }
    }
}

impl<'a, 'de: 'a> Deserialize<'de> for OwnedOrBorrowedRawArray<'a> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        match deserializer
            .deserialize_newtype_struct(RAW_ARRAY_NEWTYPE, OwnedOrBorrowedRawBsonVisitor)?
        {
            OwnedOrBorrowedRawBson::Borrowed(RawBsonRef::Array(d)) => Ok(Self::Borrowed(d)),
            OwnedOrBorrowedRawBson::Owned(RawBson::Array(d)) => Ok(Self::Owned(d)),

            // For non-BSON formats, RawArray gets serialized as bytes, so we need to deserialize
            // from them here too. For BSON, the deserializier will return an error if it
            // sees the RAW_DOCUMENT_NEWTYPE but the next type isn't a document.
            OwnedOrBorrowedRawBson::Borrowed(RawBsonRef::Binary(b))
                if b.subtype == BinarySubtype::Generic =>
            {
                let doc = RawDocument::from_bytes(b.bytes).map_err(serde::de::Error::custom)?;
                Ok(Self::Borrowed(RawArray::from_doc(doc)))
            }
            OwnedOrBorrowedRawBson::Owned(RawBson::Binary(b))
                if b.subtype == BinarySubtype::Generic =>
            {
                let doc = RawDocumentBuf::from_bytes(b.bytes).map_err(serde::de::Error::custom)?;
                Ok(Self::Owned(RawArrayBuf::from_raw_document_buf(doc)))
            }

            o => Err(serde::de::Error::custom(format!(
                "expected raw array, instead got {:?}",
                o
            ))),
        }
    }
}

/// A visitor used to deserialize types backed by raw BSON.
pub(crate) struct OwnedOrBorrowedRawBsonVisitor;

impl<'de> Visitor<'de> for OwnedOrBorrowedRawBsonVisitor {
    type Value = OwnedOrBorrowedRawBson<'de>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "a raw BSON value")
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBsonRef::String(v).into())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBson::String(v.to_string()).into())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBson::String(v).into())
    }

    fn visit_borrowed_bytes<E>(self, bytes: &'de [u8]) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBsonRef::Binary(RawBinaryRef {
            bytes,
            subtype: BinarySubtype::Generic,
        })
        .into())
    }

    fn visit_i8<E>(self, v: i8) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBsonRef::Int32(v.into()).into())
    }

    fn visit_i16<E>(self, v: i16) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBsonRef::Int32(v.into()).into())
    }

    fn visit_i32<E>(self, v: i32) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBsonRef::Int32(v).into())
    }

    fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBsonRef::Int64(v).into())
    }

    fn visit_u8<E>(self, value: u8) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(convert_unsigned_to_signed_raw(value.into())?.into())
    }

    fn visit_u16<E>(self, value: u16) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(convert_unsigned_to_signed_raw(value.into())?.into())
    }

    fn visit_u32<E>(self, value: u32) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(convert_unsigned_to_signed_raw(value.into())?.into())
    }

    fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(convert_unsigned_to_signed_raw(value)?.into())
    }

    fn visit_bool<E>(self, v: bool) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBsonRef::Boolean(v).into())
    }

    fn visit_f64<E>(self, v: f64) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBsonRef::Double(v).into())
    }

    fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBsonRef::Null.into())
    }

    fn visit_unit<E>(self) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBsonRef::Null.into())
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(self)
    }

    fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBson::Binary(Binary {
            bytes: v,
            subtype: BinarySubtype::Generic,
        })
        .into())
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut array = RawArrayBuf::new();
        while let Some(v) = seq.next_element::<RawBson>()? {
            array.push(v);
        }
        Ok(RawBson::Array(array).into())
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        /// Helper function used to build up the rest of a document once we determine that
        /// the map being visited isn't the serde data model version of a BSON type and is
        /// in fact a regular map.
        fn build_doc<'de, A>(
            first_key: &str,
            mut map: A,
        ) -> std::result::Result<OwnedOrBorrowedRawBson<'de>, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut doc = RawDocumentBuf::new();
            let v: RawBson = map.next_value()?;
            doc.append(first_key, v);

            while let Some((k, v)) = map.next_entry::<CowStr, RawBson>()? {
                doc.append(k.0, v);
            }

            Ok(RawBson::Document(doc).into())
        }

        let k = match map.next_key::<CowStr>()? {
            Some(k) => k,
            None => return Ok(RawBson::Document(RawDocumentBuf::new()).into()),
        };

        match k.0.as_ref() {
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
                let bytes = map.next_value::<ByteBuf>()?;
                return Ok(
                    RawBsonRef::Decimal128(Decimal128::deserialize_from_slice(&bytes)?).into(),
                );
            }
            "$regularExpression" => {
                #[derive(Debug, Deserialize)]
                struct BorrowedRegexBody<'a> {
                    #[serde(borrow)]
                    pattern: Cow<'a, str>,

                    #[serde(borrow)]
                    options: Cow<'a, str>,
                }
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
                #[derive(Debug, Deserialize)]
                struct BorrowedBinaryBody<'a> {
                    #[serde(borrow)]
                    bytes: Cow<'a, [u8]>,

                    #[serde(rename = "subType")]
                    subtype: u8,
                }

                let v = map.next_value::<BorrowedBinaryBody>()?;

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
                let v = map.next_value::<i64>()?;
                Ok(RawBsonRef::DateTime(DateTime::from_millis(v)).into())
            }
            "$timestamp" => {
                let v = map.next_value::<extjson::models::TimestampBody>()?;
                Ok(RawBsonRef::Timestamp(Timestamp {
                    time: v.t,
                    increment: v.i,
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
                        Err(serde::de::Error::unknown_field(&key.0, &["$scope"]))
                    }
                } else if let Cow::Borrowed(code) = code.0 {
                    Ok(RawBsonRef::JavaScriptCode(code).into())
                } else {
                    Ok(RawBson::JavaScriptCode(code.0.into_owned()).into())
                }
            }
            "$dbPointer" => {
                #[derive(Deserialize)]
                struct BorrowedDbPointerBody<'a> {
                    #[serde(rename = "$ref")]
                    #[serde(borrow)]
                    ns: CowStr<'a>,

                    #[serde(rename = "$id")]
                    id: ObjectId,
                }

                let body: BorrowedDbPointerBody = map.next_value()?;
                if let Cow::Borrowed(ns) = body.ns.0 {
                    Ok(RawBsonRef::DbPointer(RawDbPointerRef {
                        namespace: ns,
                        id: body.id,
                    })
                    .into())
                } else {
                    Ok(RawBson::DbPointer(DbPointer {
                        namespace: body.ns.0.into_owned(),
                        id: body.id,
                    })
                    .into())
                }
            }
            RAW_DOCUMENT_NEWTYPE => {
                let bson = map.next_value::<&[u8]>()?;
                let doc = RawDocument::from_bytes(bson).map_err(serde::de::Error::custom)?;
                Ok(RawBsonRef::Document(doc).into())
            }
            RAW_ARRAY_NEWTYPE => {
                let bson = map.next_value::<&[u8]>()?;
                let doc = RawDocument::from_bytes(bson).map_err(serde::de::Error::custom)?;
                Ok(RawBsonRef::Array(RawArray::from_doc(doc)).into())
            }
            k => build_doc(k, map),
        }
    }
}
