use std::convert::{TryFrom, TryInto};

use serde::de::{Error as _, Unexpected};
use serde_json::{Value, json};

use crate::{
    Binary,
    Bson,
    DbPointer,
    Document,
    JavaScriptCodeWithScope,
    Regex,
    Timestamp,
    error::{Error, Result},
    extjson::models,
};

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::deserialization(error)
    }
}

/// Converts the [`serde_json::Map`] into [`Bson`]. This conversion can interpret both canonical
/// and relaxed [extended JSON](https://www.mongodb.com/docs/manual/reference/mongodb-extended-json/).
impl TryFrom<serde_json::Map<String, serde_json::Value>> for Bson {
    type Error = Error;

    fn try_from(obj: serde_json::Map<String, serde_json::Value>) -> Result<Self> {
        let keys: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();

        use models::ObjectType;
        match ObjectType::from_keys(&keys) {
            ObjectType::ObjectId => {
                let oid: models::ObjectId = serde_json::from_value(obj.into())?;
                Ok(Bson::ObjectId(oid.parse()?))
            }
            ObjectType::Symbol => {
                let symbol: models::Symbol = serde_json::from_value(obj.into())?;
                Ok(Bson::Symbol(symbol.value))
            }
            ObjectType::RegularExpression => {
                let regex: models::Regex = serde_json::from_value(obj.into())?;
                Ok(regex.parse()?.into())
            }
            ObjectType::Int32 => {
                let int: models::Int32 = serde_json::from_value(obj.into())?;
                Ok(Bson::Int32(int.parse()?))
            }
            ObjectType::Int64 => {
                let int: models::Int64 = serde_json::from_value(obj.into())?;
                Ok(Bson::Int64(int.parse()?))
            }
            ObjectType::Double => {
                let double: models::Double = serde_json::from_value(obj.into())?;
                Ok(Bson::Double(double.parse()?))
            }
            ObjectType::Decimal128 => {
                let decimal: models::Decimal128 = serde_json::from_value(obj.into())?;
                Ok(Bson::Decimal128(decimal.parse()?))
            }
            ObjectType::Binary => {
                let binary: models::Binary = serde_json::from_value(obj.into())?;
                Ok(Bson::Binary(binary.parse()?))
            }
            ObjectType::Uuid => {
                let uuid: models::Uuid = serde_json::from_value(obj.into())?;
                Ok(Bson::Binary(uuid.parse()?))
            }
            ObjectType::JavaScriptCode => {
                let code: models::JavaScriptCode = serde_json::from_value(obj.into())?;
                Ok(Bson::JavaScriptCode(code.code))
            }
            ObjectType::JavaScriptCodeWithScope => {
                let code: models::JavaScriptCodeWithScope<
                    serde_json::Map<String, serde_json::Value>,
                > = serde_json::from_value(obj.into())?;
                Ok(crate::JavaScriptCodeWithScope {
                    code: code.code,
                    scope: code.scope.try_into()?,
                }
                .into())
            }
            ObjectType::Timestamp => {
                let ts: models::Timestamp = serde_json::from_value(obj.into())?;
                Ok(ts.parse()?.into())
            }
            ObjectType::DateTime => {
                let extjson_datetime: models::DateTime = serde_json::from_value(obj.into())?;
                Ok(Bson::DateTime(extjson_datetime.parse()?))
            }
            ObjectType::MinKey => {
                let min_key: models::MinKey = serde_json::from_value(obj.into())?;
                min_key.parse()
            }
            ObjectType::MaxKey => {
                let max_key: models::MaxKey = serde_json::from_value(obj.into())?;
                max_key.parse()
            }
            ObjectType::DbPointer => {
                let db_ptr: models::DbPointer = serde_json::from_value(obj.into())?;
                Ok(db_ptr.parse()?.into())
            }
            ObjectType::Undefined => {
                let undefined: models::Undefined = serde_json::from_value(obj.into())?;
                undefined.parse()
            }
            ObjectType::Document => Ok(Bson::Document(obj.try_into()?)),
        }
    }
}

/// Converts the [`serde_json::Value`] into [`Bson`]. This conversion can interpret both canonical
/// and relaxed [extended JSON](https://www.mongodb.com/docs/manual/reference/mongodb-extended-json/).
impl TryFrom<serde_json::Value> for Bson {
    type Error = Error;

    fn try_from(value: serde_json::Value) -> Result<Self> {
        match value {
            serde_json::Value::Number(x) => x
                .as_i64()
                .map(|i| {
                    if i >= i32::MIN as i64 && i <= i32::MAX as i64 {
                        Bson::Int32(i as i32)
                    } else {
                        Bson::Int64(i)
                    }
                })
                .or_else(|| x.as_f64().map(Bson::from))
                .ok_or_else(|| {
                    Error::invalid_value(
                        Unexpected::Other(format!("{}", x).as_str()),
                        &"a number that could fit in i32, i64, or f64",
                    )
                }),
            serde_json::Value::String(x) => Ok(x.into()),
            serde_json::Value::Bool(x) => Ok(x.into()),
            serde_json::Value::Array(x) => Ok(Bson::Array(
                x.into_iter()
                    .map(Bson::try_from)
                    .collect::<Result<Vec<Bson>>>()?,
            )),
            serde_json::Value::Null => Ok(Bson::Null),
            serde_json::Value::Object(map) => map.try_into(),
        }
    }
}

/// Converts the [`serde_json::Map`] into a [`Document`]. This conversion can interpret both
/// canonical and relaxed [extended JSON](https://www.mongodb.com/docs/manual/reference/mongodb-extended-json/).
impl TryFrom<serde_json::Map<String, serde_json::Value>> for Document {
    type Error = Error;

