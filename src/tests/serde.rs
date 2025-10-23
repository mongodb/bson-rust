#![allow(clippy::disallowed_names)]

mod json;
mod serialize_deserialize;
pub(crate) mod util;

use std::{
    collections::BTreeMap,
    convert::{TryFrom, TryInto},
};

use crate::{
    bson,
    cstr,
    deserialize_from_bson,
    doc,
    oid::ObjectId,
    serialize_to_bson,
    spec::BinarySubtype,
    tests::LOCK,
    Binary,
    Bson,
    DateTime,
    Deserializer,
    Document,
    Serializer,
};

use serde::{Deserialize, Serialize};
use serde_json::json;

#[test]
fn test_ser_vec() {
    let _guard = LOCK.run_concurrently();
    let vec = vec![1, 2, 3];

    let serializer = Serializer::new();
    let result = vec.serialize(serializer).unwrap();

    let expected = bson!([1, 2, 3]);
    assert_eq!(expected, result);
}

#[test]
fn test_ser_map() {
    let _guard = LOCK.run_concurrently();
    let mut map = BTreeMap::new();
    map.insert("x", 0);
    map.insert("y", 1);

    let serializer = Serializer::new();
    let result = map.serialize(serializer).unwrap();

    let expected = bson!({ "x": 0, "y": 1 });
    assert_eq!(expected, result);
}

#[test]
fn test_de_vec() {
    let _guard = LOCK.run_concurrently();
    let bson = bson!([1, 2, 3]);

    let deserializer = Deserializer::new(bson);
    let vec = Vec::<i32>::deserialize(deserializer).unwrap();

    let expected = vec![1, 2, 3];
    assert_eq!(expected, vec);
}

#[test]
fn test_de_map() {
    let _guard = LOCK.run_concurrently();
    let bson = bson!({ "x": 0, "y": 1 });

    let deserializer = Deserializer::new(bson);
    let map = BTreeMap::<String, i32>::deserialize(deserializer).unwrap();

    let mut expected = BTreeMap::new();
    expected.insert("x".to_string(), 0);
    expected.insert("y".to_string(), 1);
    assert_eq!(expected, map);
}

#[test]
fn test_ser_timestamp() {
    let _guard = LOCK.run_concurrently();
    use bson::Timestamp;

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Foo {
        ts: Timestamp,
    }

    let foo = Foo {
        ts: Timestamp {
            time: 12,
            increment: 10,
        },
    };

    let x = serialize_to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! { "ts": Bson::Timestamp(Timestamp { time: 0x0000_000C, increment: 0x0000_000A }) }
    );

    let xfoo: Foo = deserialize_from_bson(x).unwrap();
    assert_eq!(xfoo, foo);
}

#[test]
fn test_de_timestamp() {
    let _guard = LOCK.run_concurrently();
    use bson::Timestamp;

    #[derive(Deserialize, Eq, PartialEq, Debug)]
    struct Foo {
        ts: Timestamp,
    }

    let foo: Foo = deserialize_from_bson(Bson::Document(doc! {
        "ts": Bson::Timestamp(Timestamp { time: 0x0000_000C, increment: 0x0000_000A }),
    }))
    .unwrap();

    assert_eq!(
        foo.ts,
        Timestamp {
            time: 12,
            increment: 10
        }
    );
}

#[test]
fn test_ser_regex() {
    let _guard = LOCK.run_concurrently();
    use bson::Regex;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        regex: Regex,
    }

    let regex = Regex {
        pattern: cstr!("12").into(),
        options: cstr!("01").into(),
    };

    let foo = Foo {
        regex: regex.clone(),
    };

    let x = serialize_to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! { "regex": Bson::RegularExpression(regex) }
    );

    let xfoo: Foo = deserialize_from_bson(x).unwrap();
    assert_eq!(xfoo, foo);
}

#[test]
fn test_de_regex() {
    let _guard = LOCK.run_concurrently();
    use bson::Regex;

    #[derive(Deserialize, PartialEq, Debug)]
    struct Foo {
        regex: Regex,
    }

    let regex = Regex {
        pattern: cstr!("12").into(),
        options: cstr!("01").into(),
    };

    let foo: Foo = deserialize_from_bson(Bson::Document(doc! {
        "regex": Bson::RegularExpression(regex.clone()),
    }))
    .unwrap();

    assert_eq!(foo.regex, regex);
}

