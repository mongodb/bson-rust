//! Support for the `facet` crate.
use facet::Facet;

use crate::{extjson::models, Bson};

/// A type for use with #[facet(proxy)] that represents BSON values in their canonical extended JSON
/// form.
#[derive(Facet, Debug)]
#[facet(transparent)]
pub struct ExtJson(ExtJsonInner);

#[derive(Facet, Debug)]
#[facet(untagged)]
#[repr(C)]
enum ExtJsonInner {
    Double(models::Double),
    RegularExpression(models::Regex),
    JavaScriptCode(models::JavaScriptCode),
    Int32(models::Int32),
    Int64(models::Int64),
    Timestamp(models::Timestamp),
    Binary(models::Binary),
    ObjectId(models::ObjectId),
    DateTime(models::DateTime),
    Symbol(models::Symbol),
    Decimal128(models::Decimal128),
    Undefined(models::Undefined),
    MaxKey(models::MaxKey),
    MinKey(models::MinKey),
    DbPointer(models::DbPointer),
    Boolean(bool),
    Null(()),
    String(String),
    Array(Vec<ExtJson>),
}

impl From<ExtJsonInner> for ExtJson {
    fn from(value: ExtJsonInner) -> Self {
        Self(value)
    }
}

impl TryFrom<&Bson> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &Bson) -> Result<Self, Self::Error> {
        Ok(match value {
            Bson::Double(v) => ExtJsonInner::Double(models::Double::from(v)).into(),
            Bson::String(s) => ExtJsonInner::String(s.clone()).into(),
            Bson::Boolean(b) => ExtJsonInner::Boolean(*b).into(),
            Bson::Null => ExtJsonInner::Null(()).into(),
            Bson::RegularExpression(r) => {
                ExtJsonInner::RegularExpression(models::Regex::from(r)).into()
            }
            Bson::JavaScriptCode(s) => {
                ExtJsonInner::JavaScriptCode(models::JavaScriptCode::from(s.as_str())).into()
            }
            Bson::JavaScriptCodeWithScope(_) => todo!(),
            Bson::Int32(v) => ExtJsonInner::Int32(models::Int32::from(v)).into(),
            Bson::Int64(v) => ExtJsonInner::Int64(models::Int64::from(v)).into(),
            Bson::Timestamp(ts) => ExtJsonInner::Timestamp(models::Timestamp::from(*ts)).into(),
            Bson::Binary(b) => ExtJsonInner::Binary(models::Binary::from(b)).into(),
            Bson::ObjectId(id) => ExtJsonInner::ObjectId(models::ObjectId::from(*id)).into(),
            Bson::DateTime(dt) => ExtJsonInner::DateTime(models::DateTime::from(*dt)).into(),
            Bson::Symbol(s) => ExtJsonInner::Symbol(models::Symbol::from(s.clone())).into(),
            Bson::Decimal128(d) => ExtJsonInner::Decimal128(models::Decimal128::from(d)).into(),
            Bson::Undefined => ExtJsonInner::Undefined(models::Undefined { value: true }).into(),
            Bson::MaxKey => ExtJsonInner::MaxKey(models::MaxKey { value: 1 }).into(),
            Bson::MinKey => ExtJsonInner::MinKey(models::MinKey { value: 1 }).into(),
            Bson::DbPointer(dp) => ExtJsonInner::DbPointer(models::DbPointer::from(dp)).into(),
            Bson::Array(arr) => ExtJsonInner::Array(
                arr.into_iter()
                    .map(|v| v.try_into())
                    .collect::<Result<Vec<_>, _>>()?,
            )
            .into(),
            Bson::Document(_) => todo!(),
        })
    }
}

