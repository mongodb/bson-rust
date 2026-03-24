//! Support for the `facet` crate.

use std::str::FromStr;

use facet::Facet;
use facet_value::{value, Destructured, VObject, Value};

use crate::{
    error::{Error, Result},
    Bson,
    Document,
    Regex,
};

/// A type for use with #[facet(proxy)] that represents BSON values in their canonical extended JSON
/// form.
#[derive(Facet, Debug)]
#[facet(transparent)]
pub struct ExtJson(Value);

impl TryFrom<&Bson> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &Bson) -> std::result::Result<Self, Self::Error> {
        match value {
            Bson::Double(d) => d.try_into(),
            Bson::String(s) => s.try_into(),
            Bson::Array(bsons) => bsons.try_into(),
            Bson::Document(doc) => doc.try_into(),
            Bson::Boolean(b) => b.try_into(),
            Bson::Null => Ok(ExtJson(Value::NULL)),
            Bson::RegularExpression(regex) => regex.try_into(),
            Bson::JavaScriptCode(s) => Ok(ExtJson(value!({"$code": (s)}))),
            Bson::JavaScriptCodeWithScope(jsc) => jsc.try_into(),
            Bson::Int32(i) => i.try_into(),
            Bson::Int64(i) => i.try_into(),
            Bson::Timestamp(ts) => ts.try_into(),
            Bson::Binary(bin) => bin.try_into(),
            Bson::ObjectId(oid) => oid.try_into(),
            Bson::DateTime(dt) => dt.try_into(),
            Bson::Symbol(s) => Ok(ExtJson(value!({"$symbol": (s)}))),
            Bson::Decimal128(d) => d.try_into(),
            Bson::Undefined => Ok(ExtJson(value!({"$undefined": true}))),
            Bson::MaxKey => Ok(ExtJson(value!({"$maxKey": 1}))),
            Bson::MinKey => Ok(ExtJson(value!({"$minKey": 1}))),
            Bson::DbPointer(d) => d.try_into(),
        }
    }
}

impl TryFrom<ExtJson> for Bson {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self> {
        use Destructured::*;
        Ok(match value.0.destructure() {
            Null => Bson::Null,
            Bool(b) => Bson::Boolean(b),
            String(vs) => Bson::String(vs.as_str().to_owned()),
            Array(varray) => Bson::Array(
                varray
                    .into_iter()
                    .map(|e| Bson::try_from(ExtJson(e)))
                    .collect::<Result<Vec<_>>>()?,
            ),
            Object(obj) => {
                let mut keys: Vec<_> = obj.keys().map(|vs| vs.as_str()).collect();
                keys.sort_unstable();
                match keys.as_slice() {
                    ["$oid"] => {
                        Bson::ObjectId(crate::oid::ObjectId::try_from(ExtJson(obj.into()))?)
                    }
                    ["$symbol"] => match values(obj, ["$symbol"])? {
                        [String(s)] => Bson::Symbol(s.into()),
                        [other] => {
                            return Err(parse_err!("$symbol: expected string, got {other:?}"))
                        }
                    },
                    ["$numberInt"] => Bson::Int32(i32::try_from(ExtJson(obj.into()))?),
                    ["$numberLong"] => Bson::Int64(i64::try_from(ExtJson(obj.into()))?),
                    ["$numberDouble"] => Bson::Double(f64::try_from(ExtJson(obj.into()))?),
                    ["$numberDecimal"] => {
                        Bson::Decimal128(crate::Decimal128::try_from(ExtJson(obj.into()))?)
                    }
                    ["$binary"] => Bson::Binary(crate::Binary::try_from(ExtJson(obj.into()))?),
                    ["$code"] => match values(obj, ["$code"])? {
                        [String(s)] => Bson::JavaScriptCode(s.into()),
                        [other] => return Err(parse_err!("$code: expected string, got {other:?}")),
                    },
                    ["$code", "$scope"] => Bson::JavaScriptCodeWithScope(
                        crate::JavaScriptCodeWithScope::try_from(ExtJson(obj.into()))?,
                    ),
                    ["$timestamp"] => {
                        Bson::Timestamp(crate::Timestamp::try_from(ExtJson(obj.into()))?)
                    }
                    _ => Bson::Document(Document::try_from(ExtJson(obj.into()))?),
                }
            }
            other => return Err(parse_err!("unexpected value type {other:?}")),
        })
    }
}

