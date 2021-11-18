use std::{borrow::Cow, fmt::Debug};

use serde::{de::Visitor, Deserialize};
use serde_bytes::ByteBuf;

use crate::{
    de::convert_unsigned_to_signed_raw,
    extjson,
    oid::ObjectId,
    raw::{OwnedRawJavaScriptCodeWithScope, RAW_ARRAY_NEWTYPE, RAW_DOCUMENT_NEWTYPE},
    spec::BinarySubtype,
    Binary,
    DateTime,
    DbPointer,
    Decimal128,
    RawArray,
    RawArrayBuf,
    RawBinary,
    RawBson,
    RawDbPointer,
    RawDocument,
    RawDocumentBuf,
    RawJavaScriptCodeWithScope,
    RawRegex,
    Regex,
    Timestamp,
};

use super::{owned_bson::OwnedRawBson, RAW_BSON_NEWTYPE};

pub(crate) enum OwnedOrBorrowedRawBson<'a> {
    Owned(OwnedRawBson),
    Borrowed(RawBson<'a>),
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

impl<'a> From<RawBson<'a>> for OwnedOrBorrowedRawBson<'a> {
    fn from(b: RawBson<'a>) -> Self {
        OwnedOrBorrowedRawBson::Borrowed(b)
    }
}

impl<'a> From<OwnedRawBson> for OwnedOrBorrowedRawBson<'a> {
    fn from(b: OwnedRawBson) -> Self {
        OwnedOrBorrowedRawBson::Owned(b)
    }
}

#[derive(Debug, Deserialize)]
struct CowStr<'a>(#[serde(borrow)] Cow<'a, str>);

