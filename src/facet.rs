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
            Destructured::Bytes(vbytes) => {
                Bson::Binary(crate::Binary::try_from(ExtJson(vbytes.into()))?)
            }
            Destructured::Array(varray) => Bson::Array(
                varray
                    .into_iter()
                    .map(|e| Bson::try_from(ExtJson(e)))
                    .collect::<Result<Vec<_>, Self::Error>>()?,
            ),
            Destructured::Object(vobject) => {
                let mut keys: Vec<_> = vobject.keys().map(|vs| vs.as_str()).collect();
                keys.sort_unstable();
                match keys.as_slice() {
                    ["$oid"] => todo!(),
                    _ => (),
                }

                todo!()
            }
            Destructured::DateTime(vdt) => {
                if vdt.offset_minutes().unwrap_or(0) != 0 {
                    return Err(Error::deserialization(format!(
                        "cannot deserialize from non-UTC datetime {vdt:?}"
                    )));
                }
                if !vdt.has_date() {
                    return Err(Error::deserialization(format!(
                        "cannot deserialize from time without date {vdt:?}"
                    )));
                }
                Bson::DateTime(
                    crate::DateTime::builder()
                        .year(vdt.year())
                        .month(vdt.month())
                        .day(vdt.day())
                        .hour(vdt.hour())
                        .minute(vdt.minute())
                        .second(vdt.second())
                        .millisecond((vdt.nanos() / 1_000_000) as u16)
                        .build()?,
                )
            }
            Destructured::QName(vqname) => {
                if vqname.has_namespace() {
                    return Err(Error::deserialization(format!(
                        "cannot deserialize from qualified name with namespace {vqname:?}"
                    )));
                }
                Bson::try_from(ExtJson(vqname.local_name().clone()))?
            }
            Destructured::Uuid(vuuid) => Bson::Binary(Binary::from_uuid(crate::Uuid::from_bytes(
                *vuuid.as_bytes(),
            ))),
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
        Ok(match value.0.destructure() {
            Destructured::Bytes(vb) => Binary {
                subtype: BinarySubtype::Generic,
                bytes: vb.as_slice().to_vec(),
            },
            Destructured::Uuid(vu) => Binary {
                subtype: BinarySubtype::Uuid,
                bytes: vu.as_bytes().into(),
            },
            Destructured::Object(vo) => match ObjMatch::new(vo).take_pairs("Binary")? {
                [("$binary", inner)] => todo!(),
                _ => todo!(),
            },
            _ => todo!(),
        })
    }
}

struct ObjMatch {
    keys: Vec<VString>,
    values: Vec<Value>,
}

impl ObjMatch {
    fn new(obj: VObject) -> Self {
        let mut keys = vec![];
        let mut values = vec![];
        for (k, v) in obj {
            keys.push(k);
            values.push(v);
        }
        Self { keys, values }
    }

    fn take_pairs<const N: usize>(
        &mut self,
        name: &str,
    ) -> crate::error::Result<[(&str, Value); N]> {
        let v: Vec<_> = self
            .keys
            .iter()
            .map(|vs| vs.as_str())
            .zip(self.values.drain(..))
            .collect();
        v.try_into().map_err(|v| {
            Error::deserialization(format!("expected {name} as {N} entries, got {v:?}"))
        })
    }
}

impl TryFrom<&crate::oid::ObjectId> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(oid: &crate::oid::ObjectId) -> Result<Self, Self::Error> {
        Ok(ExtJson(value!({"$oid": (oid.to_string())})))
    }
}

impl TryFrom<ExtJson> for crate::oid::ObjectId {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0.destructure() {
            Destructured::Uuid(vu) => todo!(),
            other => {
                return Err(Error::deserialization(format!(
                    "invalid ObjectId: {other:?}"
                )))
            }
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
