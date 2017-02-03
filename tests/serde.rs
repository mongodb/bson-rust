#[macro_use]
extern crate bson;
use bson::{Encoder, Decoder};

extern crate serde;
use serde::{Serialize, Deserialize};

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
