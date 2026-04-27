//! Support for the `facet` crate.

pub(crate) mod opaque;
mod parse;
mod ser;

use facet::Facet;

use crate::{
    Bson,
    Document,
    JavaScriptCodeWithScope,
    extjson::models::{self, ObjectType, parse_err},
};

pub use parse::deserialize_from_slice;
pub use ser::serialize_to_vec;

/// A type for use with #[facet(proxy)] that represents BSON values in their canonical extended JSON
/// form.
#[derive(Facet, Debug)]
#[facet(transparent)]
pub struct ExtJson(facet_value::Value);

type ToValueError = facet_format::SerializeError<facet_value::ToValueError>;

impl TryFrom<&Bson> for ExtJson {
    type Error = ToValueError;

    fn try_from(value: &Bson) -> Result<Self, Self::Error> {
        match value {
            Bson::Double(v) => v.try_into(),
            Bson::String(s) => s.try_into(),
            Bson::Boolean(b) => b.try_into(),
            Bson::Null => Ok(ExtJson(facet_value::Value::NULL)),
            Bson::RegularExpression(r) => r.try_into(),
            Bson::JavaScriptCode(s) => {
                facet_value::to_value(&models::JavaScriptCode::from(s.as_str())).map(ExtJson)
            }
            Bson::JavaScriptCodeWithScope(jsc) => jsc.try_into(),
            Bson::Int32(v) => v.try_into(),
            Bson::Int64(v) => v.try_into(),
            Bson::Timestamp(ts) => ts.try_into(),
            Bson::Binary(b) => b.try_into(),
            Bson::ObjectId(id) => id.try_into(),
            Bson::DateTime(dt) => dt.try_into(),
            Bson::Symbol(s) => facet_value::to_value(&models::Symbol::from(s.clone())).map(ExtJson),
            Bson::Decimal128(d) => d.try_into(),
            Bson::Undefined => {
                facet_value::to_value(&models::Undefined { value: true }).map(ExtJson)
            }
            Bson::MaxKey => facet_value::to_value(&models::MaxKey { value: 1 }).map(ExtJson),
            Bson::MinKey => facet_value::to_value(&models::MinKey { value: 1 }).map(ExtJson),
            Bson::DbPointer(dp) => dp.try_into(),
            Bson::Array(arr) => arr.try_into(),
            Bson::Document(doc) => doc.try_into(),
        }
    }
}

impl TryFrom<ExtJson> for Bson {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        use facet_value::Destructured;

        match value.0.destructure() {
            Destructured::String(s) => Ok(Bson::String(s.into())),
            Destructured::Bool(b) => Ok(Bson::Boolean(b)),
            Destructured::Null => Ok(Bson::Null),
            Destructured::Array(arr) => Ok(Bson::Array(
                arr.into_iter()
                    .map(|v| Bson::try_from(ExtJson(v)))
                    .collect::<Result<Vec<_>, Self::Error>>()?,
            )),
            Destructured::Object(obj) => {
                let elt = {
                    let keys: Vec<&str> = obj.keys().map(|vs| vs.as_str()).collect();
                    ObjectType::from_keys(&keys)
                };
                let v = ExtJson(obj.into_value());
                match elt {
                    ObjectType::Double => f64::try_from(v).map(Bson::Double),
                    ObjectType::Int32 => i32::try_from(v).map(Bson::Int32),
                    ObjectType::Int64 => i64::try_from(v).map(Bson::Int64),
                    ObjectType::Decimal128 => crate::Decimal128::try_from(v).map(Bson::Decimal128),
                    ObjectType::ObjectId => crate::oid::ObjectId::try_from(v).map(Bson::ObjectId),
                    ObjectType::Binary => crate::Binary::try_from(v).map(Bson::Binary),
                    ObjectType::Uuid => facet_value::from_value::<models::Uuid>(v.0)
                        .map_err(|e| parse_err!("{e}"))
                        .and_then(|u| u.parse())
                        .map(Bson::Binary),
                    ObjectType::Timestamp => crate::Timestamp::try_from(v).map(Bson::Timestamp),
                    ObjectType::RegularExpression => {
                        crate::Regex::try_from(v).map(Bson::RegularExpression)
                    }
                    ObjectType::DbPointer => crate::DbPointer::try_from(v).map(Bson::DbPointer),
                    ObjectType::DateTime => crate::DateTime::try_from(v).map(Bson::DateTime),
                    ObjectType::Symbol => facet_value::from_value::<models::Symbol>(v.0)
                        .map_err(|e| parse_err!("{e}"))
                        .map(|m| Bson::Symbol(m.value)),
                    ObjectType::JavaScriptCode => {
                        facet_value::from_value::<models::JavaScriptCode>(v.0)
                            .map_err(|e| parse_err!("{e}"))
                            .map(|m| Bson::JavaScriptCode(m.code))
                    }
                    ObjectType::JavaScriptCodeWithScope => {
                        JavaScriptCodeWithScope::try_from(v).map(Bson::JavaScriptCodeWithScope)
                    }
                    ObjectType::Undefined => facet_value::from_value::<models::Undefined>(v.0)
                        .map_err(|e| parse_err!("{e}"))
                        .and_then(|m| m.parse()),
                    ObjectType::MaxKey => facet_value::from_value::<models::MaxKey>(v.0)
                        .map_err(|e| parse_err!("{e}"))
                        .and_then(|m| m.parse()),
                    ObjectType::MinKey => facet_value::from_value::<models::MinKey>(v.0)
                        .map_err(|e| parse_err!("{e}"))
                        .and_then(|m| m.parse()),
                    ObjectType::Document => Document::try_from(v).map(Bson::Document),
                }
            }
            _ => Err(parse_err!("unexpected value type")),
        }
    }
}