impl TryFrom<&f64> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(f: &f64) -> std::result::Result<Self, Self::Error> {
        let s = if f.is_nan() {
            "NaN"
        } else if f.is_infinite() {
            if f.is_sign_positive() {
                "Infinity"
            } else {
                "-Infinity"
            }
        } else {
            &f.to_string()
        };
        Ok(ExtJson(value!({"$numberDouble": (s)})))
    }
}

impl TryFrom<ExtJson> for f64 {
    type Error = crate::error::Error;

    fn try_from(xj: ExtJson) -> Result<Self> {
        use Destructured::*;
        match xj.0.destructure() {
            Object(vo) => match values(vo, ["$numberDouble"])? {
                [String(s)] => Ok(match s.as_str() {
                    "NaN" => std::f64::NAN,
                    "Infinity" => std::f64::INFINITY,
                    "-Infinity" => std::f64::NEG_INFINITY,
                    s => s.parse().map_err(|e| parse_err!("{e}"))?,
                }),
                [other] => Err(parse_err!("$numberDouble: expected string, got {other:?}")),
            },
            other => Err(parse_err!("f64: expected object, got {other:?}")),
        }
    }
}

impl TryFrom<&f32> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(f: &f32) -> std::result::Result<Self, Self::Error> {
        (&(*f as f64)).try_into()
    }
}

impl TryFrom<ExtJson> for f32 {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self> {
        // f64 -> f32 never fails because it always incurs a loss of precision
        Ok(f64::try_from(value)? as f32)
    }
}

impl TryFrom<&i64> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(i: &i64) -> std::result::Result<Self, Self::Error> {
        Ok(ExtJson(value!({"$numberLong": (i.to_string())})))
    }
}

impl TryFrom<ExtJson> for i64 {
    type Error = crate::error::Error;

    fn try_from(xj: ExtJson) -> Result<Self> {
        use Destructured::*;
        match xj.0.destructure() {
            Object(obj) => match values(obj, ["$numberLong"])? {
                [String(s)] => s.parse().map_err(|e| parse_err!("{e}")),
                [other] => Err(parse_err!("$numberLong: expected string, got {other:?}")),
            },
            other => Err(parse_err!("i64: expected object, got {other:?}")),
        }
    }
}

impl TryFrom<&i32> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(i: &i32) -> std::result::Result<Self, Self::Error> {
        Ok(ExtJson(value!({"$numberInt": (i.to_string())})))
    }
}

impl TryFrom<ExtJson> for i32 {
    type Error = crate::error::Error;

    fn try_from(xj: ExtJson) -> Result<Self> {
        use Destructured::*;
        match xj.0.destructure() {
            Object(obj) => match values(obj, ["$numberInt"])? {
                [String(s)] => s.parse().map_err(|e| parse_err!("{e}")),
                [other] => Err(parse_err!("$numberInt: expected string, got {other:?}")),
            },
            other => Err(parse_err!("i32: expected object, got {other:?}")),
        }
    }
}

impl TryFrom<&u32> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(i: &u32) -> std::result::Result<Self, Self::Error> {
        (&(*i as i64)).try_into()
    }
}

impl TryFrom<ExtJson> for u32 {
    type Error = crate::error::Error;

    fn try_from(xj: ExtJson) -> Result<Self> {
        u32::try_from(i64::try_from(xj)?).map_err(|e| parse_err!("{e}"))
    }
}

impl TryFrom<&str> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(s: &str) -> std::result::Result<Self, Self::Error> {
        Ok(ExtJson(Value::from(s)))
    }
}