#[test]
fn test_ser_code_with_scope() {
    let _guard = LOCK.run_concurrently();
    use bson::JavaScriptCodeWithScope;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        code_with_scope: JavaScriptCodeWithScope,
    }

    let code_with_scope = JavaScriptCodeWithScope {
        code: "x".into(),
        scope: doc! { "x": 12 },
    };

    let foo = Foo {
        code_with_scope: code_with_scope.clone(),
    };

    let x = serialize_to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! { "code_with_scope": Bson::JavaScriptCodeWithScope(code_with_scope) }
    );

    let xfoo: Foo = deserialize_from_bson(x).unwrap();
    assert_eq!(xfoo, foo);
}

#[test]
fn test_de_code_with_scope() {
    let _guard = LOCK.run_concurrently();
    use bson::JavaScriptCodeWithScope;

    #[derive(Deserialize, PartialEq, Debug)]
    struct Foo {
        code_with_scope: JavaScriptCodeWithScope,
    }

    let code_with_scope = JavaScriptCodeWithScope {
        code: "x".into(),
        scope: doc! { "x": 12 },
    };

    let foo: Foo = deserialize_from_bson(Bson::Document(doc! {
        "code_with_scope": Bson::JavaScriptCodeWithScope(code_with_scope.clone()),
    }))
    .unwrap();

    assert_eq!(foo.code_with_scope, code_with_scope);
}

#[test]
fn test_ser_datetime() {
    let _guard = LOCK.run_concurrently();
    use crate::DateTime;

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Foo {
        date: DateTime,
    }

    let now = DateTime::now();

    let foo = Foo { date: now };

    let x = serialize_to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! { "date": (Bson::DateTime(now)) }
    );

    let xfoo: Foo = deserialize_from_bson(x).unwrap();
    assert_eq!(xfoo, foo);
}

#[test]
fn test_binary_generic_roundtrip() {
    let _guard = LOCK.run_concurrently();
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub struct Foo {
        data: Bson,
    }

    let x = Foo {
        data: Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: b"12345abcde".to_vec(),
        }),
    };

    let b = serialize_to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::Generic, bytes: b"12345abcde".to_vec() })}
    );

    let f = deserialize_from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}

#[test]
fn test_binary_non_generic_roundtrip() {
    let _guard = LOCK.run_concurrently();
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub struct Foo {
        data: Bson,
    }

    let x = Foo {
        data: Bson::Binary(Binary {
            subtype: BinarySubtype::BinaryOld,
            bytes: b"12345abcde".to_vec(),
        }),
    };

    let b = serialize_to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::BinaryOld, bytes: b"12345abcde".to_vec() })}
    );

    let f = deserialize_from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}

#[test]
fn test_byte_vec() {
    let _guard = LOCK.run_concurrently();
    #[derive(Serialize, Debug, Eq, PartialEq)]
    pub struct AuthChallenge<'a> {
        #[serde(with = "serde_bytes")]
        pub challenge: &'a [u8],
    }

    let x = AuthChallenge {
        challenge: b"18762b98b7c34c25bf9dc3154e4a5ca3",
    };

    let b = serialize_to_bson(&x).unwrap();
    assert_eq!(
        b,
        Bson::Document(
            doc! { "challenge": (Bson::Binary(Binary { subtype: BinarySubtype::Generic, bytes: x.challenge.to_vec() }))}
        )
    );
}

#[test]
fn test_serde_bytes() {
    let _guard = LOCK.run_concurrently();
    #[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
    pub struct Foo {
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    }

    let x = Foo {
        data: b"12345abcde".to_vec(),
    };

    let b = serialize_to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::Generic, bytes: b"12345abcde".to_vec() })}
    );

    let f = deserialize_from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}

#[test]
fn test_serde_newtype_struct() {
    let _guard = LOCK.run_concurrently();
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Email(String);

    let email_1 = Email(String::from("bson@serde.rs"));
    let b = serialize_to_bson(&email_1).unwrap();
    assert_eq!(b, Bson::String(email_1.0));

    let s = String::from("root@localho.st");
    let de = Bson::String(s.clone());
    let email_2 = deserialize_from_bson::<Email>(de).unwrap();
    assert_eq!(email_2, Email(s));
}

