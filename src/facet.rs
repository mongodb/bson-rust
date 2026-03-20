//! Support for the `facet` crate.

use facet::Facet;
use facet_value::{value, Destructured, VObject, VString, Value};

use crate::{error::Error, spec::BinarySubtype, Binary, Bson, Document, Regex};

/// A type for use with #[facet(proxy)] that represents BSON values in their canonical extended JSON
/// form.
#[derive(Facet, Debug)]
#[facet(transparent)]
pub struct ExtJson(Value);

impl TryFrom<&Bson> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &Bson) -> Result<Self, Self::Error> {
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

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        use Destructured::*;
        Ok(match value.0.destructure() {
            Null => Bson::Null,
            Bool(b) => Bson::Boolean(b),
            String(vs) => Bson::String(vs.as_str().to_owned()),
            Array(varray) => Bson::Array(
                varray
                    .into_iter()
                    .map(|e| Bson::try_from(ExtJson(e)))
                    .collect::<Result<Vec<_>, Self::Error>>()?,
            ),
            Object(vobject) => {
                let mut keys: Vec<_> = vobject.keys().map(|vs| vs.as_str()).collect();
                keys.sort_unstable();
                match keys.as_slice() {
                    ["$oid"] => {
                        return crate::oid::ObjectId::try_from(ExtJson(vobject.into()))
                            .map(Bson::ObjectId)
                    }
                    _ => (),
                }

                let mut out = Document::new();
                for (k, v) in vobject {
                    let v: Bson = ExtJson(v).try_into()?;
                    out.insert(k, v);
                }
                Bson::Document(out)
            }
            other => return Err(parse_err!("unexpected value type {other:?}")),
        })
    }
}

impl TryFrom<&f64> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(f: &f64) -> Result<Self, Self::Error> {
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

    fn try_from(xj: ExtJson) -> Result<Self, Self::Error> {
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

    fn try_from(f: &f32) -> Result<Self, Self::Error> {
        (&(*f as f64)).try_into()
    }
}

impl TryFrom<&i64> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(i: &i64) -> Result<Self, Self::Error> {
        Ok(ExtJson(value!({"$numberLong": (i.to_string())})))
    }
}

impl TryFrom<&i32> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(i: &i32) -> Result<Self, Self::Error> {
        Ok(ExtJson(value!({"$numberInt": (i.to_string())})))
    }
}

impl TryFrom<&u32> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(i: &u32) -> Result<Self, Self::Error> {
        (&(*i as i64)).try_into()
    }
}

impl TryFrom<&str> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Ok(ExtJson(Value::from(s)))
    }
}

impl TryFrom<&String> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(s: &String) -> Result<Self, Self::Error> {
        Ok(ExtJson(Value::from(s)))
    }
}

impl TryFrom<&crate::Array> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(bsons: &crate::Array) -> Result<Self, Self::Error> {
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

    fn try_from(doc: &Document) -> Result<Self, Self::Error> {
        Ok(ExtJson(
            doc.iter()
                .map(|(k, v)| (k, ExtJson::try_from(v).unwrap().0))
                .collect(),
        ))
    }
}

impl TryFrom<&bool> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(b: &bool) -> Result<Self, Self::Error> {
        Ok(ExtJson(Value::from(*b)))
    }
}

impl TryFrom<&Regex> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(regex: &Regex) -> Result<Self, Self::Error> {
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

    fn try_from(jsc: &crate::JavaScriptCodeWithScope) -> Result<Self, Self::Error> {
        let scope: ExtJson = (&jsc.scope).try_into()?;
        Ok(ExtJson(value!({
            "$code": (&jsc.code),
            "$scope": (scope.0),
        })))
    }
}

impl TryFrom<&crate::Timestamp> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(ts: &crate::Timestamp) -> Result<Self, Self::Error> {
        Ok(ExtJson(value!({
            "$timestamp": {
                "t": (ts.time),
                "i": (ts.increment),
            }
        })))
    }
}

impl TryFrom<&crate::Binary> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(bin: &crate::Binary) -> Result<Self, Self::Error> {
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

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
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

    fn try_from(oid: &crate::oid::ObjectId) -> Result<Self, Self::Error> {
        Ok(ExtJson(value!({"$oid": (oid.to_hex())})))
    }
}

impl TryFrom<ExtJson> for crate::oid::ObjectId {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
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

    fn try_from(dt: &crate::DateTime) -> Result<Self, Self::Error> {
        Ok(ExtJson(value!({
            "$date": {
                "$numberLong": (dt.timestamp_millis()),
            }
        })))
    }
}

impl TryFrom<&crate::Decimal128> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(d: &crate::Decimal128) -> Result<Self, Self::Error> {
        Ok(ExtJson(value!({"$numberDecimal": (d.to_string())})))
    }
}

impl TryFrom<&crate::DbPointer> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(d: &crate::DbPointer) -> Result<Self, Self::Error> {
        let id: ExtJson = (&d.id).try_into()?;
        Ok(ExtJson(value!({
            "$ref": (&d.namespace),
            "$id": (id.0),
        })))
    }
}

fn values<const N: usize>(
    mut obj: VObject,
    keys: [&str; N],
) -> crate::error::Result<[Destructured; N]> {
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
