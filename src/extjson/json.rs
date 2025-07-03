use std::convert::{TryFrom, TryInto};

use serde::de::{Error as _, Unexpected};
use serde_json::{json, Value};

use crate::{
    error::{Error, Result},
    extjson::models,
    Binary,
    Bson,
    DbPointer,
    Document,
    JavaScriptCodeWithScope,
    Regex,
    Timestamp,
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
        if obj.contains_key("$oid") {
            let oid: models::ObjectId = serde_json::from_value(obj.into())?;
            return Ok(Bson::ObjectId(oid.parse()?));
        }

        if obj.contains_key("$symbol") {
            let symbol: models::Symbol = serde_json::from_value(obj.into())?;
            return Ok(Bson::Symbol(symbol.value));
        }

        if obj.contains_key("$regularExpression") {
            let regex: models::Regex = serde_json::from_value(obj.into())?;
            return Ok(regex.parse()?.into());
        }

        if obj.contains_key("$numberInt") {
            let int: models::Int32 = serde_json::from_value(obj.into())?;
            return Ok(Bson::Int32(int.parse()?));
        }

        if obj.contains_key("$numberLong") {
            let int: models::Int64 = serde_json::from_value(obj.into())?;
            return Ok(Bson::Int64(int.parse()?));
        }

        if obj.contains_key("$numberDouble") {
            let double: models::Double = serde_json::from_value(obj.into())?;
            return Ok(Bson::Double(double.parse()?));
        }

        if obj.contains_key("$numberDecimal") {
            let decimal: models::Decimal128 = serde_json::from_value(obj.into())?;
            return Ok(Bson::Decimal128(decimal.parse()?));
        }

        if obj.contains_key("$binary") {
            let binary: models::Binary = serde_json::from_value(obj.into())?;
            return Ok(Bson::Binary(binary.parse()?));
        }

        if obj.contains_key("$uuid") {
            let uuid: models::Uuid = serde_json::from_value(obj.into())?;
            return Ok(Bson::Binary(uuid.parse()?));
        }

        if obj.contains_key("$code") {
            let code_w_scope: models::JavaScriptCodeWithScope = serde_json::from_value(obj.into())?;
            return match code_w_scope.scope {
                Some(scope) => Ok(crate::JavaScriptCodeWithScope {
                    code: code_w_scope.code,
                    scope: scope.try_into()?,
                }
                .into()),
                None => Ok(Bson::JavaScriptCode(code_w_scope.code)),
            };
        }

        if obj.contains_key("$timestamp") {
            let ts: models::Timestamp = serde_json::from_value(obj.into())?;
            return Ok(ts.parse().into());
        }

        if obj.contains_key("$date") {
            let extjson_datetime: models::DateTime = serde_json::from_value(obj.into())?;
            return Ok(Bson::DateTime(extjson_datetime.parse()?));
        }

        if obj.contains_key("$minKey") {
            let min_key: models::MinKey = serde_json::from_value(obj.into())?;
            return min_key.parse();
        }

        if obj.contains_key("$maxKey") {
            let max_key: models::MaxKey = serde_json::from_value(obj.into())?;
            return max_key.parse();
        }

        if obj.contains_key("$dbPointer") {
            let db_ptr: models::DbPointer = serde_json::from_value(obj.into())?;
            return Ok(db_ptr.parse()?.into());
        }

        if obj.contains_key("$undefined") {
            let undefined: models::Undefined = serde_json::from_value(obj.into())?;
            return undefined.parse();
        }

        Ok(Bson::Document(obj.try_into()?))
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