impl TryFrom<ExtJson> for Bson {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        Ok(match value.0 {
            ExtJsonInner::Double(v) => Bson::Double(v.parse()?),
            ExtJsonInner::String(s) => Bson::String(s),
            ExtJsonInner::Boolean(b) => Bson::Boolean(b),
            ExtJsonInner::Null(()) => Bson::Null,
            ExtJsonInner::RegularExpression(r) => Bson::RegularExpression(r.parse()?),
            ExtJsonInner::JavaScriptCode(models::JavaScriptCode { code }) => {
                Bson::JavaScriptCode(code)
            }
            ExtJsonInner::Int32(v) => Bson::Int32(v.parse()?),
            ExtJsonInner::Int64(v) => Bson::Int64(v.parse()?),
            ExtJsonInner::Timestamp(v) => Bson::Timestamp(v.parse()),
            ExtJsonInner::Binary(v) => Bson::Binary(v.parse()?),
            ExtJsonInner::ObjectId(v) => Bson::ObjectId(v.parse()?),
            ExtJsonInner::DateTime(v) => Bson::DateTime(v.parse()?),
            ExtJsonInner::Symbol(models::Symbol { value }) => Bson::Symbol(value),
            ExtJsonInner::Decimal128(v) => Bson::Decimal128(v.parse()?),
            ExtJsonInner::Undefined(v) => v.parse()?,
            ExtJsonInner::MaxKey(v) => v.parse()?,
            ExtJsonInner::MinKey(v) => v.parse()?,
            ExtJsonInner::DbPointer(v) => Bson::DbPointer(v.parse()?),
            ExtJsonInner::Array(v) => Bson::Array(
                v.into_iter()
                    .map(|bv| bv.try_into())
                    .collect::<crate::error::Result<Vec<Bson>>>()?,
            ),
        })
    }
}

impl TryFrom<&i32> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &i32) -> Result<Self, Self::Error> {
        Ok(ExtJsonInner::Int32(models::Int32::from(value)).into())
    }
}

impl TryFrom<ExtJson> for i32 {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0 {
            ExtJsonInner::Int32(model) => model.parse(),
            other => Err(parse_err!("expected Int32, got {other:?}")),
        }
    }
}

impl TryFrom<&i64> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &i64) -> Result<Self, Self::Error> {
        Ok(ExtJsonInner::Int64(models::Int64::from(value)).into())
    }
}

impl TryFrom<ExtJson> for i64 {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0 {
            ExtJsonInner::Int64(model) => model.parse(),
            other => Err(parse_err!("expected Int64, got {other:?}")),
        }
    }
}

impl TryFrom<&f64> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &f64) -> Result<Self, Self::Error> {
        Ok(ExtJsonInner::Double(models::Double::from(value)).into())
    }
}

impl TryFrom<ExtJson> for f64 {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0 {
            ExtJsonInner::Double(model) => model.parse(),
            other => Err(parse_err!("expected Double, got {other:?}")),
        }
    }
}

impl TryFrom<&crate::oid::ObjectId> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &crate::oid::ObjectId) -> Result<Self, Self::Error> {
        Ok(ExtJsonInner::ObjectId(models::ObjectId::from(*value)).into())
    }
}

impl TryFrom<ExtJson> for crate::oid::ObjectId {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0 {
            ExtJsonInner::ObjectId(model) => model.parse(),
            other => Err(parse_err!("expected ObjectId, got {other:?}")),
        }
    }
}

impl TryFrom<&crate::DateTime> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &crate::DateTime) -> Result<Self, Self::Error> {
        Ok(ExtJsonInner::DateTime(models::DateTime::from(*value)).into())
    }
}

impl TryFrom<ExtJson> for crate::DateTime {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0 {
            ExtJsonInner::DateTime(model) => model.parse(),
            other => Err(parse_err!("expected DateTime, got {other:?}")),
        }
    }
}

impl TryFrom<&crate::Decimal128> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &crate::Decimal128) -> Result<Self, Self::Error> {
        Ok(ExtJsonInner::Decimal128(models::Decimal128::from(value)).into())
    }
}

impl TryFrom<ExtJson> for crate::Decimal128 {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0 {
            ExtJsonInner::Decimal128(model) => model.parse(),
            other => Err(parse_err!("expected Decimal128, got {other:?}")),
        }
    }
}

impl TryFrom<&crate::Binary> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &crate::Binary) -> Result<Self, Self::Error> {
        Ok(ExtJsonInner::Binary(models::Binary::from(value)).into())
    }
}

impl TryFrom<ExtJson> for crate::Binary {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0 {
            ExtJsonInner::Binary(model) => model.parse(),
            other => Err(parse_err!("expected Binary, got {other:?}")),
        }
    }
}