impl TryFrom<&i32> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &i32) -> Result<Self, Self::Error> {
        facet_value::to_value(&models::Int32::from(value)).map(ExtJson)
    }
}
impl TryFrom<ExtJson> for i32 {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        facet_value::from_value::<models::Int32>(value.0)
            .map_err(|e| parse_err!("{e}"))
            .and_then(|m| m.parse())
    }
}

impl TryFrom<&i64> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &i64) -> Result<Self, Self::Error> {
        facet_value::to_value(&models::Int64::from(value)).map(ExtJson)
    }
}
impl TryFrom<ExtJson> for i64 {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        facet_value::from_value::<models::Int64>(value.0)
            .map_err(|e| parse_err!("{e}"))
            .and_then(|m| m.parse())
    }
}

impl TryFrom<&f64> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &f64) -> Result<Self, Self::Error> {
        facet_value::to_value(&models::Double::from(value)).map(ExtJson)
    }
}
impl TryFrom<ExtJson> for f64 {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        facet_value::from_value::<models::Double>(value.0)
            .map_err(|e| parse_err!("{e}"))
            .and_then(|m| m.parse())
    }
}

impl TryFrom<&crate::Decimal128> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &crate::Decimal128) -> Result<Self, Self::Error> {
        facet_value::to_value(&models::Decimal128::from(value)).map(ExtJson)
    }
}
impl TryFrom<ExtJson> for crate::Decimal128 {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        facet_value::from_value::<models::Decimal128>(value.0)
            .map_err(|e| parse_err!("{e}"))
            .and_then(|m| m.parse())
    }
}

impl TryFrom<&crate::oid::ObjectId> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &crate::oid::ObjectId) -> Result<Self, Self::Error> {
        facet_value::to_value(&models::ObjectId::from(*value)).map(ExtJson)
    }
}
impl TryFrom<ExtJson> for crate::oid::ObjectId {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        facet_value::from_value::<models::ObjectId>(value.0)
            .map_err(|e| parse_err!("{e}"))
            .and_then(|m| m.parse())
    }
}

impl TryFrom<&crate::Binary> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &crate::Binary) -> Result<Self, Self::Error> {
        facet_value::to_value(&models::Binary::from(value)).map(ExtJson)
    }
}
impl TryFrom<ExtJson> for crate::Binary {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        facet_value::from_value::<models::Binary>(value.0)
            .map_err(|e| parse_err!("{e}"))
            .and_then(|m| m.parse())
    }
}

impl TryFrom<&crate::Timestamp> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &crate::Timestamp) -> Result<Self, Self::Error> {
        facet_value::to_value(&models::Timestamp::from(*value)).map(ExtJson)
    }
}
impl TryFrom<ExtJson> for crate::Timestamp {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        facet_value::from_value::<models::Timestamp>(value.0)
            .map_err(|e| parse_err!("{e}"))
            .map(|m| m.parse())
    }
}

