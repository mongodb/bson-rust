//! A module defining serde models for the extended JSON representations of the various BSON types.

#![cfg_attr(not(feature = "serde_json-1"), allow(unused))]

use serde::{
    de::{Error as _, Unexpected},
    Deserialize,
    Serialize,
    Serializer,
};
use std::{borrow::Cow, result::Result as StdResult};

use crate::{
    base64,
    error::{Error, Result},
    oid,
    raw::serde::CowStr,
    spec::BinarySubtype,
    Bson,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Int32 {
    #[serde(rename = "$numberInt")]
    value: String,
}

impl Int32 {
    pub(crate) fn parse(self) -> Result<i32> {
        self.value.parse().map_err(|_| {
            Error::invalid_value(Unexpected::Str(self.value.as_str()), &"i32 as a string")
        })
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Int64 {
    #[serde(rename = "$numberLong")]
    value: String,
}

impl Int64 {
    pub(crate) fn parse(self) -> Result<i64> {
        self.value.parse().map_err(|_| {
            Error::invalid_value(Unexpected::Str(self.value.as_str()), &"i64 as a string")
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Double {
    #[serde(rename = "$numberDouble")]
    value: String,
}

impl Double {
    pub(crate) fn parse(self) -> Result<f64> {
        match self.value.as_str() {
            "Infinity" => Ok(f64::INFINITY),
            "-Infinity" => Ok(f64::NEG_INFINITY),
            "NaN" => Ok(f64::NAN),
            other => other.parse().map_err(|_| {
                Error::invalid_value(Unexpected::Str(other), &"bson double as string")
            }),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Decimal128 {
    #[serde(rename = "$numberDecimal")]
    value: String,
}

impl Decimal128 {
    pub(crate) fn parse(self) -> Result<crate::Decimal128> {
        self.value.parse().map_err(|_| {
            Error::invalid_value(Unexpected::Str(&self.value), &"bson decimal128 as string")
        })
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObjectId {
    #[serde(rename = "$oid")]
    oid: String,
}

impl ObjectId {
    pub(crate) fn parse(self) -> Result<oid::ObjectId> {
        oid::ObjectId::parse_str(self.oid.as_str())
    }
}

impl From<crate::oid::ObjectId> for ObjectId {
    fn from(id: crate::oid::ObjectId) -> Self {
        Self { oid: id.to_hex() }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Symbol {
    #[serde(rename = "$symbol")]
    pub(crate) value: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Regex {
    #[serde(rename = "$regularExpression")]
    body: RegexBody,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RegexBody {
    pub(crate) pattern: String,
    pub(crate) options: String,
}

impl Regex {
    pub(crate) fn parse(self) -> crate::error::Result<crate::Regex> {
        crate::Regex::from_strings(self.body.pattern, self.body.options)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Binary {
    #[serde(rename = "$binary")]
    pub(crate) body: BinaryBody,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BinaryBody {
    pub(crate) base64: String,

    #[serde(rename = "subType")]
    pub(crate) subtype: String,
}

impl Binary {
    pub(crate) fn parse(self) -> Result<crate::Binary> {
        let bytes = base64::decode(self.body.base64.as_str()).map_err(|_| {
            Error::invalid_value(
                Unexpected::Str(self.body.base64.as_str()),
                &"base64 encoded bytes",
            )
        })?;

        let subtype = hex::decode(self.body.subtype.as_str()).map_err(|_| {
            Error::invalid_value(
                Unexpected::Str(self.body.subtype.as_str()),
                &"hexadecimal number as a string",
            )
        })?;

        if subtype.len() == 1 {
            Ok(crate::Binary {
                bytes,
                subtype: subtype[0].into(),
            })
        } else {
            Err(Error::invalid_value(
                Unexpected::Bytes(subtype.as_slice()),
                &"one byte subtype",
            ))
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Uuid {
    #[serde(rename = "$uuid")]
    pub(crate) value: String,
}

impl Uuid {
    pub(crate) fn parse(self) -> Result<crate::Binary> {
        let uuid = uuid::Uuid::parse_str(&self.value).map_err(|_| {
            Error::invalid_value(
                Unexpected::Str(&self.value),
                &"$uuid value does not follow RFC 4122 format regarding length and hyphens",
            )
        })?;

        Ok(crate::Binary {
            subtype: BinarySubtype::Uuid,
            bytes: uuid.as_bytes().to_vec(),
        })
    }
}

#[cfg(feature = "serde_json-1")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct JavaScriptCodeWithScope {
    #[serde(rename = "$code")]
    pub(crate) code: String,

    #[serde(rename = "$scope")]
    #[serde(default)]
    pub(crate) scope: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Timestamp {
    #[serde(rename = "$timestamp")]
    body: TimestampBody,
}

/// Serializes a u32 as an i64.
fn serialize_u32_as_i64<S: Serializer>(val: &u32, serializer: S) -> StdResult<S::Ok, S::Error> {
    serializer.serialize_i64(*val as i64)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct TimestampBody {
    #[serde(serialize_with = "serialize_u32_as_i64")]
    pub(crate) t: u32,

    #[serde(serialize_with = "serialize_u32_as_i64")]
    pub(crate) i: u32,
}

impl Timestamp {
    pub(crate) fn parse(self) -> crate::Timestamp {
        crate::Timestamp {
            time: self.body.t,
            increment: self.body.i,
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DateTime {
    #[serde(rename = "$date")]
    pub(crate) body: DateTimeBody,
}

#[derive(Deserialize, Serialize)]
#[serde(untagged)]
pub(crate) enum DateTimeBody {
    Canonical(Int64),
    Relaxed(String),
    Legacy(i64),
}

impl DateTimeBody {
    pub(crate) fn from_millis(m: i64) -> Self {
        DateTimeBody::Canonical(Int64 {
            value: m.to_string(),
        })
    }
}

impl DateTime {
    pub(crate) fn parse(self) -> Result<crate::DateTime> {
        match self.body {
            DateTimeBody::Canonical(date) => {
                let date = date.parse()?;
                Ok(crate::DateTime::from_millis(date))
            }
            DateTimeBody::Relaxed(date) => {
                let datetime = crate::DateTime::parse_rfc3339_str(date.as_str()).map_err(|_| {
                    Error::invalid_value(
                        Unexpected::Str(date.as_str()),
                        &"rfc3339 formatted utc datetime",
                    )
                })?;
                Ok(datetime)
            }
            DateTimeBody::Legacy(ms) => Ok(crate::DateTime::from_millis(ms)),
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MinKey {
    #[serde(rename = "$minKey")]
    pub(crate) value: u8,
}

impl MinKey {
    pub(crate) fn parse(self) -> Result<Bson> {
        if self.value == 1 {
            Ok(Bson::MinKey)
        } else {
            Err(Error::invalid_value(
                Unexpected::Unsigned(self.value as u64),
                &"value of $minKey should always be 1",
            ))
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MaxKey {
    #[serde(rename = "$maxKey")]
    pub(crate) value: u8,
}

impl MaxKey {
    pub(crate) fn parse(self) -> Result<Bson> {
        if self.value == 1 {
            Ok(Bson::MaxKey)
        } else {
            Err(Error::invalid_value(
                Unexpected::Unsigned(self.value as u64),
                &"value of $maxKey should always be 1",
            ))
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DbPointer {
    #[serde(rename = "$dbPointer")]
    body: DbPointerBody,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DbPointerBody {
    #[serde(rename = "$ref")]
    pub(crate) ref_ns: String,

    #[serde(rename = "$id")]
    pub(crate) id: ObjectId,
}

impl DbPointer {
    pub(crate) fn parse(self) -> Result<crate::DbPointer> {
        Ok(crate::DbPointer {
            namespace: self.body.ref_ns,
            id: self.body.id.parse()?,
        })
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Undefined {
    #[serde(rename = "$undefined")]
    pub(crate) value: bool,
}

impl Undefined {
    pub(crate) fn parse(self) -> Result<Bson> {
        if self.value {
            Ok(Bson::Undefined)
        } else {
            Err(Error::invalid_value(
                Unexpected::Bool(false),
                &"$undefined should always be true",
            ))
        }
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct BorrowedRegexBody<'a> {
    #[serde(borrow)]
    pub(crate) pattern: Cow<'a, str>,

    #[serde(borrow)]
    pub(crate) options: Cow<'a, str>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct BorrowedBinaryBody<'a> {
    #[serde(borrow)]
    pub(crate) bytes: Cow<'a, [u8]>,

    #[serde(rename = "subType")]
    pub(crate) subtype: u8,
}

#[derive(Deserialize)]
pub(crate) struct BorrowedDbPointerBody<'a> {
    #[serde(rename = "$ref")]
    #[serde(borrow)]
    pub(crate) ns: CowStr<'a>,

    #[serde(rename = "$id")]
    pub(crate) id: oid::ObjectId,
}
