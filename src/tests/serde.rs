#![allow(clippy::blacklisted_name)]

use crate::{
    bson,
    compat::u2f,
    doc,
    from_bson,
    from_document,
    oid::ObjectId,
    serde_helpers,
    serde_helpers::{
        bson_datetime_as_iso_string,
        chrono_datetime_as_bson_datetime,
        iso_string_as_bson_datetime,
        timestamp_as_u32,
        u32_as_timestamp,
        uuid_as_binary,
    },
    spec::BinarySubtype,
    tests::LOCK,
    to_bson,
    to_document,
    Binary,
    Bson,
    DateTime,
    Deserializer,
    Document,
    Serializer,
    Timestamp,
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use std::{collections::BTreeMap, convert::TryFrom, str::FromStr};

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

    let x = to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! { "ts": Bson::Timestamp(Timestamp { time: 0x0000_000C, increment: 0x0000_000A }) }
    );

    let xfoo: Foo = from_bson(x).unwrap();
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

    let foo: Foo = from_bson(Bson::Document(doc! {
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
        pattern: "12".into(),
        options: "01".into(),
    };

    let foo = Foo {
        regex: regex.clone(),
    };

    let x = to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! { "regex": Bson::RegularExpression(regex) }
    );

    let xfoo: Foo = from_bson(x).unwrap();
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
        pattern: "12".into(),
        options: "01".into(),
    };

    let foo: Foo = from_bson(Bson::Document(doc! {
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

    let x = to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! { "code_with_scope": Bson::JavaScriptCodeWithScope(code_with_scope) }
    );

    let xfoo: Foo = from_bson(x).unwrap();
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

    let foo: Foo = from_bson(Bson::Document(doc! {
        "code_with_scope": Bson::JavaScriptCodeWithScope(code_with_scope.clone()),
    }))
    .unwrap();

    assert_eq!(foo.code_with_scope, code_with_scope);
}

#[test]
fn test_ser_datetime() {
    let _guard = LOCK.run_concurrently();
    use bson::DateTime;
    use chrono::{Timelike, Utc};

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Foo {
        date: DateTime,
    }

    let now = Utc::now();
    // FIXME: Due to BSON's datetime precision
    let now = now
        .with_nanosecond(now.nanosecond() / 1_000_000 * 1_000_000)
        .unwrap();

    let foo = Foo {
        date: From::from(now),
    };

    let x = to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! { "date": (Bson::DateTime(now)) }
    );

    let xfoo: Foo = from_bson(x).unwrap();
    assert_eq!(xfoo, foo);
}

#[test]
fn test_compat_u2f() {
    let _guard = LOCK.run_concurrently();
    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Foo {
        #[serde(with = "u2f")]
        x: u32,
    }

    let foo = Foo { x: 20 };
    let b = to_bson(&foo).unwrap();
    assert_eq!(b, Bson::Document(doc! { "x": (Bson::Double(20.0)) }));

    let de_foo = from_bson::<Foo>(b).unwrap();
    assert_eq!(de_foo, foo);
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

    let b = to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::Generic, bytes: b"12345abcde".to_vec() })}
    );

    let f = from_bson::<Foo>(b).unwrap();
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

    let b = to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::BinaryOld, bytes: b"12345abcde".to_vec() })}
    );

    let f = from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}

#[test]
fn test_binary_helper_generic_roundtrip() {
    let _guard = LOCK.run_concurrently();
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub struct Foo {
        data: Binary,
    }

    let x = Foo {
        data: Binary {
            subtype: BinarySubtype::Generic,
            bytes: b"12345abcde".to_vec(),
        },
    };

    let b = to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::Generic, bytes: b"12345abcde".to_vec() })}
    );

    let f = from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}

#[test]
fn test_binary_helper_non_generic_roundtrip() {
    let _guard = LOCK.run_concurrently();
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub struct Foo {
        data: Binary,
    }

    let x = Foo {
        data: Binary {
            subtype: BinarySubtype::BinaryOld,
            bytes: b"12345abcde".to_vec(),
        },
    };

    let b = to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::BinaryOld, bytes: b"12345abcde".to_vec() })}
    );

    let f = from_bson::<Foo>(b).unwrap();
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

    let b = to_bson(&x).unwrap();
    assert_eq!(
        b,
        Bson::Document(
            doc! { "challenge": (Bson::Binary(Binary { subtype: BinarySubtype::Generic, bytes: x.challenge.to_vec() }))}
        )
    );

    // let mut buf = Vec::new();
    // b.as_document().unwrap().to_writer(&mut buf).unwrap();

    // let xb = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    // assert_eq!(b.as_document().unwrap(), &xb);
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

    let b = to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::Generic, bytes: b"12345abcde".to_vec() })}
    );

    let f = from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}

