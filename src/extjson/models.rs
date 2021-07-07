//! A module defining serde models for the extended JSON representations of the various BSON types.

use serde::{
    de::{Error, Unexpected},
    Deserialize,
};

use crate::{extjson, oid, spec::BinarySubtype, Bson};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Int32 {
    #[serde(rename = "$numberInt")]
    value: String,
}

impl Int32 {
    pub(crate) fn parse(self) -> extjson::de::Result<i32> {
        let i: i32 = self.value.parse().map_err(|_| {
            extjson::de::Error::invalid_value(
                Unexpected::Str(self.value.as_str()),
                &"expected i32 as a string",
            )
        })?;
        Ok(i)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Int64 {
    #[serde(rename = "$numberLong")]
    value: String,
}

impl Int64 {
    pub(crate) fn parse(self) -> extjson::de::Result<i64> {
        let i: i64 = self.value.parse().map_err(|_| {
            extjson::de::Error::invalid_value(
                Unexpected::Str(self.value.as_str()),
                &"expected i64 as a string",
            )
        })?;
        Ok(i)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Double {
    #[serde(rename = "$numberDouble")]
    value: String,
}

impl Double {
    pub(crate) fn parse(self) -> extjson::de::Result<f64> {
        match self.value.as_str() {
            "Infinity" => Ok(std::f64::INFINITY),
            "-Infinity" => Ok(std::f64::NEG_INFINITY),
            "NaN" => Ok(std::f64::NAN),
            other => {
                let d: f64 = other.parse().map_err(|_| {
                    extjson::de::Error::invalid_value(
                        Unexpected::Str(other),
                        &"expected bson double as string",
                    )
                })?;
                Ok(d)
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ObjectId {
    #[serde(rename = "$oid")]
    oid: String,
}

impl ObjectId {
    pub(crate) fn parse(self) -> extjson::de::Result<oid::ObjectId> {
        let oid = oid::ObjectId::parse_str(self.oid.as_str())?;
        Ok(oid)
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RegexBody {
    pattern: String,
    options: String,
}

impl Regex {
    pub(crate) fn parse(self) -> crate::Regex {
        let mut chars: Vec<_> = self.body.options.chars().collect();
        chars.sort_unstable();
        let options: String = chars.into_iter().collect();

        crate::Regex {
            pattern: self.body.pattern,
            options,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Binary {
    #[serde(rename = "$binary")]
    body: BinaryBody,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BinaryBody {
    base64: String,
    #[serde(rename = "subType")]
    subtype: String,
}

impl Binary {
    pub(crate) fn parse(self) -> extjson::de::Result<crate::Binary> {
        let bytes = base64::decode(self.body.base64.as_str()).map_err(|_| {
            extjson::de::Error::invalid_value(
                Unexpected::Str(self.body.base64.as_str()),
                &"base64 encoded bytes",
            )
        })?;

        let subtype = hex::decode(self.body.subtype.as_str()).map_err(|_| {
            extjson::de::Error::invalid_value(
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
            Err(extjson::de::Error::invalid_value(
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
    value: String,
}

impl Uuid {
    pub(crate) fn parse(self) -> extjson::de::Result<crate::Binary> {
        let uuid = uuid::Uuid::parse_str(&self.value).map_err(|_| {
            extjson::de::Error::invalid_value(
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TimestampBody {
    t: u32,
    i: u32,
}

impl Timestamp {
    pub(crate) fn parse(self) -> crate::Timestamp {
        crate::Timestamp {
            time: self.body.t,
            increment: self.body.i,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DateTime {
    #[serde(rename = "$date")]
    body: DateTimeBody,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum DateTimeBody {
    Canonical(Int64),
    Relaxed(String),
}

impl DateTime {
    pub(crate) fn parse(self) -> extjson::de::Result<crate::DateTime> {
        match self.body {
            DateTimeBody::Canonical(date) => {
                let date = date.parse()?;
                Ok(crate::DateTime::from_millis(date))
            }
            DateTimeBody::Relaxed(date) => {
                let datetime = crate::DateTime::parse_rfc3339(&date).map_err(|_| {
                    extjson::de::Error::invalid_value(
                        Unexpected::Str(date.as_str()),
                        &"rfc3339 formatted utc datetime",
                    )
                })?;
                Ok(datetime)
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MinKey {
    #[serde(rename = "$minKey")]
    value: u8,
}

impl MinKey {
    pub(crate) fn parse(self) -> extjson::de::Result<Bson> {
        if self.value == 1 {
            Ok(Bson::MinKey)
        } else {
            Err(extjson::de::Error::invalid_value(
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
    value: u8,
}

impl MaxKey {
    pub(crate) fn parse(self) -> extjson::de::Result<Bson> {
        if self.value == 1 {
            Ok(Bson::MaxKey)
        } else {
            Err(extjson::de::Error::invalid_value(
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

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DbPointerBody {
    #[serde(rename = "$ref")]
    ref_ns: String,

    #[serde(rename = "$id")]
    id: ObjectId,
}

impl DbPointer {
    pub(crate) fn parse(self) -> extjson::de::Result<crate::DbPointer> {
        Ok(crate::DbPointer {
            namespace: self.body.ref_ns,
            id: self.body.id.parse()?,
        })
    }
}

#[cfg(feature = "decimal128")]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Decimal128 {
    #[serde(rename = "$numberDecimal")]
    value: String,
}

#[cfg(feature = "decimal128")]
impl Decimal128 {
    pub(crate) fn parse(self) -> extjson::de::Result<crate::Decimal128> {
        let decimal128: crate::Decimal128 = self.value.parse().map_err(|_| {
            extjson::de::Error::invalid_value(
                Unexpected::Str(self.value.as_str()),
                &"decimal128 value as a string",
            )
        })?;
        Ok(decimal128)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Undefined {
    #[serde(rename = "$undefined")]
    value: bool,
}

impl Undefined {
    pub(crate) fn parse(self) -> extjson::de::Result<Bson> {
        if self.value {
            Ok(Bson::Undefined)
        } else {
            Err(extjson::de::Error::invalid_value(
                Unexpected::Bool(false),
                &"$undefined should always be true",
            ))
        }
    }
}
