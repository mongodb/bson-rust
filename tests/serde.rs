#![allow(clippy::blacklisted_name)]

use bson::{bson, doc, spec::BinarySubtype, Binary, Bson, Decoder, Encoder};
use serde::{Deserialize, Serialize};
use serde_derive::{Deserialize, Serialize};
use serde_json::json;

use std::collections::BTreeMap;

#[test]
fn test_ser_vec() {
    let vec = vec![1, 2, 3];

    let encoder = Encoder::new();
    let result = vec.serialize(encoder).unwrap();

    let expected = bson!([1, 2, 3]);
    assert_eq!(expected, result);
}

#[test]
fn test_ser_map() {
    let mut map = BTreeMap::new();
    map.insert("x", 0);
    map.insert("y", 1);

    let encoder = Encoder::new();
    let result = map.serialize(encoder).unwrap();

    let expected = bson!({ "x": 0, "y": 1 });
    assert_eq!(expected, result);
}

#[test]
fn test_de_vec() {
    let bson = bson!([1, 2, 3]);

    let decoder = Decoder::new(bson);
    let vec = Vec::<i32>::deserialize(decoder).unwrap();

    let expected = vec![1, 2, 3];
    assert_eq!(expected, vec);
}

#[test]
fn test_de_map() {
    let bson = bson!({ "x": 0, "y": 1 });

    let decoder = Decoder::new(bson);
    let map = BTreeMap::<String, i32>::deserialize(decoder).unwrap();

    let mut expected = BTreeMap::new();
    expected.insert("x".to_string(), 0);
    expected.insert("y".to_string(), 1);
    assert_eq!(expected, map);
}

#[test]
fn test_ser_timestamp() {
    use bson::TimeStamp;

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Foo {
        ts: TimeStamp,
    }

    let foo = Foo {
        ts: TimeStamp {
            time: 12,
            increment: 10,
        },
    };

    let x = bson::to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! { "ts": Bson::TimeStamp(TimeStamp { time: 0x0000_000C, increment: 0x0000_000A }) }
    );

    let xfoo: Foo = bson::from_bson(x).unwrap();
    assert_eq!(xfoo, foo);
}

#[test]
fn test_de_timestamp() {
    use bson::TimeStamp;

    #[derive(Deserialize, Eq, PartialEq, Debug)]
    struct Foo {
        ts: TimeStamp,
    }

    let foo: Foo = bson::from_bson(Bson::Document(doc! {
        "ts": Bson::TimeStamp(TimeStamp { time: 0x0000_000C, increment: 0x0000_000A }),
    }))
    .unwrap();

    assert_eq!(
        foo.ts,
        TimeStamp {
            time: 12,
            increment: 10
        }
    );
}

#[test]
fn test_ser_regex() {
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

    let x = bson::to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! { "regex": Bson::Regex(regex) }
    );

    let xfoo: Foo = bson::from_bson(x).unwrap();
    assert_eq!(xfoo, foo);
}

#[test]
fn test_de_regex() {
    use bson::Regex;

    #[derive(Deserialize, PartialEq, Debug)]
    struct Foo {
        regex: Regex,
    }

    let regex = Regex {
        pattern: "12".into(),
        options: "01".into(),
    };

    let foo: Foo = bson::from_bson(Bson::Document(doc! {
        "regex": Bson::Regex(regex.clone()),
    }))
    .unwrap();

    assert_eq!(foo.regex, regex);
}

#[test]
fn test_ser_code_with_scope() {
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

    let x = bson::to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! { "code_with_scope": Bson::JavaScriptCodeWithScope(code_with_scope) }
    );

    let xfoo: Foo = bson::from_bson(x).unwrap();
    assert_eq!(xfoo, foo);
}

#[test]
fn test_de_code_with_scope() {
    use bson::JavaScriptCodeWithScope;

    #[derive(Deserialize, PartialEq, Debug)]
    struct Foo {
        code_with_scope: JavaScriptCodeWithScope,
    }

    let code_with_scope = JavaScriptCodeWithScope {
        code: "x".into(),
        scope: doc! { "x": 12 },
    };

    let foo: Foo = bson::from_bson(Bson::Document(doc! {
        "code_with_scope": Bson::JavaScriptCodeWithScope(code_with_scope.clone()),
    }))
    .unwrap();

    assert_eq!(foo.code_with_scope, code_with_scope);
}

#[test]
fn test_ser_datetime() {
    use bson::UtcDateTime;
    use chrono::{Timelike, Utc};

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Foo {
        date: UtcDateTime,
    }

    let now = Utc::now();
    // FIXME: Due to BSON's datetime precision
    let now = now
        .with_nanosecond(now.nanosecond() / 1_000_000 * 1_000_000)
        .unwrap();

    let foo = Foo {
        date: From::from(now),
    };

    let x = bson::to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! { "date": (Bson::UtcDatetime(now)) }
    );

    let xfoo: Foo = bson::from_bson(x).unwrap();
    assert_eq!(xfoo, foo);
}

#[test]
fn test_compat_u2f() {
    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Foo {
        #[serde(with = "bson::compat::u2f")]
        x: u32,
    }

    let foo = Foo { x: 20 };
    let b = bson::to_bson(&foo).unwrap();
    assert_eq!(b, Bson::Document(doc! { "x": (Bson::FloatingPoint(20.0)) }));

    let de_foo = bson::from_bson::<Foo>(b).unwrap();
    assert_eq!(de_foo, foo);
}

#[test]
fn test_binary_generic_roundtrip() {
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

    let b = bson::to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::Generic, bytes: b"12345abcde".to_vec() })}
    );

    let f = bson::from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}

