//! A module defining struct models for the extended JSON representations of the various BSON types.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize, Serializer};
#[cfg(feature = "serde")]
use std::borrow::Cow;

#[cfg(feature = "serde")]
use crate::raw::serde::CowStr;
use crate::{Bson, base64, error::Result, oid, spec::BinarySubtype};

// BSON types represented by objects in extended JSON.
pub(crate) enum ObjectType {
    ObjectId,
    Symbol,
    Int32,
    Int64,
    Double,
    Decimal128,
    Binary,
    JavaScriptCode,
    JavaScriptCodeWithScope,
    Timestamp,
    RegularExpression,
    DbPointer,
    DateTime,
    MinKey,
    MaxKey,
    Undefined,
    Uuid,
    Document,
}

impl ObjectType {
    pub(crate) fn from_keys(keys: &[&str]) -> Self {
        match keys {
            ["$oid"] => Self::ObjectId,
            ["$symbol"] => Self::Symbol,
            ["$numberInt"] => Self::Int32,
            ["$numberLong"] => Self::Int64,
            ["$numberDouble"] => Self::Double,
            ["$numberDecimal"] => Self::Decimal128,
            ["$binary"] => Self::Binary,
            ["$code"] => Self::JavaScriptCode,
            ["$code", "$scope"] | ["$scope", "$code"] => Self::JavaScriptCodeWithScope,
            ["$timestamp"] => Self::Timestamp,
            ["$regularExpression"] => Self::RegularExpression,
            ["$dbPointer"] => Self::DbPointer,
            ["$date"] => Self::DateTime,
            ["$minKey"] => Self::MinKey,
            ["$maxKey"] => Self::MaxKey,
            ["$undefined"] => Self::Undefined,
            ["$uuid"] => Self::Uuid,
            _ => Self::Document,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct Int32 {
    #[cfg_attr(feature = "serde", serde(rename = "$numberInt"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$numberInt"))]
    value: String,
}

impl Int32 {
    pub(crate) fn parse(self) -> Result<i32> {
        self.value
            .parse()
            .map_err(|e| parse_err!("failed to parse i32 as a string: {e}"))
    }
}

impl From<&i32> for Int32 {
    fn from(value: &i32) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub(crate) struct Int64 {
    #[cfg_attr(feature = "serde", serde(rename = "$numberLong"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$numberLong"))]
    value: String,
}

impl Int64 {
    pub(crate) fn parse(self) -> Result<i64> {
        self.value
            .parse()
            .map_err(|e| parse_err!("failed to parse i64 as a string: {e}"))
    }
}

impl From<&i64> for Int64 {
    fn from(value: &i64) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct Double {
    #[cfg_attr(feature = "serde", serde(rename = "$numberDouble"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$numberDouble"))]
    value: String,
}

impl From<&f64> for Double {
    fn from(value: &f64) -> Self {
        let s = if value.is_nan() {
            "NaN".to_string()
        } else if *value == f64::INFINITY {
            "Infinity".to_string()
        } else if *value == f64::NEG_INFINITY {
            "-Infinity".to_string()
        } else {
            value.to_string()
        };
        Self { value: s }
    }
}

impl Double {
    pub(crate) fn parse(self) -> Result<f64> {
        match self.value.as_str() {
            "Infinity" => Ok(f64::INFINITY),
            "-Infinity" => Ok(f64::NEG_INFINITY),
            "NaN" => Ok(f64::NAN),
            other => other
                .parse()
                .map_err(|_| parse_err!("invalid bson double: {}", other)),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct Decimal128 {
    #[cfg_attr(feature = "serde", serde(rename = "$numberDecimal"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$numberDecimal"))]
    value: String,
}

impl From<&crate::Decimal128> for Decimal128 {
    fn from(value: &crate::Decimal128) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

impl Decimal128 {
    pub(crate) fn parse(self) -> Result<crate::Decimal128> {
        self.value
            .parse()
            .map_err(|_| parse_err!("invalid bson decimal128: {}", self.value))
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct ObjectId {
    #[cfg_attr(feature = "serde", serde(rename = "$oid"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$oid"))]
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

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct Symbol {
    #[cfg_attr(feature = "serde", serde(rename = "$symbol"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$symbol"))]
    pub(crate) value: String,
}

impl From<String> for Symbol {
    fn from(value: String) -> Self {
        Self { value }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct Regex {
    #[cfg_attr(feature = "serde", serde(rename = "$regularExpression"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$regularExpression"))]
    body: RegexBody,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct RegexBody {
    pub(crate) pattern: String,
    pub(crate) options: String,
}

impl From<&crate::Regex> for Regex {
    fn from(r: &crate::Regex) -> Self {
        Self {
            body: RegexBody {
                pattern: r.pattern.to_string(),
                options: r.options.to_string(),
            },
        }
    }
}

impl Regex {
    pub(crate) fn parse(self) -> crate::error::Result<crate::Regex> {
        crate::Regex::from_strings(self.body.pattern, self.body.options)
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct Binary {
    #[cfg_attr(feature = "serde", serde(rename = "$binary"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$binary"))]
    pub(crate) body: BinaryBody,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct BinaryBody {
    pub(crate) base64: String,

    #[cfg_attr(feature = "serde", serde(rename = "subType"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "subType"))]
    pub(crate) subtype: String,
}

impl From<&crate::Binary> for Binary {
    fn from(b: &crate::Binary) -> Self {
        Self {
            body: BinaryBody {
                base64: crate::base64::encode(&b.bytes),
                subtype: format!("{:02x}", u8::from(b.subtype)),
            },
        }
    }
}

impl Binary {
    pub(crate) fn parse(self) -> Result<crate::Binary> {
        let bytes = base64::decode(self.body.base64.as_str())
            .map_err(|_| parse_err!("invalid base64 encoded bytes: {}", self.body.base64))?;

        let subtype = hex::decode(self.body.subtype.as_str())
            .map_err(|_| parse_err!("invalid hexadecimal subtype: {}", self.body.subtype))?;

        if subtype.len() == 1 {
            Ok(crate::Binary {
                bytes,
                subtype: subtype[0].into(),
            })
        } else {
            Err(parse_err!(
                "binary subtype must be one byte, got {}",
                subtype.len()
            ))
        }
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct Uuid {
    #[cfg_attr(feature = "serde", serde(rename = "$uuid"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$uuid"))]
    pub(crate) value: String,
}

impl Uuid {
    pub(crate) fn parse(self) -> Result<crate::Binary> {
        let uuid = uuid::Uuid::parse_str(&self.value).map_err(|_| {
            parse_err!(
                "$uuid value does not follow RFC 4122 format regarding length and hyphens: {}",
                self.value
            )
        })?;

        Ok(crate::Binary {
            subtype: BinarySubtype::Uuid,
            bytes: uuid.as_bytes().to_vec(),
        })
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct JavaScriptCode {
    #[cfg_attr(feature = "serde", serde(rename = "$code"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$code"))]
    pub(crate) code: String,
}

impl From<&str> for JavaScriptCode {
    fn from(s: &str) -> Self {
        Self {
            code: s.to_string(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct JavaScriptCodeWithScope<Scope> {
    #[cfg_attr(feature = "serde", serde(rename = "$code"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$code"))]
    pub(crate) code: String,

    #[cfg_attr(feature = "serde", serde(rename = "$scope"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$scope"))]
    pub(crate) scope: Scope,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct Timestamp {
    #[cfg_attr(feature = "serde", serde(rename = "$timestamp"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$timestamp"))]
    body: TimestampBody,
}

/// Serializes a u32 as an i64.
#[cfg(feature = "serde")]
fn serialize_u32_as_i64<S: Serializer>(
    val: &u32,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.serialize_i64(*val as i64)
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct TimestampBody {
    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_u32_as_i64"))]
    pub(crate) t: u32,

    #[cfg_attr(feature = "serde", serde(serialize_with = "serialize_u32_as_i64"))]
    pub(crate) i: u32,
}

impl From<crate::Timestamp> for Timestamp {
    fn from(ts: crate::Timestamp) -> Self {
        Self {
            body: TimestampBody {
                t: ts.time,
                i: ts.increment,
            },
        }
    }
}

impl Timestamp {
    pub(crate) fn parse(self) -> crate::Timestamp {
        crate::Timestamp {
            time: self.body.t,
            increment: self.body.i,
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct DateTime {
    #[cfg_attr(feature = "serde", serde(rename = "$date"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$date"))]
    pub(crate) body: DateTimeBody,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize, Serialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(untagged))]
#[repr(C)]
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

impl From<crate::DateTime> for DateTime {
    fn from(dt: crate::DateTime) -> Self {
        Self {
            body: DateTimeBody::from_millis(dt.timestamp_millis()),
        }
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
                let datetime = crate::DateTime::parse_rfc3339_str(date.as_str())
                    .map_err(|_| parse_err!("invalid rfc3339 formatted utc datetime: {}", date))?;
                Ok(datetime)
            }
            DateTimeBody::Legacy(ms) => Ok(crate::DateTime::from_millis(ms)),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct MinKey {
    #[cfg_attr(feature = "serde", serde(rename = "$minKey"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$minKey"))]
    pub(crate) value: u8,
}

impl MinKey {
    pub(crate) fn parse(self) -> Result<Bson> {
        if self.value == 1 {
            Ok(Bson::MinKey)
        } else {
            Err(parse_err!(
                "value of $minKey should always be 1, got {}",
                self.value
            ))
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct MaxKey {
    #[cfg_attr(feature = "serde", serde(rename = "$maxKey"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$maxKey"))]
    pub(crate) value: u8,
}

impl MaxKey {
    pub(crate) fn parse(self) -> Result<Bson> {
        if self.value == 1 {
            Ok(Bson::MaxKey)
        } else {
            Err(parse_err!(
                "value of $maxKey should always be 1, got {}",
                self.value
            ))
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct DbPointer {
    #[cfg_attr(feature = "serde", serde(rename = "$dbPointer"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$dbPointer"))]
    body: DbPointerBody,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct DbPointerBody {
    #[cfg_attr(feature = "serde", serde(rename = "$ref"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$ref"))]
    pub(crate) ref_ns: String,

    #[cfg_attr(feature = "serde", serde(rename = "$id"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$id"))]
    pub(crate) id: ObjectId,
}

impl From<&crate::DbPointer> for DbPointer {
    fn from(dp: &crate::DbPointer) -> Self {
        Self {
            body: DbPointerBody {
                ref_ns: dp.namespace.clone(),
                id: ObjectId::from(dp.id),
            },
        }
    }
}

impl DbPointer {
    pub(crate) fn parse(self) -> Result<crate::DbPointer> {
        Ok(crate::DbPointer {
            namespace: self.body.ref_ns,
            id: self.body.id.parse()?,
        })
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(facet::Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct Undefined {
    #[cfg_attr(feature = "serde", serde(rename = "$undefined"))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "$undefined"))]
    pub(crate) value: bool,
}

impl Undefined {
    pub(crate) fn parse(self) -> Result<Bson> {
        if self.value {
            Ok(Bson::Undefined)
        } else {
            Err(parse_err!("$undefined should always be true, got false"))
        }
    }
}

#[cfg(feature = "serde")]
#[derive(Debug, Deserialize)]
pub(crate) struct BorrowedRegexBody<'a> {
    #[serde(borrow)]
    pub(crate) pattern: Cow<'a, str>,

    #[serde(borrow)]
    pub(crate) options: Cow<'a, str>,
}

#[cfg(feature = "serde")]
#[derive(Debug, Deserialize)]
pub(crate) struct BorrowedBinaryBody<'a> {
    #[serde(borrow)]
    pub(crate) bytes: Cow<'a, [u8]>,

    #[serde(rename = "subType")]
    pub(crate) subtype: u8,
}

#[cfg(feature = "serde")]
#[derive(Deserialize)]
pub(crate) struct BorrowedDbPointerBody<'a> {
    #[serde(rename = "$ref")]
    #[serde(borrow)]
    pub(crate) ns: CowStr<'a>,

    #[serde(rename = "$id")]
    pub(crate) id: oid::ObjectId,
}

macro_rules! parse_err {
    ($fmt:literal $(, $a:expr)*) => {{
        crate::error::Error::deserialization(format!($fmt $(, $a)*))
    }};
}
pub(crate) use parse_err;
