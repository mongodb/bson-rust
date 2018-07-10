#[macro_use]
extern crate bson;
extern crate chrono;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_bytes;

use bson::{Bson, Decoder, Encoder};
use serde::{Deserialize, Serialize};

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

    let foo = Foo { ts: TimeStamp { t: 12, i: 10 } };

    let x = bson::to_bson(&foo).unwrap();
    assert_eq!(x.as_document().unwrap(), &doc! { "ts": Bson::TimeStamp(0x0000_000C_0000_000A) });

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
        "ts": Bson::TimeStamp(0x0000_000C_0000_000A),
    })).unwrap();

    assert_eq!(foo.ts, TimeStamp { t: 12, i: 10 });
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
    let now = now.with_nanosecond(now.nanosecond() / 1000000 * 1000000).unwrap();

    let foo = Foo { date: From::from(now) };

    let x = bson::to_bson(&foo).unwrap();
    assert_eq!(x.as_document().unwrap(), &doc! { "date": (Bson::UtcDatetime(now)) });

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
fn test_byte_vec() {
    #[derive(Serialize, Debug, Eq, PartialEq)]
    pub struct AuthChallenge<'a> {
        #[serde(with = "serde_bytes")]
        pub challenge: &'a [u8],
    }

    let x = AuthChallenge { challenge: b"18762b98b7c34c25bf9dc3154e4a5ca3", };

    let b = bson::to_bson(&x).unwrap();
    assert_eq!(b, Bson::Document(doc! { "challenge": (Bson::Binary(bson::spec::BinarySubtype::Generic, x.challenge.to_vec()))}));

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

    let x = Foo { data: b"12345abcde".to_vec(), };

    let b = bson::to_bson(&x).unwrap();
    assert_eq!(b.as_document().unwrap(),
               &doc! {"data": Bson::Binary(bson::spec::BinarySubtype::Generic, b"12345abcde".to_vec())});

    let f = bson::from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}