#[derive(Debug, Deserialize)]
struct CowRawDocument<'a>(#[serde(borrow)] Cow<'a, RawDocument>);

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
        Ok(RawBson::String(v).into())
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(OwnedRawBson::String(v.to_string()).into())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(OwnedRawBson::String(v).into())
    }

    fn visit_borrowed_bytes<E>(self, bytes: &'de [u8]) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBson::Binary(RawBinary {
            bytes,
            subtype: BinarySubtype::Generic,
        })
        .into())
    }

    fn visit_i8<E>(self, v: i8) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBson::Int32(v.into()).into())
    }

    fn visit_i16<E>(self, v: i16) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBson::Int32(v.into()).into())
    }

    fn visit_i32<E>(self, v: i32) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBson::Int32(v).into())
    }

    fn visit_i64<E>(self, v: i64) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBson::Int64(v).into())
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
        Ok(RawBson::Boolean(v).into())
    }

    fn visit_f64<E>(self, v: f64) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBson::Double(v).into())
    }

    fn visit_none<E>(self) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBson::Null.into())
    }

    fn visit_unit<E>(self) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(RawBson::Null.into())
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
        Ok(OwnedRawBson::Binary(Binary {
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
        while let Some(v) = seq.next_element::<OwnedRawBson>()? {
            array.push(v);
        }
        Ok(OwnedRawBson::Array(array).into())
    }

    fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        fn build_doc<'de, A>(
            first_key: &str,
            mut map: A,
        ) -> std::result::Result<OwnedOrBorrowedRawBson<'de>, A::Error>
        where
            A: serde::de::MapAccess<'de>,
        {
            let mut doc = RawDocumentBuf::new();
            let v: OwnedRawBson = map.next_value()?;
            doc.append(first_key, v);

            while let Some((k, v)) = map.next_entry::<String, OwnedRawBson>()? {
                doc.append(k, v);
            }

            Ok(OwnedRawBson::Document(doc).into())
        }

        println!("deserializing cow");
        let k = match map.next_key::<CowStr>()? {
            Some(k) => k,
            None => return Ok(OwnedRawBson::Document(RawDocumentBuf::new()).into()),
        };
        println!("deserialized {}", k.0);

        match k.0 {
            Cow::Borrowed(_) => println!("borrowed"),
            _ => println!("not borrowed"),
        };

        match k.0.as_ref() {
            "$oid" => {
                let oid: ObjectId = map.next_value()?;
                Ok(RawBson::ObjectId(oid).into())
            }
            "$symbol" => {
                let s: CowStr = map.next_value()?;
                match s.0 {
                    Cow::Borrowed(s) => Ok(RawBson::Symbol(s).into()),
                    Cow::Owned(s) => Ok(OwnedRawBson::Symbol(s).into()),
                }
            }
            "$numberDecimalBytes" => {
                let bytes = map.next_value::<ByteBuf>()?;
                return Ok(RawBson::Decimal128(Decimal128::deserialize_from_slice(&bytes)?).into());
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
                        Ok(RawBson::RegularExpression(RawRegex {
                            pattern: p,
                            options: o,
                        })
                        .into())
                    }
                    (p, o) => Ok(OwnedRawBson::RegularExpression(Regex {
                        pattern: p.into_owned(),
                        options: o.into_owned(),
                    })
                    .into()),
                }
            }
            "$undefined" => {
                let _: bool = map.next_value()?;
                Ok(RawBson::Undefined.into())
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
                    Ok(RawBson::Binary(RawBinary {
                        bytes,
                        subtype: v.subtype.into(),
                    })
                    .into())
                } else {
                    Ok(OwnedRawBson::Binary(Binary {
                        bytes: v.bytes.into_owned(),
                        subtype: v.subtype.into(),
                    })
                    .into())
                }
            }
            "$date" => {
                let v = map.next_value::<i64>()?;
                Ok(RawBson::DateTime(DateTime::from_millis(v)).into())
            }
            "$timestamp" => {
                let v = map.next_value::<extjson::models::TimestampBody>()?;
                Ok(RawBson::Timestamp(Timestamp {
                    time: v.t,
                    increment: v.i,
                })
                .into())
            }
            "$minKey" => {
                let _ = map.next_value::<i32>()?;
                Ok(RawBson::MinKey.into())
            }
            "$maxKey" => {
                let _ = map.next_value::<i32>()?;
                Ok(RawBson::MaxKey.into())
            }
            "$code" => {
                let code = map.next_value::<CowStr>()?;
                if let Some(key) = map.next_key::<CowStr>()? {
                    if key.0.as_ref() == "$scope" {
                        let scope = map.next_value::<OwnedOrBorrowedRawBson>()?;
                        match (code.0, scope) {
                            (
                                Cow::Borrowed(code),
                                OwnedOrBorrowedRawBson::Borrowed(RawBson::Document(scope)),
                            ) => Ok(
                                RawBson::JavaScriptCodeWithScope(RawJavaScriptCodeWithScope {
                                    code,
                                    scope,
                                })
                                .into(),
                            ),
                            (
                                Cow::Owned(code),
                                OwnedOrBorrowedRawBson::Owned(OwnedRawBson::Document(scope)),
                            ) => Ok(OwnedRawBson::JavaScriptCodeWithScope(
                                OwnedRawJavaScriptCodeWithScope { code, scope },
                            )
                            .into()),
                            (code, scope) => Err(serde::de::Error::custom(format!(
                                "invalid code_w_scope: code: {:?}, scope: {:?}",
                                code, scope
                            ))),
                        }
                    } else {
                        Err(serde::de::Error::unknown_field(&key.0, &["$scope"]))
                    }
                } else if let Cow::Borrowed(code) = code.0 {
                    Ok(RawBson::JavaScriptCode(code).into())
                } else {
                    Ok(OwnedRawBson::JavaScriptCode(code.0.into_owned()).into())
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
                    Ok(RawBson::DbPointer(RawDbPointer {
                        namespace: ns,
                        id: body.id,
                    })
                    .into())
                } else {
                    Ok(OwnedRawBson::DbPointer(DbPointer {
                        namespace: body.ns.0.into_owned(),
                        id: body.id,
                    })
                    .into())
                }
            }
            RAW_DOCUMENT_NEWTYPE => {
                let bson = map.next_value::<&[u8]>()?;
                let doc = RawDocument::new(bson).map_err(serde::de::Error::custom)?;
                Ok(RawBson::Document(doc).into())
            }
            RAW_ARRAY_NEWTYPE => {
                let bson = map.next_value::<&[u8]>()?;
                let doc = RawDocument::new(bson).map_err(serde::de::Error::custom)?;
                Ok(RawBson::Array(RawArray::from_doc(doc)).into())
            }
            k => build_doc(k, map),
        }
    }
}
