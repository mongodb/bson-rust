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
    Int32(models::Int32),
    Symbol(models::Symbol),
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
            Bson::Int32(v) => ExtJsonInner::Int32(models::Int32::from(v)).into(),
            Bson::Symbol(s) => ExtJsonInner::Symbol(models::Symbol::from(s.clone())).into(),
            _ => todo!(),
        })
    }
}

impl TryFrom<ExtJson> for Bson {
    type Error = crate::error::Error;

    fn try_from(value: ExtJson) -> Result<Self, Self::Error> {
        Ok(match value.0 {
            ExtJsonInner::Int32(v) => Bson::Int32(v.parse()?),
            ExtJsonInner::Symbol(models::Symbol { value }) => Bson::Symbol(value),
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
}