#[test]
fn test_serde_tuple_struct() {
    let _guard = LOCK.run_concurrently();
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Name(String, String); // first, last

    let name_1 = Name(String::from("Graydon"), String::from("Hoare"));
    let b = serialize_to_bson(&name_1).unwrap();
    assert_eq!(b, bson!([name_1.0.clone(), name_1.1]));

    let (first, last) = (String::from("Donald"), String::from("Knuth"));
    let de = bson!([first.clone(), last.clone()]);
    let name_2 = deserialize_from_bson::<Name>(de).unwrap();
    assert_eq!(name_2, Name(first, last));
}

#[test]
fn test_serde_newtype_variant() {
    let _guard = LOCK.run_concurrently();
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "type", content = "value")]
    enum Number {
        Int(i64),
        Float(f64),
    }

    let n = 42;
    let num_1 = Number::Int(n);
    let b = serialize_to_bson(&num_1).unwrap();
    assert_eq!(b, bson!({ "type": "Int", "value": n }));

    let x = 1337.0;
    let de = bson!({ "type": "Float", "value": x });
    let num_2 = deserialize_from_bson::<Number>(de).unwrap();
    assert_eq!(num_2, Number::Float(x));
}

#[test]
fn test_serde_tuple_variant() {
    let _guard = LOCK.run_concurrently();
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Point {
        TwoDim(f64, f64),
        ThreeDim(f64, f64, f64),
    }

    #[allow(clippy::approx_constant)]
    let (x1, y1) = (3.14, -2.71);
    let p1 = Point::TwoDim(x1, y1);
    let b = serialize_to_bson(&p1).unwrap();
    assert_eq!(b, bson!({ "TwoDim": [x1, y1] }));

    let (x2, y2, z2) = (0.0, -13.37, 4.2);
    let de = bson!({ "ThreeDim": [x2, y2, z2] });
    let p2 = deserialize_from_bson::<Point>(de).unwrap();
    assert_eq!(p2, Point::ThreeDim(x2, y2, z2));
}

#[test]
fn test_ser_db_pointer() {
    let _guard = LOCK.run_concurrently();
    use bson::DbPointer;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        db_pointer: DbPointer,
    }

    let db_pointer = Bson::try_from(json!({
        "$dbPointer": {
            "$ref": "db.coll",
            "$id": { "$oid": "507f1f77bcf86cd799439011" },
        }
    }))
    .unwrap();

    let db_pointer = db_pointer.as_db_pointer().unwrap();

    let foo = Foo {
        db_pointer: db_pointer.clone(),
    };

    let x = serialize_to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! {"db_pointer": Bson::DbPointer(db_pointer.clone()) }
    );

    let xfoo: Foo = deserialize_from_bson(x).unwrap();
    assert_eq!(xfoo, foo);
}

#[test]
fn test_de_db_pointer() {
    let _guard = LOCK.run_concurrently();
    use bson::DbPointer;

    #[derive(Deserialize, PartialEq, Debug)]
    struct Foo {
        db_pointer: DbPointer,
    }

    let db_pointer = Bson::try_from(json!({
        "$dbPointer": {
            "$ref": "db.coll",
            "$id": { "$oid": "507f1f77bcf86cd799439011" },
        }
    }))
    .unwrap();
    let db_pointer = db_pointer.as_db_pointer().unwrap();

    let foo: Foo = deserialize_from_bson(Bson::Document(
        doc! {"db_pointer": Bson::DbPointer(db_pointer.clone())},
    ))
    .unwrap();

    assert_eq!(foo.db_pointer, db_pointer.clone());
}

#[test]
fn test_de_uuid_extjson_string() {
    let _guard = LOCK.run_concurrently();

    let uuid_bson_bytes =
        hex::decode("1D000000057800100000000473FFD26444B34C6990E8E7D1DFC035D400").unwrap();
    let uuid_document = Document::from_reader(uuid_bson_bytes.as_slice()).unwrap();
    let expected_uuid_bson = Bson::from_extended_document(uuid_document);

    let ext_json_uuid = "{\"x\" : { \"$uuid\" : \"73ffd264-44b3-4c69-90e8-e7d1dfc035d4\"}}";
    let actual_uuid_bson: Bson = serde_json::from_str(ext_json_uuid).unwrap();

    assert_eq!(actual_uuid_bson, expected_uuid_bson);
}