    fn try_from(obj: serde_json::Map<String, serde_json::Value>) -> Result<Self> {
        Ok(obj
            .into_iter()
            .map(|(k, v)| -> Result<(String, Bson)> {
                let value: Bson = v.try_into()?;
                Ok((k, value))
            })
            .collect::<Result<Vec<(String, Bson)>>>()?
            .into_iter()
            .collect())
    }
}

/// Converts [`Bson`] into a [`serde_json::Value`] in relaxed
/// [extended JSON](https://www.mongodb.com/docs/manual/reference/mongodb-extended-json/).
impl From<Bson> for Value {
    fn from(bson: Bson) -> Self {
        bson.into_relaxed_extjson()
    }
}

impl Bson {
    /// Converts this value into a [`serde_json::Value`] in relaxed
    /// [extended JSON](https://www.mongodb.com/docs/manual/reference/mongodb-extended-json/)
    /// format.
    pub fn into_relaxed_extjson(self) -> Value {
        match self {
            Bson::Double(v) if v.is_nan() => {
                let s = if v.is_sign_negative() { "-NaN" } else { "NaN" };

                json!({ "$numberDouble": s })
            }
            Bson::Double(v) if v.is_infinite() => {
                let s = if v.is_sign_negative() {
                    "-Infinity"
                } else {
                    "Infinity"
                };

                json!({ "$numberDouble": s })
            }
            Bson::Double(v) => json!(v),
            Bson::String(v) => json!(v),
            Bson::Array(v) => Value::Array(v.into_iter().map(Bson::into_relaxed_extjson).collect()),
            Bson::Document(v) => Value::Object(
                v.into_iter()
                    .map(|(k, v)| (k, v.into_relaxed_extjson()))
                    .collect(),
            ),
            Bson::Boolean(v) => json!(v),
            Bson::Null => Value::Null,
            Bson::RegularExpression(Regex { pattern, options }) => {
                let mut chars: Vec<_> = options.as_str().chars().collect();
                chars.sort_unstable();

                let options: String = chars.into_iter().collect();

                json!({
                    "$regularExpression": {
                        "pattern": pattern.into_string(),
                        "options": options,
                    }
                })
            }
            Bson::JavaScriptCode(code) => json!({ "$code": code }),
            Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope { code, scope }) => json!({
                "$code": code,
                "$scope": Bson::Document(scope).into_relaxed_extjson(),
            }),
            Bson::Int32(v) => v.into(),
            Bson::Int64(v) => v.into(),
            Bson::Timestamp(Timestamp { time, increment }) => json!({
                "$timestamp": {
                    "t": time,
                    "i": increment,
                }
            }),
            Bson::Binary(Binary { subtype, ref bytes }) => {
                let tval: u8 = From::from(subtype);
                json!({
                    "$binary": {
                        "base64": crate::base64::encode(bytes),
                        "subType": hex::encode([tval]),
                    }
                })
            }
            Bson::ObjectId(v) => json!({"$oid": v.to_hex()}),
            Bson::DateTime(v) if v.timestamp_millis() >= 0 && v.to_time_0_3().year() <= 9999 => {
                json!({
                    // Unwrap safety: timestamps in the guarded range can always be formatted.
                    "$date": v.try_to_rfc3339_string().unwrap(),
                })
            }
            Bson::DateTime(v) => json!({
                "$date": { "$numberLong": v.timestamp_millis().to_string() },
            }),
            Bson::Symbol(v) => json!({ "$symbol": v }),
            Bson::Decimal128(v) => json!({ "$numberDecimal": v.to_string() }),
            Bson::Undefined => json!({ "$undefined": true }),
            Bson::MinKey => json!({ "$minKey": 1 }),
            Bson::MaxKey => json!({ "$maxKey": 1 }),
            Bson::DbPointer(DbPointer {
                ref namespace,
                ref id,
            }) => json!({
                "$dbPointer": {
                    "$ref": namespace,
                    "$id": {
                        "$oid": id.to_hex()
                    }
                }
            }),
        }
    }

    /// Converts this value into a [`serde_json::Value`] in canonical
    /// [extended JSON](https://www.mongodb.com/docs/manual/reference/mongodb-extended-json/)
    /// format.
    pub fn into_canonical_extjson(self) -> Value {
        match self {
            Bson::Int32(i) => json!({ "$numberInt": i.to_string() }),
            Bson::Int64(i) => json!({ "$numberLong": i.to_string() }),
            Bson::Double(f) if f.is_normal() => {
                let mut s = f.to_string();
                if f.fract() == 0.0 {
                    s.push_str(".0");
                }

                json!({ "$numberDouble": s })
            }
            Bson::Double(f) if f == 0.0 => {
                let s = if f.is_sign_negative() { "-0.0" } else { "0.0" };

                json!({ "$numberDouble": s })
            }
            Bson::DateTime(date) => {
                json!({ "$date": { "$numberLong": date.timestamp_millis().to_string() } })
            }
            Bson::Array(arr) => {
                Value::Array(arr.into_iter().map(Bson::into_canonical_extjson).collect())
            }
            Bson::Document(arr) => Value::Object(
                arr.into_iter()
                    .map(|(k, v)| (k, v.into_canonical_extjson()))
                    .collect(),
            ),
            Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope { code, scope }) => json!({
                "$code": code,
                "$scope": Bson::Document(scope).into_canonical_extjson(),
            }),

            other => other.into_relaxed_extjson(),
        }
    }
}