#[test]
fn test_binary_non_generic_roundtrip() {
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

    let b = bson::to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::BinaryOld, bytes: b"12345abcde".to_vec() })}
    );

    let f = bson::from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}

#[test]
fn test_binary_helper_generic_roundtrip() {
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

    let b = bson::to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::Generic, bytes: b"12345abcde".to_vec() })}
    );

    let f = bson::from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}

#[test]
fn test_binary_helper_non_generic_roundtrip() {
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

    let b = bson::to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::BinaryOld, bytes: b"12345abcde".to_vec() })}
    );

    let f = bson::from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}

#[test]
fn test_byte_vec() {
    #[derive(Serialize, Debug, Eq, PartialEq)]
    pub struct AuthChallenge<'a> {
        #[serde(with = "serde_bytes")]
        pub challenge: &'a [u8],
    }

    let x = AuthChallenge {
        challenge: b"18762b98b7c34c25bf9dc3154e4a5ca3",
    };

    let b = bson::to_bson(&x).unwrap();
    assert_eq!(
        b,
        Bson::Document(
            doc! { "challenge": (Bson::Binary(Binary { subtype: bson::spec::BinarySubtype::Generic, bytes: x.challenge.to_vec() }))}
        )
    );

    // let mut buf = Vec::new();
    // bson::encode_document(&mut buf, b.as_document().unwrap()).unwrap();

    // let xb = bson::decode_document(&mut Cursor::new(buf)).unwrap();
    // assert_eq!(b.as_document().unwrap(), &xb);
}

#[test]
fn test_serde_bytes() {
    #[derive(Serialize, Deserialize, Debug, Eq, PartialEq)]
    pub struct Foo {
        #[serde(with = "serde_bytes")]
        data: Vec<u8>,
    }

    let x = Foo {
        data: b"12345abcde".to_vec(),
    };

    let b = bson::to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: bson::spec::BinarySubtype::Generic, bytes: b"12345abcde".to_vec() })}
    );

    let f = bson::from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}

#[test]
fn test_serde_newtype_struct() {
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Email(String);

    let email_1 = Email(String::from("bson@serde.rs"));
    let b = bson::to_bson(&email_1).unwrap();
    assert_eq!(b, Bson::String(email_1.0));

    let s = String::from("root@localho.st");
    let de = Bson::String(s.clone());
    let email_2 = bson::from_bson::<Email>(de).unwrap();
    assert_eq!(email_2, Email(s));
}

#[test]
fn test_serde_tuple_struct() {
    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Name(String, String); // first, last

    let name_1 = Name(String::from("Graydon"), String::from("Hoare"));
    let b = bson::to_bson(&name_1).unwrap();
    assert_eq!(b, bson!([name_1.0.clone(), name_1.1]));

    let (first, last) = (String::from("Donald"), String::from("Knuth"));
    let de = bson!([first.clone(), last.clone()]);
    let name_2 = bson::from_bson::<Name>(de).unwrap();
    assert_eq!(name_2, Name(first, last));
}

#[test]
fn test_serde_newtype_variant() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "type", content = "value")]
    enum Number {
        Int(i64),
        Float(f64),
    }

    let n = 42;
    let num_1 = Number::Int(n);
    let b = bson::to_bson(&num_1).unwrap();
    assert_eq!(b, bson!({ "type": "Int", "value": n }));

    let x = 1337.0;
    let de = bson!({ "type": "Float", "value": x });
    let num_2 = bson::from_bson::<Number>(de).unwrap();
    assert_eq!(num_2, Number::Float(x));
}

#[test]
fn test_serde_tuple_variant() {
    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Point {
        TwoDim(f64, f64),
        ThreeDim(f64, f64, f64),
    }

    #[allow(clippy::approx_constant)]
    let (x1, y1) = (3.14, -2.71);
    let p1 = Point::TwoDim(x1, y1);
    let b = bson::to_bson(&p1).unwrap();
    assert_eq!(b, bson!({ "TwoDim": [x1, y1] }));

    let (x2, y2, z2) = (0.0, -13.37, 4.2);
    let de = bson!({ "ThreeDim": [x2, y2, z2] });
    let p2 = bson::from_bson::<Point>(de).unwrap();
    assert_eq!(p2, Point::ThreeDim(x2, y2, z2));
}

#[test]
fn test_ser_db_pointer() {
    use bson::DbPointer;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        db_pointer: DbPointer,
    }

    let db_pointer = Bson::from(json!({
        "$dbPointer": {
            "$ref": "db.coll",
            "$id": { "$oid": "507f1f77bcf86cd799439011" },
        }
    }));

    let db_pointer = db_pointer.as_db_pointer().unwrap();

    let foo = Foo {
        db_pointer: db_pointer.clone(),
    };

    let x = bson::to_bson(&foo).unwrap();
    assert_eq!(
        x.as_document().unwrap(),
        &doc! {"db_pointer": Bson::DbPointer(db_pointer.clone()) }
    );

    let xfoo: Foo = bson::from_bson(x).unwrap();
    assert_eq!(xfoo, foo);
}

#[test]
fn test_de_db_pointer() {
    use bson::DbPointer;

    #[derive(Deserialize, PartialEq, Debug)]
    struct Foo {
        db_pointer: DbPointer,
    }

    let db_pointer = Bson::from(json!({
        "$dbPointer": {
            "$ref": "db.coll",
            "$id": { "$oid": "507f1f77bcf86cd799439011" },
        }
    }));
    let db_pointer = db_pointer.as_db_pointer().unwrap();

    let foo: Foo = bson::from_bson(Bson::Document(
        doc! {"db_pointer": Bson::DbPointer(db_pointer.clone())},
    ))
    .unwrap();

    assert_eq!(foo.db_pointer, db_pointer.clone());
}