#[test]
fn test_de_date_extjson_number() {
    let _guard = LOCK.run_concurrently();

    let ext_json_canonical = r#"{ "$date": { "$numberLong": "1136239445000" } }"#;
    let expected_date_bson: Bson = serde_json::from_str(ext_json_canonical).unwrap();

    let ext_json_legacy_java = r#"{ "$date": 1136239445000 }"#;
    let actual_date_bson: Bson = serde_json::from_str(ext_json_legacy_java).unwrap();

    assert_eq!(actual_date_bson, expected_date_bson);
}

#[test]
fn test_de_oid_string() {
    let _guard = LOCK.run_concurrently();

    #[derive(Debug, Deserialize)]
    struct Foo {
        pub oid: ObjectId,
    }

    let foo: Foo = serde_json::from_str("{ \"oid\": \"507f1f77bcf86cd799439011\" }").unwrap();
    let oid = ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap();
    assert_eq!(foo.oid, oid);
}

#[test]
fn test_serialize_deserialize_unsigned_numbers() {
    let _guard = LOCK.run_concurrently();

    let num = 1;
    let json = format!("{{ \"num\": {} }}", num);
    let doc: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(doc.get_i32("num").unwrap(), num);

    let num = i32::MAX as u64 + 1;
    let json = format!("{{ \"num\": {} }}", num);
    let doc: Document = serde_json::from_str(&json).unwrap();
    assert_eq!(doc.get_i64("num").unwrap(), num as i64);

    let num = u64::MAX;
    let json = format!("{{ \"num\": {} }}", num);
    let doc_result: Result<Document, serde_json::Error> = serde_json::from_str(&json);
    assert!(doc_result.is_err());
}

#[test]
fn large_dates() {
    let _guard = LOCK.run_concurrently();

    let json = json!({ "d": { "$date": { "$numberLong": i64::MAX.to_string() } } });
    let d = serde_json::from_value::<Document>(json.clone()).unwrap();
    assert_eq!(d.get_datetime("d").unwrap(), &DateTime::MAX);
    let d: Bson = json.try_into().unwrap();
    assert_eq!(
        d.as_document().unwrap().get_datetime("d").unwrap(),
        &DateTime::MAX
    );

    let json = json!({ "d": { "$date": { "$numberLong": i64::MIN.to_string() } } });
    let d = serde_json::from_value::<Document>(json.clone()).unwrap();
    assert_eq!(d.get_datetime("d").unwrap(), &DateTime::MIN);
    let d: Bson = json.try_into().unwrap();
    assert_eq!(
        d.as_document().unwrap().get_datetime("d").unwrap(),
        &DateTime::MIN
    );
}

#[test]
fn fuzz_regression_00() {
    let buf: &[u8] = &[227, 0, 35, 4, 2, 0, 255, 255, 255, 127, 255, 255, 255, 47];
    let _ = crate::deserialize_from_slice::<Document>(buf);
}

#[cfg(feature = "serde_path_to_error")]
mod serde_path_to_error {
    use super::*;

    #[derive(Serialize, Deserialize, Debug)]
    struct Foo {
        one: Bar,
        two: Bar,
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct Bar {
        value: u64,
    }

    #[test]
    fn de() {
        let src = doc! {
            "one": {
                "value": 42,
            },
            "two": {
                "value": "hello",
            },
        };
        let result: Result<Foo, _> = crate::deserialize_from_document(src);
        assert!(result.is_err());
        let path = result.unwrap_err().path.unwrap();
        assert_eq!(path.to_string(), "two.value");
    }

    #[test]
    fn de_raw() {
        let src = rawdoc! {
            "one": {
                "value": 42,
            },
            "two": {
                "value": "hello",
            },
        }
        .into_bytes();
        let result: Result<Foo, _> = crate::deserialize_from_slice(&src);
        assert!(result.is_err());
        let path = result.unwrap_err().path.unwrap();
        assert_eq!(path.to_string(), "two.value");
    }

    #[test]
    fn ser() {
        let src = Foo {
            one: Bar { value: 42 },
            two: Bar { value: u64::MAX },
        };
        let result = crate::serialize_to_bson(&src);
        assert!(result.is_err());
        let path = result.unwrap_err().path.unwrap();
        assert_eq!(path.to_string(), "two.value");
    }

    #[test]
    fn ser_raw() {
        let src = Foo {
            one: Bar { value: 42 },
            two: Bar { value: u64::MAX },
        };
        let result = crate::serialize_to_vec(&src);
        assert!(result.is_err());
        let path = result.unwrap_err().path.unwrap();
        assert_eq!(path.to_string(), "two.value");
    }
}
