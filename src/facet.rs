//! Support for the `facet` crate.

use facet::Facet;
use facet_value::{value, VNumber, Value};

use crate::{error::Error, Bson, Regex};

/// A type for use with #[facet(proxy)] that represents BSON values in their extended JSON form.
#[derive(Facet, Debug)]
#[facet(transparent)]
pub struct ExtJson(Value);

impl TryFrom<&Bson> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &Bson) -> Result<Self, Self::Error> {
        Ok(ExtJson(match value {
            Bson::Double(d) => return d.try_into(),
            Bson::String(s) => Value::from(s),
            Bson::Array(bsons) => return bsons.try_into(),
            Bson::Document(document) => todo!(),
            Bson::Boolean(b) => Value::from(*b),
            Bson::Null => Value::NULL,
            Bson::RegularExpression(regex) => return regex.try_into(),
            Bson::JavaScriptCode(_) => todo!(),
            Bson::JavaScriptCodeWithScope(java_script_code_with_scope) => todo!(),
            Bson::Int32(_) => todo!(),
            Bson::Int64(_) => todo!(),
            Bson::Timestamp(timestamp) => todo!(),
            Bson::Binary(binary) => todo!(),
            Bson::ObjectId(object_id) => todo!(),
            Bson::DateTime(date_time) => todo!(),
            Bson::Symbol(_) => todo!(),
            Bson::Decimal128(decimal128) => todo!(),
            Bson::Undefined => todo!(),
            Bson::MaxKey => todo!(),
            Bson::MinKey => todo!(),
            Bson::DbPointer(db_pointer) => todo!(),
        }))
    }
}

impl TryFrom<ExtJson> for Bson {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        Ok(match value.0.destructure() {
            facet_value::Destructured::Null => Bson::Null,
            facet_value::Destructured::Bool(b) => Bson::Boolean(b),
            facet_value::Destructured::Number(vn) => {
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
            facet_value::Destructured::String(vs) => Bson::String(vs.as_str().to_owned()),
            facet_value::Destructured::Bytes(vbytes) => todo!(),
            facet_value::Destructured::Array(varray) => todo!(),
            facet_value::Destructured::Object(vobject) => todo!(),
            facet_value::Destructured::DateTime(vdate_time) => todo!(),
            facet_value::Destructured::QName(vqname) => todo!(),
            facet_value::Destructured::Uuid(vuuid) => todo!(),
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

try_from_num!(f64, f32, i64, i32, u32, bool);

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
                .collect::<Value>(),
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
