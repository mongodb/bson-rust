use crate::{oid::ObjectId, tests::LOCK};

#[test]
fn string_oid() {
    let _guard = LOCK.run_concurrently();
    let s = "123456789012123456789012";
    let oid_res = ObjectId::with_string(s);
    assert!(oid_res.is_ok());
    let actual_s = hex::encode(oid_res.unwrap().bytes());
    assert_eq!(s.to_owned(), actual_s);
}

#[test]
fn byte_string_oid() {
    let _guard = LOCK.run_concurrently();
    let s = "541b1a00e8a23afa832b218e";
    let oid_res = ObjectId::with_string(s);
    assert!(oid_res.is_ok());
    let oid = oid_res.unwrap();
    let bytes: [u8; 12] = [
        0x54u8, 0x1Bu8, 0x1Au8, 0x00u8, 0xE8u8, 0xA2u8, 0x3Au8, 0xFAu8, 0x83u8, 0x2Bu8, 0x21u8,
        0x8Eu8,
    ];

    assert_eq!(bytes, oid.bytes());
    assert_eq!(s, oid.to_string());
}

#[test]
fn oid_equals() {
    let _guard = LOCK.run_concurrently();
    let oid = ObjectId::new();
    assert_eq!(oid, oid);
}

#[test]
fn oid_not_equals() {
    let _guard = LOCK.run_concurrently();
    assert!(ObjectId::new() != ObjectId::new());
}

// check that the last byte in objectIDs is increasing
#[test]
fn counter_increasing() {
    let _guard = LOCK.run_concurrently();
    let oid1_bytes = ObjectId::new().bytes();
    let oid2_bytes = ObjectId::new().bytes();
    assert!(oid1_bytes[11] < oid2_bytes[11]);
}