#[test]
fn test_serde_newtype_struct() {
    let _guard = LOCK.run_concurrently();
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Email(String);

    let email_1 = Email(String::from("bson@serde.rs"));
    let b = to_bson(&email_1).unwrap();
    assert_eq!(b, Bson::String(email_1.0));

    let s = String::from("root@localho.st");
    let de = Bson::String(s.clone());
    let email_2 = from_bson::<Email>(de).unwrap();
    assert_eq!(email_2, Email(s));
}

#[test]
fn test_serde_tuple_struct() {
    let _guard = LOCK.run_concurrently();
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Name(String, String); // first, last

    let name_1 = Name(String::from("Graydon"), String::from("Hoare"));
    let b = to_bson(&name_1).unwrap();
    assert_eq!(b, bson!([name_1.0.clone(), name_1.1]));

    let (first, last) = (String::from("Donald"), String::from("Knuth"));
    let de = bson!([first.clone(), last.clone()]);
    let name_2 = from_bson::<Name>(de).unwrap();
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
    let b = to_bson(&num_1).unwrap();
    assert_eq!(b, bson!({ "type": "Int", "value": n }));

    let x = 1337.0;
    let de = bson!({ "type": "Float", "value": x });
    let num_2 = from_bson::<Number>(de).unwrap();
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
    let b = to_bson(&p1).unwrap();
    assert_eq!(b, bson!({ "TwoDim": [x1, y1] }));

    let (x2, y2, z2) = (0.0, -13.37, 4.2);
    let de = bson!({ "ThreeDim": [x2, y2, z2] });
    let p2 = from_bson::<Point>(de).unwrap();
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

    let x = to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! {"db_pointer": Bson::DbPointer(db_pointer.clone()) }
    );

    let xfoo: Foo = from_bson(x).unwrap();
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

    let foo: Foo = from_bson(Bson::Document(
        doc! {"db_pointer": Bson::DbPointer(db_pointer.clone())},
    ))
    .unwrap();

    assert_eq!(foo.db_pointer, db_pointer.clone());
}

