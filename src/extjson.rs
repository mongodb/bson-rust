use crate::{oid, Bson, DecoderError, DecoderResult};
use chrono::{TimeZone, Utc};
use serde::{
    de::{Error, Unexpected},
    Deserialize,
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Int32 {
    #[serde(rename = "$numberInt")]
    value: String,
}

impl Int32 {
    pub(crate) fn parse(self) -> DecoderResult<i32> {
        let i: i32 = self.value.parse().map_err(|_| {
            DecoderError::invalid_value(
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
    pub(crate) fn parse(self) -> DecoderResult<i64> {
        let i: i64 = self.value.parse().map_err(|_| {
            DecoderError::invalid_value(
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
    pub(crate) fn parse(self) -> DecoderResult<f64> {
        match self.value.as_str() {
            "Infinity" => Ok(f64::INFINITY),
            "-Infinity" => Ok(f64::NEG_INFINITY),
            "NaN" => Ok(f64::NAN),
            other => {
                let d: f64 = other.parse().map_err(|_| {
                    DecoderError::invalid_value(
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
    pub(crate) fn parse(self) -> DecoderResult<oid::ObjectId> {
        let oid = oid::ObjectId::with_string(self.oid.as_str())?;
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
    pub(crate) fn parse(self) -> DecoderResult<crate::Regex> {
        let mut chars: Vec<_> = self.body.options.chars().collect();
        chars.sort();
        let options: String = chars.into_iter().collect();

        Ok(crate::Regex {
            pattern: self.body.pattern,
            options,
        })
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
    pub(crate) fn parse(self) -> DecoderResult<crate::Binary> {
        let bytes = base64::decode(self.body.base64.as_str()).map_err(|_| {
            DecoderError::invalid_value(
                Unexpected::Str(self.body.base64.as_str()),
                &"base64 encoded bytes",
            )
        })?;

        let subtype = hex::decode(self.body.subtype.as_str()).map_err(|_| {
            DecoderError::invalid_value(
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
            Err(DecoderError::invalid_value(
                Unexpected::Bytes(subtype.as_slice()),
                &"one byte subtype",
            ))
        }
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
    pub(crate) fn parse(self) -> DecoderResult<crate::TimeStamp> {
        Ok(crate::TimeStamp {
            time: self.body.t,
            increment: self.body.i,
        }
        .into())
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
    pub(crate) fn parse(self) -> DecoderResult<crate::UtcDateTime> {
        match self.body {
            DateTimeBody::Canonical(date) => {
                let date = date.parse()?;

                let mut num_secs = date / 1000;
                let mut num_millis = date % 1000;

                // The chrono API only lets us create a DateTime with an i64 number of seconds
                // and a u32 number of nanoseconds. In the case of a negative timestamp, this
                // means that we need to turn the negative fractional part into a positive and
                // shift the number of seconds down. For example:
                //
                //     date       = -4300 ms
                //     num_secs   = date / 1000 = -4300 / 1000 = -4
                //     num_millis = date % 1000 = -4300 % 1000 = -300
                //
                // Since num_millis is less than 0:
                //     num_secs   = num_secs -1 = -4 - 1 = -5
                //     num_millis = num_nanos + 1000 = -300 + 1000 = 700
                //
                // Instead of -4 seconds and -300 milliseconds, we now have -5 seconds and +700
                // milliseconds, which expresses the same timestamp, but in a way we can create
                // a DateTime with.
                if num_millis < 0 {
                    num_secs -= 1;
                    num_millis += 1000;
                };

                return Ok(Utc
                    .timestamp(num_secs, num_millis as u32 * 1_000_000)
                    .into());
            }
            DateTimeBody::Relaxed(date) => {
                let datetime: chrono::DateTime<Utc> =
                    chrono::DateTime::parse_from_rfc3339(date.as_str())
                        .map_err(|_| {
                            DecoderError::invalid_value(
                                Unexpected::Str(date.as_str()),
                                &"rfc3339 formatted utc datetime",
                            )
                        })?
                        .into();
                return Ok(datetime.into());
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
    pub(crate) fn parse(self) -> DecoderResult<Bson> {
        if self.value == 1 {
            Ok(Bson::MinKey)
        } else {
            Err(DecoderError::invalid_value(
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
    pub(crate) fn parse(self) -> DecoderResult<Bson> {
        if self.value == 1 {
            Ok(Bson::MaxKey)
        } else {
            Err(DecoderError::invalid_value(
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
    pub(crate) fn parse(self) -> DecoderResult<crate::DbPointer> {
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
    pub(crate) fn parse(self) -> DecoderResult<crate::Decimal128> {
        let decimal128: crate::Decimal128 = self.value.parse().map_err(|_| {
            DecoderError::invalid_value(
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
    pub(crate) fn parse(self) -> DecoderResult<Bson> {
        if self.value {
            Ok(Bson::Undefined)
        } else {
            Err(DecoderError::invalid_value(
                Unexpected::Bool(false),
                &"$undefined should always be true",
            ))
        }
    }
}