impl TryFrom<&String> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(s: &String) -> std::result::Result<Self, Self::Error> {
        Ok(ExtJson(Value::from(s)))
    }
}

impl TryFrom<&crate::Array> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(bsons: &crate::Array) -> std::result::Result<Self, Self::Error> {
        Ok(ExtJson(
            bsons
                .iter()
                .map(|v| ExtJson::try_from(v).unwrap().0)
                .collect(),
        ))
    }
}

impl TryFrom<&Document> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(doc: &Document) -> std::result::Result<Self, Self::Error> {
        Ok(ExtJson(
            doc.iter()
                .map(|(k, v)| (k, ExtJson::try_from(v).unwrap().0))
                .collect(),
        ))
    }
}

impl TryFrom<ExtJson> for Document {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self> {
        match value.0.destructure() {
            Destructured::Object(obj) => {
                let mut out = Document::new();
                for (k, v) in obj {
                    let v: Bson = ExtJson(v).try_into()?;
                    out.insert(k, v);
                }
                Ok(out)
            }
            other => Err(parse_err!("document: expected object, got {other:?}")),
        }
    }
}

impl TryFrom<&bool> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(b: &bool) -> std::result::Result<Self, Self::Error> {
        Ok(ExtJson(Value::from(*b)))
    }
}

impl TryFrom<&Regex> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(regex: &Regex) -> std::result::Result<Self, Self::Error> {
        Ok(ExtJson(value!({
            "$regularExpression": {
                "pattern": (regex.pattern.as_str()),
                "options": (regex.options.as_str()),
            }
        })))
    }
}

impl TryFrom<&crate::JavaScriptCodeWithScope> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(jsc: &crate::JavaScriptCodeWithScope) -> std::result::Result<Self, Self::Error> {
        let scope: ExtJson = (&jsc.scope).try_into()?;
        Ok(ExtJson(value!({
            "$code": (&jsc.code),
            "$scope": (scope.0),
        })))
    }
}

impl TryFrom<ExtJson> for crate::JavaScriptCodeWithScope {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self> {
        use Destructured::*;
        match value.0.destructure() {
            Object(obj) => match values(obj, ["$code", "$scope"])? {
                [String(code), Object(scope)] => Ok(crate::JavaScriptCodeWithScope {
                    code: code.into(),
                    scope: ExtJson(scope.into()).try_into()?,
                }),
                other => Err(parse_err!("code with scope: invalid body {other:?}")),
            },
            other => Err(parse_err!(
                "code with scope: expected object, got {other:?}"
            )),
        }
    }
}

impl TryFrom<&crate::Timestamp> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(ts: &crate::Timestamp) -> std::result::Result<Self, Self::Error> {
        Ok(ExtJson(value!({
            "$timestamp": {
                "t": (ts.time),
                "i": (ts.increment),
            }
        })))
    }
}

impl TryFrom<ExtJson> for crate::Timestamp {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self> {
        use Destructured::*;
        match value.0.destructure() {
            Object(obj) => match values(obj, ["$timestamp"])? {
                [Object(body)] => match values(body, ["t", "i"])? {
                    [Number(t), Number(i)] if t.is_integer() && i.is_integer() => {
                        let time = t
                            .to_u32()
                            .ok_or_else(|| parse_err!("timestamp.t: expected u32, got {t:?}"))?;
                        let increment = i
                            .to_u32()
                            .ok_or_else(|| parse_err!("timestamp.i: expected u32, got {i:?}"))?;
                        Ok(crate::Timestamp { time, increment })
                    }
                    other => Err(parse_err!("$timestamp: invalid body {other:?}")),
                },
                other => Err(parse_err!("$timestamp: expected object, got {other:?}")),
            },
            other => Err(parse_err!("timestamp: expected object, got {other:?}")),
        }
    }
}