impl TryFrom<&crate::Timestamp> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &crate::Timestamp) -> Result<Self, Self::Error> {
        Ok(ExtJsonInner::Timestamp(models::Timestamp::from(*value)).into())
    }
}

impl TryFrom<ExtJson> for crate::Timestamp {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0 {
            ExtJsonInner::Timestamp(model) => Ok(model.parse()),
            other => Err(parse_err!("expected Timestamp, got {other:?}")),
        }
    }
}

impl TryFrom<&crate::Regex> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &crate::Regex) -> Result<Self, Self::Error> {
        Ok(ExtJsonInner::RegularExpression(models::Regex::from(value)).into())
    }
}

impl TryFrom<ExtJson> for crate::Regex {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0 {
            ExtJsonInner::RegularExpression(model) => model.parse(),
            other => Err(parse_err!("expected Regex, got {other:?}")),
        }
    }
}

impl TryFrom<&crate::DbPointer> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &crate::DbPointer) -> Result<Self, Self::Error> {
        Ok(ExtJsonInner::DbPointer(models::DbPointer::from(value)).into())
    }
}

impl TryFrom<ExtJson> for crate::DbPointer {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0 {
            ExtJsonInner::DbPointer(model) => model.parse(),
            other => Err(parse_err!("expected DbPointer, got {other:?}")),
        }
    }
}

impl TryFrom<&String> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Ok(ExtJsonInner::String(value.clone()).into())
    }
}

impl TryFrom<ExtJson> for String {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0 {
            ExtJsonInner::String(s) => Ok(s),
            other => Err(parse_err!("expected String, got {other:?}")),
        }
    }
}

impl TryFrom<&bool> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &bool) -> Result<Self, Self::Error> {
        Ok(ExtJsonInner::Boolean(*value).into())
    }
}

impl TryFrom<ExtJson> for bool {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0 {
            ExtJsonInner::Boolean(b) => Ok(b),
            other => Err(parse_err!("expected Boolean, got {other:?}")),
        }
    }
}

impl TryFrom<&crate::Array> for ExtJson {
    type Error = std::convert::Infallible;

    fn try_from(value: &crate::Array) -> Result<Self, Self::Error> {
        Ok(ExtJsonInner::Array(
            value
                .into_iter()
                .map(|v| v.try_into())
                .collect::<Result<Vec<_>, _>>()?,
        )
        .into())
    }
}

impl TryFrom<ExtJson> for crate::Array {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        match value.0 {
            ExtJsonInner::Array(arr) => arr.into_iter().map(|v| v.try_into()).collect(),
            other => Err(parse_err!("expected Array, got {other:?}")),
        }
    }
}

macro_rules! parse_err {
    ($fmt:literal $(, $a:expr)*) => {{
        crate::error::Error::deserialization(format!($fmt $(, $a)*))
    }};
}
use parse_err;

#[cfg(test)]
mod test {
    use facet::Facet;
    use facet_json;

    use crate::Bson;

    use super::ExtJson;

