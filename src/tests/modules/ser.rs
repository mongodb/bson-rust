use std::collections::BTreeMap;

use assert_matches::assert_matches;

use crate::{from_bson, oid::ObjectId, ser, tests::LOCK, to_bson, to_vec, Bson, Document, Regex};

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
    assert!(b);

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

#[test]
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
fn uint64_u2i() {
    let _guard = LOCK.run_concurrently();
    let obj_min: Bson = to_bson(&u64::MIN).unwrap();
    let deser_min: u64 = from_bson(obj_min).unwrap();
    assert_eq!(deser_min, u64::MIN);

    let err: ser::Error = to_bson(&u64::MAX).unwrap_err().strip_path();
    assert_matches!(err, ser::Error::UnsignedIntegerExceededRange(u64::MAX));
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

#[test]
fn cstring_null_bytes_error() {
    let _guard = LOCK.run_concurrently();

    let doc = doc! { "\0": "a" };
    verify_doc(doc);

    let doc = doc! { "a": { "\0": "b" } };
    verify_doc(doc);

    let regex = doc! { "regex": Regex { pattern: "\0".into(), options: "a".into() } };
    verify_doc(regex);

    let regex = doc! { "regex": Regex { pattern: "a".into(), options: "\0".into() } };
    verify_doc(regex);

    fn verify_doc(doc: Document) {
        assert!(matches!(
            doc.to_vec().unwrap_err(),
            crate::error::Error {
                kind: crate::error::ErrorKind::MalformedValue { .. },
                ..
            }
        ));
        assert!(matches!(
            to_vec(&doc).unwrap_err().strip_path(),
            ser::Error::InvalidCString(_)
        ));
    }
}