impl TryFrom<&crate::Binary> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(bin: &crate::Binary) -> std::result::Result<Self, Self::Error> {
        Ok(ExtJson(value!({
            "$binary": {
                "base64": (crate::base64::encode(&bin.bytes)),
                "subType": (hex::encode([u8::from(bin.subtype)])),
            }
        })))
    }
}

impl TryFrom<ExtJson> for crate::Binary {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self> {
        use Destructured::*;
        Ok(match value.0.destructure() {
            Object(vo) => match values(vo, ["$binary"])? {
                [Object(body)] => match values(body, ["base64", "subType"])? {
                    [String(bytes), String(subtype)] => {
                        let bytes = crate::base64::decode(bytes).map_err(|e| parse_err!("{e}"))?;
                        let subtype = match hex::decode(subtype)
                            .map_err(|e| parse_err!("{e}"))?
                            .as_slice()
                        {
                            [b] => (*b).into(),
                            other => return Err(parse_err!("invalid binary subtype {other:?}")),
                        };
                        crate::Binary { bytes, subtype }
                    }
                    other => return Err(parse_err!("invalid $binary values: {other:?}")),
                },
                [other] => return Err(parse_err!("expected `$binary` object, got {other:?}")),
            },
            other => return Err(parse_err!("binary: expected object, got {other:?}")),
        })
    }
}

impl TryFrom<&crate::oid::ObjectId> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(oid: &crate::oid::ObjectId) -> std::result::Result<Self, Self::Error> {
        Ok(ExtJson(value!({"$oid": (oid.to_hex())})))
    }
}

impl TryFrom<ExtJson> for crate::oid::ObjectId {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self> {
        use Destructured::*;
        match value.0.destructure() {
            Object(obj) => match values(obj, ["$oid"])? {
                [String(bytes)] => crate::oid::ObjectId::parse_str(bytes),
                other => Err(parse_err!("$oid: expected string, got {other:?}")),
            },
            other => Err(parse_err!("objectid: expected object, got {other:?}")),
        }
    }
}

impl TryFrom<&crate::DateTime> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(dt: &crate::DateTime) -> std::result::Result<Self, Self::Error> {
        Ok(ExtJson(value!({
            "$date": {
                "$numberLong": (dt.timestamp_millis()),
            }
        })))
    }
}

impl TryFrom<&crate::Decimal128> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(d: &crate::Decimal128) -> std::result::Result<Self, Self::Error> {
        Ok(ExtJson(value!({"$numberDecimal": (d.to_string())})))
    }
}

impl TryFrom<ExtJson> for crate::Decimal128 {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self> {
        use Destructured::*;
        match value.0.destructure() {
            Object(obj) => match values(obj, ["$numberDecimal"])? {
                [String(d)] => crate::Decimal128::from_str(d.as_str()),
                [other] => Err(parse_err!("$numberDecimal: expected string, got {other:?}")),
            },
            other => Err(parse_err!("decimal128: expected object, got {other:?}")),
        }
    }
}

impl TryFrom<&crate::DbPointer> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(d: &crate::DbPointer) -> std::result::Result<Self, Self::Error> {
        let id: ExtJson = (&d.id).try_into()?;
        Ok(ExtJson(value!({
            "$ref": (&d.namespace),
            "$id": (id.0),
        })))
    }
}

fn values<const N: usize>(mut obj: VObject, keys: [&str; N]) -> Result<[Destructured; N]> {
    if obj.len() != N {
        return Err(parse_err!(
            "wrong object len: expected keys {keys:?}, got {obj:?}"
        ));
    }
    let mut out = Vec::with_capacity(N);
    for key in keys {
        out.push(
            obj.remove(key)
                .ok_or_else(|| parse_err!("expected key {key} not found"))?
                .destructure(),
        );
    }
    out.try_into().map_err(|v| {
        parse_err!("internal: invalid value array length, expected {N} values got {v:?}")
    })
}

macro_rules! parse_err {
    ($fmt:literal $(, $a:expr)*) => {{
        Error::deserialization(format!($fmt $(, $a)*))
    }};
}
use parse_err;
