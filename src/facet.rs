//! Support for the `facet` crate.

use facet::Facet;
use facet_value::{value, Destructured, VNumber, Value};

use crate::{error::Error, Bson, Document, Regex};

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
            Bson::Undefined => todo!(),
            Bson::MaxKey => todo!(),
            Bson::MinKey => todo!(),
            Bson::DbPointer(db_pointer) => todo!(),
        }
    }
}

impl TryFrom<ExtJson> for Bson {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        Ok(match value.0.destructure() {
            Destructured::Null => Bson::Null,
            Destructured::Bool(b) => Bson::Boolean(b),
            Destructured::Number(vn) => {
                if vn.is_float() {
                    Bson::Double(vn.to_f64_lossy())
                } else if let Some(i) = vn.to_i32() {
                    Bson::Int32(i)
                } else if let Some(i) = vn.to_i64() {
                    Bson::Int64(i)
                } else {
                    return Err(Error::deserialization(format!(
                        "expected double, int32, or in34, got {vn:?}"
                    )));
                }
            }
            Destructured::String(vs) => Bson::String(vs.as_str().to_owned()),
            Destructured::Bytes(vbytes) => todo!(),
            Destructured::Array(varray) => todo!(),
            Destructured::Object(vobject) => todo!(),
            Destructured::DateTime(vdate_time) => todo!(),
            Destructured::QName(vqname) => todo!(),
            Destructured::Uuid(vuuid) => todo!(),
        })
    }
}

macro_rules! try_from_num {
    ($($t:ty),+) => {
        $(
            impl TryFrom<&$t> for ExtJson {
                type Error = std::convert::Infallible;

                fn try_from(t: &$t) -> Result<Self, Self::Error> {
                    Ok(ExtJson(Value::from(*t)))
                }
            }
        )+
    };
}

try_from_num!(bool);

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
        match xj.0.destructure() {
            Destructured::Number(vn) => Ok(vn.to_f64_lossy()),
            Destructured::Object(vo) => {
                let s = vo
                    .get("$numberDouble")
                    .and_then(|v| v.as_string())
                    .ok_or_else(|| Error::deserialization("expected string for $numberDouble"))?;
                Ok(match s.as_str() {
                    "NaN" => std::f64::NAN,
                    "Infinity" => std::f64::INFINITY,
                    "-Infinity" => std::f64::NEG_INFINITY,
                    s => s
                        .parse()
                        .map_err(|e| Error::deserialization(format!("could not parse f64: {e}")))?,
                })
            }
            other => Err(Error::deserialization(format!(
                "expected number or object, got {other:?}"
            ))),
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

/*
impl TryFrom<ExtJson> for f64 {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        value
            .0
            .as_number()
            .map(VNumber::to_f64_lossy)
            .ok_or_else(|| {
                crate::error::Error::deserialization(format!("expected number, got {value:?}"))
            })
    }
}
*/

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

impl TryFrom<&crate::oid::ObjectId> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(oid: &crate::oid::ObjectId) -> Result<Self, Self::Error> {
        Ok(ExtJson(value!({"$oid": (oid.to_string())})))
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
