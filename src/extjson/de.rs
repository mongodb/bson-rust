//! Deserializing [MongoDB Extended JSON v2](https://docs.mongodb.com/manual/reference/mongodb-extended-json/)
//!
//! ## Usage
//!
//! Extended JSON can be deserialized using [`Bson`](../../enum.Bson.html)'s
//! `TryFrom<serde_json::Value>` implementation. This implementation accepts both canonical and
//! relaxed extJSON, and the two modes can even be mixed within a single representation.
//!
//! e.g.
//! ```rust
//! # use bson::Bson;
//! # use serde_json::json;
//! # use std::convert::{TryFrom, TryInto};
//! let json_doc = json!({ "x": 5i32, "y": { "$numberInt": "5" }, "z": { "subdoc": "hello" } });
//! let bson: Bson = json_doc.try_into().unwrap(); // Bson::Document(...)
//!
//! let json_date = json!({ "$date": { "$numberLong": "1590972160292" } });
//! let bson_date: Bson = json_date.try_into().unwrap(); // Bson::DateTime(...)
//!
//! let invalid_ext_json = json!({ "$numberLong": 5 });
//! Bson::try_from(invalid_ext_json).expect_err("5 should be a string");
//! ```

use std::convert::{TryFrom, TryInto};

use serde::de::{Error as _, Unexpected};

use crate::{extjson::models, oid, Bson, Document};

#[derive(Clone, Debug)]
#[non_exhaustive]
/// Error cases that can occur during deserialization from [extended JSON](https://docs.mongodb.com/manual/reference/mongodb-extended-json/).
pub enum Error {
    /// Errors that can occur during OID construction and generation from the input data.
    InvalidObjectId(oid::Error),

    /// A general error encountered during deserialization.
    /// See: https://docs.serde.rs/serde/de/trait.Error.html
    DeserializationError { message: String },
}

impl std::fmt::Display for Error {
    fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Self::InvalidObjectId(ref err) => err.fmt(fmt),
            Self::DeserializationError { ref message } => message.fmt(fmt),
        }
    }
}

impl std::error::Error for Error {}

impl serde::de::Error for Error {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::DeserializationError {
            message: format!("{}", msg),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Self::DeserializationError {
            message: err.to_string(),
        }
    }
}

impl From<oid::Error> for Error {
    fn from(err: oid::Error) -> Self {
        Self::InvalidObjectId(err)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// This converts from the input JSON object as if it were [MongoDB Extended JSON v2](https://docs.mongodb.com/manual/reference/mongodb-extended-json/).
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
            return Ok(regex.parse().into());
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

        if obj.contains_key("$numberDecimal") {
            #[cfg(feature = "decimal128")]
            {
                let decimal: models::Decimal128 = serde_json::from_value(obj.into())?;
                return Ok(Bson::Decimal128(decimal.parse()?));
            }

            #[cfg(not(feature = "decimal128"))]
            {
                return Err(Error::custom("decimal128 extjson support not implemented"));
            }
        }

        if obj.contains_key("$undefined") {
            let undefined: models::Undefined = serde_json::from_value(obj.into())?;
            return undefined.parse();
        }

        Ok(Bson::Document(obj.try_into()?))
    }
}

/// This converts from the input JSON as if it were [MongoDB Extended JSON v2](https://docs.mongodb.com/manual/reference/mongodb-extended-json/).
impl TryFrom<serde_json::Value> for Bson {
    type Error = Error;

    fn try_from(value: serde_json::Value) -> Result<Self> {
        match value {
            serde_json::Value::Number(x) => x
                .as_i64()
                .map(|i| {
                    if i >= std::i32::MIN as i64 && i <= std::i32::MAX as i64 {
                        Bson::Int32(i as i32)
                    } else {
                        Bson::Int64(i)
                    }
                })
                .or_else(|| x.as_u64().map(Bson::from))
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

/// This converts from the input JSON as if it were [MongoDB Extended JSON v2](https://docs.mongodb.com/manual/reference/mongodb-extended-json/).
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