impl TryFrom<&crate::Regex> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &crate::Regex) -> Result<Self, Self::Error> {
        facet_value::to_value(&models::Regex::from(value)).map(ExtJson)
    }
}
impl TryFrom<ExtJson> for crate::Regex {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        facet_value::from_value::<models::Regex>(value.0)
            .map_err(|e| parse_err!("{e}"))
            .and_then(|m| m.parse())
    }
}

impl TryFrom<&crate::DbPointer> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &crate::DbPointer) -> Result<Self, Self::Error> {
        facet_value::to_value(&models::DbPointer::from(value)).map(ExtJson)
    }
}
impl TryFrom<ExtJson> for crate::DbPointer {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        facet_value::from_value::<models::DbPointer>(value.0)
            .map_err(|e| parse_err!("{e}"))
            .and_then(|m| m.parse())
    }
}

impl TryFrom<&crate::DateTime> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &crate::DateTime) -> Result<Self, Self::Error> {
        facet_value::to_value(&models::DateTime::from(*value)).map(ExtJson)
    }
}

impl TryFrom<ExtJson> for crate::DateTime {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        facet_value::from_value::<models::DateTime>(value.0)
            .map_err(|e| parse_err!("{e}"))
            .and_then(|m| m.parse())
    }
}

impl TryFrom<&JavaScriptCodeWithScope> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &JavaScriptCodeWithScope) -> Result<Self, Self::Error> {
        let scope: ExtJson = (&value.scope).try_into()?;
        facet_value::to_value(&models::JavaScriptCodeWithScope {
            code: value.code.clone(),
            scope: Some(scope),
        })
        .map(ExtJson)
    }
}

impl TryFrom<ExtJson> for JavaScriptCodeWithScope {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        let models::JavaScriptCodeWithScope::<facet_value::Value> { code, scope } =
            facet_value::from_value(value.0).map_err(|e| parse_err!("{e}"))?;
        let scope = Document::try_from(ExtJson(scope))?;
        Ok(JavaScriptCodeWithScope { code, scope })
    }
}

impl TryFrom<&String> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &String) -> Result<Self, Self::Error> {
        facet_value::to_value(value).map(ExtJson)
    }
}
impl TryFrom<ExtJson> for String {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        facet_value::from_value::<String>(value.0).map_err(|e| parse_err!("{e}"))
    }
}

impl TryFrom<&bool> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &bool) -> Result<Self, Self::Error> {
        facet_value::to_value(value).map(ExtJson)
    }
}
impl TryFrom<ExtJson> for bool {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        facet_value::from_value::<bool>(value.0).map_err(|e| parse_err!("{e}"))
    }
}

impl TryFrom<&crate::Array> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &crate::Array) -> Result<Self, Self::Error> {
        let items = value
            .iter()
            .map(ExtJson::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        facet_value::to_value(&items).map(ExtJson)
    }
}
impl TryFrom<ExtJson> for crate::Array {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        use facet_value::Destructured;
        match value.0.destructure() {
            Destructured::Array(arr) => arr
                .into_iter()
                .map(|v| Bson::try_from(ExtJson(v)))
                .collect(),
            other => Err(parse_err!("expected array, got {other:?}")),
        }
    }
}

impl TryFrom<&Document> for ExtJson {
    type Error = ToValueError;
    fn try_from(value: &Document) -> Result<Self, Self::Error> {
        let mut obj = facet_value::VObject::with_capacity(value.len());
        for (k, v) in value {
            let ExtJson(v) = v.try_into()?;
            obj.insert(k, v);
        }
        Ok(ExtJson(obj.into_value()))
    }
}
impl TryFrom<ExtJson> for Document {
    type Error = crate::error::Error;
    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        use facet_value::Destructured;

        match value.0.destructure() {
            Destructured::Object(obj) => {
                let mut doc = Document::new();
                for (k, v) in obj {
                    doc.insert(k, Bson::try_from(ExtJson(v))?);
                }
                Ok(doc)
            }
            other => Err(parse_err!("expected object, got {other:?}")),
        }
    }
}
