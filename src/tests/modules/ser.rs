use std::{collections::BTreeMap, u16, u32, u64, u8};

use assert_matches::assert_matches;

#[cfg(feature = "decimal128")]
use crate::decimal128::Decimal128;
use crate::{from_bson, oid::ObjectId, ser, tests::LOCK, to_bson, Bson};

#[test]
#[allow(clippy::float_cmp)]
fn floating_point() {
    let _guard = LOCK.run_concurrently();
    let obj = Bson::Double(240.5);
    let f: f64 = from_bson(obj.clone()).unwrap();
    assert_eq!(f, 240.5);

    let deser: Bson = to_bson(&f).unwrap();
    assert_eq!(obj, deser);
}

#[test]
fn string() {
    let _guard = LOCK.run_concurrently();
    let obj = Bson::String("avocado".to_owned());
    let s: String = from_bson(obj.clone()).unwrap();
    assert_eq!(s, "avocado");

    let deser: Bson = to_bson(&s).unwrap();
    assert_eq!(obj, deser);
}

#[test]
fn arr() {
    let _guard = LOCK.run_concurrently();
    let obj = Bson::Array(vec![
        Bson::Int32(0),
        Bson::Int32(1),
        Bson::Int32(2),
        Bson::Int32(3),
    ]);
    let arr: Vec<i32> = from_bson(obj.clone()).unwrap();
    assert_eq!(arr, vec![0i32, 1i32, 2i32, 3i32]);

    let deser: Bson = to_bson(&arr).unwrap();
    assert_eq!(deser, obj);
}

#[test]
fn boolean() {
    let _guard = LOCK.run_concurrently();
    let obj = Bson::Boolean(true);
    let b: bool = from_bson(obj.clone()).unwrap();
    assert_eq!(b, true);

    let deser: Bson = to_bson(&b).unwrap();
    assert_eq!(deser, obj);
}

#[test]
fn int32() {
    let _guard = LOCK.run_concurrently();
    let obj = Bson::Int32(101);
    let i: i32 = from_bson(obj.clone()).unwrap();

    assert_eq!(i, 101);

    let deser: Bson = to_bson(&i).unwrap();
    assert_eq!(deser, obj);
}

#[cfg(feature = "decimal128")]
#[test]
fn dec128() {
    let _guard = LOCK.run_concurrently();
    let d128 = Decimal128::from_str("1.05E+3");
    let obj = Bson::Decimal128(d128.clone());
    let ser: Decimal128 = from_bson(obj.clone()).unwrap();
    assert_eq!(ser, d128);

    let deser: Bson = to_bson(&ser).unwrap();
    assert_eq!(deser, obj);
}

#[test]
#[cfg(not(feature = "u2i"))]
fn uint8() {
    let _guard = LOCK.run_concurrently();
    let obj_min: ser::Result<Bson> = to_bson(&u8::MIN);
    assert_matches!(obj_min, Err(ser::Error::UnsupportedUnsignedType));
}

#[test]
#[cfg(feature = "u2i")]
fn uint8_u2i() {
    let _guard = LOCK.run_concurrently();
    let obj: Bson = to_bson(&u8::MIN).unwrap();
    let deser: u8 = from_bson(obj).unwrap();
    assert_eq!(deser, u8::MIN);

    let obj_max: Bson = to_bson(&u8::MAX).unwrap();
    let deser_max: u8 = from_bson(obj_max).unwrap();
    assert_eq!(deser_max, u8::MAX);
}

#[test]
#[cfg(not(feature = "u2i"))]
fn uint16() {
    let _guard = LOCK.run_concurrently();
    let obj_min: ser::Result<Bson> = to_bson(&u16::MIN);
    assert_matches!(obj_min, Err(ser::Error::UnsupportedUnsignedType));
}

#[test]
#[cfg(feature = "u2i")]
fn uint16_u2i() {
    let _guard = LOCK.run_concurrently();
    let obj: Bson = to_bson(&u16::MIN).unwrap();
    let deser: u16 = from_bson(obj).unwrap();
    assert_eq!(deser, u16::MIN);

    let obj_max: Bson = to_bson(&u16::MAX).unwrap();
    let deser_max: u16 = from_bson(obj_max).unwrap();
    assert_eq!(deser_max, u16::MAX);
}

#[test]
#[cfg(not(feature = "u2i"))]
fn uint32() {
    let _guard = LOCK.run_concurrently();
    let obj_min: ser::Result<Bson> = to_bson(&u32::MIN);
    assert_matches!(obj_min, Err(ser::Error::UnsupportedUnsignedType));
}

#[test]
#[cfg(feature = "u2i")]
fn uint32_u2i() {
    let _guard = LOCK.run_concurrently();
    let obj_min: Bson = to_bson(&u32::MIN).unwrap();
    let deser_min: u32 = from_bson(obj_min).unwrap();
    assert_eq!(deser_min, u32::MIN);

    let obj_max: Bson = to_bson(&u32::MAX).unwrap();
    let deser_max: u32 = from_bson(obj_max).unwrap();
    assert_eq!(deser_max, u32::MAX);
}

#[test]
#[cfg(not(feature = "u2i"))]
fn uint64() {
    let _guard = LOCK.run_concurrently();
    let obj_min: ser::Result<Bson> = to_bson(&u64::MIN);
    assert_matches!(obj_min, Err(ser::Error::UnsupportedUnsignedType));
}

#[test]
#[cfg(feature = "u2i")]
fn uint64_u2i() {
    let _guard = LOCK.run_concurrently();
    let obj_min: Bson = to_bson(&u64::MIN).unwrap();
    let deser_min: u64 = from_bson(obj_min).unwrap();
    assert_eq!(deser_min, u64::MIN);

    let obj_max: ser::Result<Bson> = to_bson(&u64::MAX);
    assert_matches!(
        obj_max,
        Err(ser::Error::UnsignedTypesValueExceedsRange(u64::MAX))
    );
}

#[test]
fn int64() {
    let _guard = LOCK.run_concurrently();
    let obj = Bson::Int64(101);
    let i: i64 = from_bson(obj.clone()).unwrap();
    assert_eq!(i, 101);

    let deser: Bson = to_bson(&i).unwrap();
    assert_eq!(deser, obj);
}

#[test]
fn oid() {
    let _guard = LOCK.run_concurrently();
    let oid = ObjectId::new();
    let obj = Bson::ObjectId(oid);
    let s: BTreeMap<String, String> = from_bson(obj.clone()).unwrap();

    let mut expected = BTreeMap::new();
    expected.insert("$oid".to_owned(), oid.to_string());
    assert_eq!(s, expected);

    let deser: Bson = to_bson(&s).unwrap();
    assert_eq!(deser, obj);
}
