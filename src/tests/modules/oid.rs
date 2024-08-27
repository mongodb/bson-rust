use crate::{oid::ObjectId, tests::LOCK};

#[test]
fn string_oid() {
    let _guard = LOCK.run_concurrently();
    let s = "123456789012123456789012";
    let oid_res = ObjectId::parse_str(s);
    assert!(oid_res.is_ok());
    let actual_s = hex::encode(oid_res.unwrap().bytes());
    assert_eq!(s.to_owned(), actual_s);
}

#[test]
fn byte_string_oid() {
    let _guard = LOCK.run_concurrently();
    let s = "541b1a00e8a23afa832b218e";
    let oid_res = ObjectId::parse_str(s);
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
#[allow(clippy::eq_op)]
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

#[test]
fn fromstr_oid() {
    let _guard = LOCK.run_concurrently();
    let s = "123456789012123456789012";
    let oid_res = s.parse::<ObjectId>();
    assert!(oid_res.is_ok(), "oid parse failed");
    let actual_s = hex::encode(oid_res.unwrap().bytes());
    assert_eq!(s, &actual_s, "parsed and expected oids differ");
}

#[test]
fn oid_from_parts() {
    let _guard = LOCK.run_concurrently();
    let seconds_since_epoch = 123;
    let process_id = [4, 5, 6, 7, 8];
    let counter = [9, 10, 11];
    let oid = ObjectId::from_parts(seconds_since_epoch, process_id, counter);
    assert_eq!(
        oid.timestamp().timestamp_millis(),
        i64::from(seconds_since_epoch) * 1000
    );
    assert_eq!(&oid.bytes()[4..9], &process_id);
    assert_eq!(&oid.bytes()[9..], &counter);
}
