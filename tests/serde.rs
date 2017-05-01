#[macro_use]
extern crate bson;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate chrono;

use serde::{Serialize, Deserialize};
use bson::{Encoder, Decoder, Bson};

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

    let expected = bson!({ "x" => 0, "y" => 1 });
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
    let bson = bson!({ "x" => 0, "y" => 1 });

    let decoder = Decoder::new(bson);
    let map = BTreeMap::<String, i32>::deserialize(decoder).unwrap();

    let mut expected = BTreeMap::new();
    expected.insert("x".to_string(), 0);
    expected.insert("y".to_string(), 1);
    assert_eq!(expected, map);
}

#[test]
fn test_ser_datetime() {
    use chrono::{UTC, Timelike};
    use bson::UtcDateTime;

    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Foo {
        date: UtcDateTime,
    }

    let now = UTC::now();
    // FIXME: Due to BSON's datetime precision
    let now = now.with_nanosecond(now.nanosecond() / 1000000 * 1000000).unwrap();

    let foo = Foo { date: From::from(now) };

    let x = bson::to_bson(&foo).unwrap();
    assert_eq!(x.as_document().unwrap(), &doc! { "date" => (Bson::UtcDatetime(now)) });

    let xfoo: Foo = bson::from_bson(x).unwrap();
    assert_eq!(xfoo, foo);
}