    fn assert_roundtrip<T: Facet<'static> + PartialEq + std::fmt::Debug>(
        value: &T,
        expected: &str,
    ) {
        let json = facet_json::to_string_pretty(value).unwrap();
        assert_eq!(json, expected);
        let back: T = facet_json::from_str(&json).unwrap();
        assert_eq!(value, &back);
    }

    #[test]
    fn roundtrip_i32() {
        #[derive(Debug, Facet, PartialEq)]
        struct Foo {
            a: i32,
            #[facet(proxy = ExtJson)]
            b: i32,
            #[facet(opaque, proxy = ExtJson)]
            c: Bson,
        }
        assert_roundtrip(
            &Foo {
                a: 13,
                b: 42,
                c: Bson::Int32(1066),
            },
            r#"{
  "a": 13,
  "b": {
    "$numberInt": "42"
  },
  "c": {
    "$numberInt": "1066"
  }
}"#,
        );
    }

    #[test]
    fn roundtrip_symbol() {
        #[derive(Debug, PartialEq, Facet)]
        struct Foo {
            #[facet(opaque, proxy = ExtJson)]
            s: Bson,
        }
        assert_roundtrip(
            &Foo {
                s: Bson::Symbol("hello".into()),
            },
            r#"{
  "s": {
    "$symbol": "hello"
  }
}"#,
        );
    }

    #[test]
    fn roundtrip_double() {
        #[derive(Debug, PartialEq, Facet)]
        struct Foo {
            #[facet(proxy = ExtJson)]
            v: f64,
            #[facet(opaque, proxy = ExtJson)]
            b: Bson,
        }
        assert_roundtrip(
            &Foo {
                v: 1.5,
                b: Bson::Double(2.5),
            },
            r#"{
  "v": {
    "$numberDouble": "1.5"
  },
  "b": {
    "$numberDouble": "2.5"
  }
}"#,
        );
    }

    #[test]
    fn roundtrip_i64() {
        #[derive(Debug, PartialEq, Facet)]
        struct Foo {
            #[facet(proxy = ExtJson)]
            v: i64,
            #[facet(opaque, proxy = ExtJson)]
            b: Bson,
        }
        assert_roundtrip(
            &Foo {
                v: 9_000_000_000,
                b: Bson::Int64(1_000_000_000_000),
            },
            r#"{
  "v": {
    "$numberLong": "9000000000"
  },
  "b": {
    "$numberLong": "1000000000000"
  }
}"#,
        );
    }

    #[test]
    fn roundtrip_object_id() {
        #[derive(Debug, PartialEq, Facet)]
        struct Foo {
            #[facet(opaque, proxy = ExtJson)]
            v: crate::oid::ObjectId,
            #[facet(opaque, proxy = ExtJson)]
            b: Bson,
        }
        let id = crate::oid::ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap();
        assert_roundtrip(
            &Foo {
                v: id,
                b: Bson::ObjectId(id),
            },
            r#"{
  "v": {
    "$oid": "507f1f77bcf86cd799439011"
  },
  "b": {
    "$oid": "507f1f77bcf86cd799439011"
  }
}"#,
        );
    }

    #[test]
    fn roundtrip_datetime() {
        #[derive(Debug, PartialEq, Facet)]
        struct Foo {
            #[facet(opaque, proxy = ExtJson)]
            v: crate::DateTime,
            #[facet(opaque, proxy = ExtJson)]
            b: Bson,
        }
        let dt = crate::DateTime::from_millis(1_000_000_000_000);
        assert_roundtrip(
            &Foo {
                v: dt,
                b: Bson::DateTime(dt),
            },
            r#"{
  "v": {
    "$date": {
      "$numberLong": "1000000000000"
    }
  },
  "b": {
    "$date": {
      "$numberLong": "1000000000000"
    }
  }
}"#,
        );
    }

    #[test]
    fn roundtrip_binary() {
        #[derive(Debug, PartialEq, Facet)]
        struct Foo {
            #[facet(opaque, proxy = ExtJson)]
            v: crate::Binary,
            #[facet(opaque, proxy = ExtJson)]
            b: Bson,
        }
        let bin = crate::Binary {
            subtype: crate::spec::BinarySubtype::Generic,
            bytes: vec![1, 2, 3],
        };
        assert_roundtrip(
            &Foo {
                v: bin.clone(),
                b: Bson::Binary(bin),
            },
            r#"{
  "v": {
    "$binary": {
      "base64": "AQID",
      "subType": "00"
    }
  },
  "b": {
    "$binary": {
      "base64": "AQID",
      "subType": "00"
    }
  }
}"#,
        );
    }

    #[test]
    fn roundtrip_timestamp() {
        #[derive(Debug, PartialEq, Facet)]
        struct Foo {
            #[facet(opaque, proxy = ExtJson)]
            v: crate::Timestamp,
            #[facet(opaque, proxy = ExtJson)]
            b: Bson,
        }
        let ts = crate::Timestamp {
            time: 1234,
            increment: 5,
        };
        assert_roundtrip(
            &Foo {
                v: ts,
                b: Bson::Timestamp(ts),
            },
            r#"{
  "v": {
    "$timestamp": {
      "t": 1234,
      "i": 5
    }
  },
  "b": {
    "$timestamp": {
      "t": 1234,
      "i": 5
    }
  }
}"#,
        );
    }

    #[test]
    fn roundtrip_decimal128() {
        #[derive(Debug, PartialEq, Facet)]
        struct Foo {
            #[facet(opaque, proxy = ExtJson)]
            v: crate::Decimal128,
            #[facet(opaque, proxy = ExtJson)]
            b: Bson,
        }
        let d: crate::Decimal128 = "3.14".parse().unwrap();
        assert_roundtrip(
            &Foo {
                v: d,
                b: Bson::Decimal128(d),
            },
            r#"{
  "v": {
    "$numberDecimal": "3.14"
  },
  "b": {
    "$numberDecimal": "3.14"
  }
}"#,
        );
    }

    #[test]
    fn roundtrip_regex() {
        #[derive(Debug, PartialEq, Facet)]
        struct Foo {
            #[facet(opaque, proxy = ExtJson)]
            v: crate::Regex,
            #[facet(opaque, proxy = ExtJson)]
            b: Bson,
        }
        let r = crate::Regex::from_strings("abc", "i").unwrap();
        assert_roundtrip(
            &Foo {
                v: r.clone(),
                b: Bson::RegularExpression(r),
            },
            r#"{
  "v": {
    "$regularExpression": {
      "pattern": "abc",
      "options": "i"
    }
  },
  "b": {
    "$regularExpression": {
      "pattern": "abc",
      "options": "i"
    }
  }
}"#,
        );
    }

    #[test]
    fn roundtrip_db_pointer() {
        #[derive(Debug, PartialEq, Facet)]
        struct Foo {
            #[facet(opaque, proxy = ExtJson)]
            v: crate::DbPointer,
            #[facet(opaque, proxy = ExtJson)]
            b: Bson,
        }
        let id = crate::oid::ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap();
        let dp = crate::DbPointer {
            namespace: "test.coll".to_string(),
            id,
        };
        assert_roundtrip(
            &Foo {
                v: dp.clone(),
                b: Bson::DbPointer(dp),
            },
            r#"{
  "v": {
    "$dbPointer": {
      "$ref": "test.coll",
      "$id": {
        "$oid": "507f1f77bcf86cd799439011"
      }
    }
  },
  "b": {
    "$dbPointer": {
      "$ref": "test.coll",
      "$id": {
        "$oid": "507f1f77bcf86cd799439011"
      }
    }
  }
}"#,
        );
    }

    #[test]
    fn roundtrip_bson_string_and_bool() {
        #[derive(Debug, PartialEq, Facet)]
        struct Foo {
            #[facet(proxy = ExtJson)]
            sv: String,
            #[facet(opaque, proxy = ExtJson)]
            sb: Bson,
            #[facet(proxy = ExtJson)]
            bv: bool,
            #[facet(opaque, proxy = ExtJson)]
            bb: Bson,
            #[facet(opaque, proxy = ExtJson)]
            n: Bson,
        }
        assert_roundtrip(
            &Foo {
                sv: "hello".into(),
                sb: Bson::String("hello".into()),
                bv: true,
                bb: Bson::Boolean(true),
                n: Bson::Null,
            },
            r#"{
  "sv": "hello",
  "sb": "hello",
  "bv": true,
  "bb": true,
  "n": null
}"#,
        );
    }

    #[test]
    fn roundtrip_array() {
        #[derive(Debug, PartialEq, Facet)]
        struct Foo {
            #[facet(opaque, proxy = ExtJson)]
            a: Bson,
            #[facet(opaque, proxy = ExtJson)]
            b: crate::Array,
        }
        let arr = vec![
            Bson::Int32(1),
            Bson::String("hello".into()),
            Bson::Boolean(false),
            Bson::Array(vec![Bson::Int64(9_000_000_000)]),
        ];
        assert_roundtrip(
            &Foo {
                a: Bson::Array(arr.clone()),
                b: arr,
            },
            r#"{
  "a": [
    {
      "$numberInt": "1"
    },
    "hello",
    false,
    [
      {
        "$numberLong": "9000000000"
      }
    ]
  ],
  "b": [
    {
      "$numberInt": "1"
    },
    "hello",
    false,
    [
      {
        "$numberLong": "9000000000"
      }
    ]
  ]
}"#,
        );
    }
}