#[test]
fn test_de_oid_string() {
    let _guard = LOCK.run_concurrently();

    #[derive(Debug, Deserialize)]
    struct Foo {
        pub oid: ObjectId,
    }

    let foo: Foo = serde_json::from_str("{ \"oid\": \"507f1f77bcf86cd799439011\" }").unwrap();
    let oid = ObjectId::with_string("507f1f77bcf86cd799439011").unwrap();
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
fn test_unsigned_helpers() {
    let _guard = LOCK.run_concurrently();

    #[derive(Serialize)]
    struct A {
        #[serde(serialize_with = "serde_helpers::serialize_u32_as_i32")]
        num_1: u32,
        #[serde(serialize_with = "serde_helpers::serialize_u64_as_i32")]
        num_2: u64,
    }

    let a = A { num_1: 1, num_2: 2 };
    let doc = to_document(&a).unwrap();
    assert!(doc.get_i32("num_1").unwrap() == 1);
    assert!(doc.get_i32("num_2").unwrap() == 2);

    let a = A {
        num_1: u32::MAX,
        num_2: 1,
    };
    let doc_result = to_document(&a);
    assert!(doc_result.is_err());

    let a = A {
        num_1: 1,
        num_2: u64::MAX,
    };
    let doc_result = to_document(&a);
    assert!(doc_result.is_err());

    #[derive(Serialize)]
    struct B {
        #[serde(serialize_with = "serde_helpers::serialize_u32_as_i64")]
        num_1: u32,
        #[serde(serialize_with = "serde_helpers::serialize_u64_as_i64")]
        num_2: u64,
    }

    let b = B {
        num_1: u32::MAX,
        num_2: i64::MAX as u64,
    };
    let doc = to_document(&b).unwrap();
    assert!(doc.get_i64("num_1").unwrap() == u32::MAX as i64);
    assert!(doc.get_i64("num_2").unwrap() == i64::MAX);

    let b = B {
        num_1: 1,
        num_2: i64::MAX as u64 + 1,
    };
    let doc_result = to_document(&b);
    assert!(doc_result.is_err());
}

#[test]
fn test_datetime_helpers() {
    let _guard = LOCK.run_concurrently();

    #[derive(Deserialize, Serialize)]
    struct A {
        #[serde(with = "bson_datetime_as_iso_string")]
        pub date: DateTime,
    }

    let iso = "1996-12-20 00:39:57 UTC";
    let date = chrono::DateTime::from_str(iso).unwrap();
    let a = A { date: date.into() };
    let doc = to_document(&a).unwrap();
    assert_eq!(doc.get_str("date").unwrap(), iso);
    let a: A = from_document(doc).unwrap();
    assert_eq!(a.date, date.into());

    #[derive(Deserialize, Serialize)]
    struct B {
        #[serde(with = "chrono_datetime_as_bson_datetime")]
        pub date: chrono::DateTime<chrono::Utc>,
    }

    let date = r#"
    {
        "date": {
                "$date": {
                    "$numberLong": "1591700287095"
                }
        }
    }"#;
    let json: Value = serde_json::from_str(&date).unwrap();
    let b: B = serde_json::from_value(json).unwrap();
    let expected: chrono::DateTime<chrono::Utc> =
        chrono::DateTime::from_str("2020-06-09 10:58:07.095 UTC").unwrap();
    assert_eq!(b.date, expected);
    let doc = to_document(&b).unwrap();
    assert_eq!(doc.get_datetime("date").unwrap(), &expected);
    let b: B = from_document(doc).unwrap();
    assert_eq!(b.date, expected);

    #[derive(Deserialize, Serialize)]
    struct C {
        #[serde(with = "iso_string_as_bson_datetime")]
        pub date: String,
    }

    let date = "2020-06-09 10:58:07.095 UTC";
    let c = C {
        date: date.to_string(),
    };
    let doc = to_document(&c).unwrap();
    assert!(doc.get_datetime("date").is_ok());
    let c: C = from_document(doc).unwrap();
    assert_eq!(c.date.as_str(), date);
}

#[test]
fn test_oid_helpers() {
    let _guard = LOCK.run_concurrently();

    #[derive(Serialize)]
    struct A {
        #[serde(serialize_with = "serde_helpers::serialize_hex_string_as_object_id")]
        oid: String,
    }

    let oid = ObjectId::new();
    let a = A {
        oid: oid.to_string(),
    };
    let doc = to_document(&a).unwrap();
    assert_eq!(doc.get_object_id("oid").unwrap(), oid);
}

#[test]
fn test_uuid_helpers() {
    let _guard = LOCK.run_concurrently();

    #[derive(Serialize, Deserialize)]
    struct A {
        #[serde(with = "uuid_as_binary")]
        uuid: Uuid,
    }

    let uuid = Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap();
    let a = A { uuid };
    let doc = to_document(&a).unwrap();
    match doc.get("uuid").unwrap() {
        Bson::Binary(bin) => {
            assert_eq!(bin.subtype, BinarySubtype::Uuid);
            assert_eq!(bin.bytes, uuid.as_bytes());
        }
        _ => panic!("expected Bson::Binary"),
    }
    let a: A = from_document(doc).unwrap();
    assert_eq!(a.uuid, uuid);
}

#[test]
fn test_timestamp_helpers() {
    let _guard = LOCK.run_concurrently();

    #[derive(Deserialize, Serialize)]
    struct A {
        #[serde(with = "u32_as_timestamp")]
        pub time: u32,
    }

    let time = 12345;
    let a = A { time };
    let doc = to_document(&a).unwrap();
    let timestamp = doc.get_timestamp("time").unwrap();
    assert_eq!(timestamp.time, time);
    assert_eq!(timestamp.increment, 0);
    let a: A = from_document(doc).unwrap();
    assert_eq!(a.time, time);

    #[derive(Deserialize, Serialize)]
    struct B {
        #[serde(with = "timestamp_as_u32")]
        pub timestamp: Timestamp,
    }

    let time = 12345;
    let timestamp = Timestamp { time, increment: 0 };
    let b = B { timestamp };
    let val = serde_json::to_value(b).unwrap();
    assert_eq!(val["timestamp"], time);
    let b: B = serde_json::from_value(val).unwrap();
    assert_eq!(b.timestamp, timestamp);

    let timestamp = Timestamp {
        time: 12334,
        increment: 1,
    };
    let b = B { timestamp };
    assert!(serde_json::to_value(b).is_err());
}
