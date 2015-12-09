use bson::{Bson, to_bson, from_bson};
use bson::oid::ObjectId;
use std::collections::BTreeMap;

#[test]
fn floating_point() {
    let obj = Bson::FloatingPoint(240.5);
    let f: f64 = from_bson(obj.clone()).unwrap();
    assert_eq!(f, 240.5);

    let deser: Bson = to_bson(&f).unwrap();
    assert_eq!(obj, deser);
}

#[test]
fn string() {
    let obj = Bson::String("avocado".to_owned());
    let s: String = from_bson(obj.clone()).unwrap();
    assert_eq!(s, "avocado");

    let deser: Bson = to_bson(&s).unwrap();
    assert_eq!(obj, deser);
}

#[test]
fn arr() {
    let obj = Bson::Array(vec![Bson::I32(0), Bson::I32(1), Bson::I32(2), Bson::I32(3)]);
    let arr: Vec<i32> = from_bson(obj.clone().clone()).unwrap();
    assert_eq!(arr, vec![0i32, 1i32, 2i32, 3i32]);

    let deser: Bson = to_bson(&arr).unwrap();
    assert_eq!(deser, obj);
}

#[test]
fn boolean() {
    let obj = Bson::Boolean(true);
    let b: bool = from_bson(obj.clone()).unwrap();
    assert_eq!(b, true);

    let deser: Bson = to_bson(&b).unwrap();
    assert_eq!(deser, obj);
}

#[test]
fn int32() {
    let obj = Bson::I32(101);
    let i: i32 = from_bson(obj.clone()).unwrap();

    assert_eq!(i, 101);

    let deser: Bson = to_bson(&i).unwrap();
    assert_eq!(deser, obj);
}

#[test]
fn int64() {
    let obj = Bson::I64(101);
    let i: i64 = from_bson(obj.clone()).unwrap();

    assert_eq!(i, 101);

    let deser: Bson = to_bson(&i).unwrap();
    assert_eq!(deser, obj);
}

#[test]
fn oid() {
    let oid = ObjectId::new().unwrap();
    let obj = Bson::ObjectId(oid.clone());
    let s: BTreeMap<String, String> = from_bson(obj.clone()).unwrap();

    let mut expected = BTreeMap::new();
    expected.insert("$oid".to_owned(), oid.to_string());
    assert_eq!(s, expected);

    let deser: Bson = to_bson(&s).unwrap();
    assert_eq!(deser, obj);
}
