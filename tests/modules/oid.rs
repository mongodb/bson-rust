use bson::oid::ObjectId;
use hex;

#[test]
fn deserialize() {
    let bytes: [u8; 12] = [0xDEu8, 0xADu8, 0xBEu8, 0xEFu8, // timestamp is 3735928559
                           0xEFu8, 0xCDu8, 0xABu8, 0xFAu8, 0x29u8, 0x11u8, 0x22u8,
                           0x33u8 /* increment is 1122867 */];

    let oid = ObjectId::with_bytes(bytes);
    assert_eq!(3735928559 as u32, oid.timestamp());
    assert_eq!(1122867 as u32, oid.counter());
}

#[test]
fn timestamp() {
    let time: u32 = 2000000;
    let oid = ObjectId::with_timestamp(time);
    let timestamp = oid.timestamp();
    assert_eq!(time, timestamp);
}

#[test]
fn timestamp_is_big_endian() {
    let time: u32 = 3857379;
    let oid = ObjectId::with_timestamp(time);
    assert_eq!(0x00u8, oid.bytes()[0]);
    assert_eq!(0x3Au8, oid.bytes()[1]);
    assert_eq!(0xDBu8, oid.bytes()[2]);
    assert_eq!(0xE3u8, oid.bytes()[3]);
}

#[test]
fn string_oid() {
    let s = "123456789012123456789012";
    let oid_res = ObjectId::with_string(s);
    assert!(oid_res.is_ok());
    let actual_s = hex::encode(oid_res.unwrap().bytes());
    assert_eq!(s.to_owned(), actual_s);
}

#[test]
fn byte_string_oid() {
    let s = "541b1a00e8a23afa832b218e";
    let oid_res = ObjectId::with_string(s);
    assert!(oid_res.is_ok());
    let oid = oid_res.unwrap();
    let bytes: [u8; 12] =
        [0x54u8, 0x1Bu8, 0x1Au8, 0x00u8, 0xE8u8, 0xA2u8, 0x3Au8, 0xFAu8, 0x83u8, 0x2Bu8, 0x21u8, 0x8Eu8];

    assert_eq!(bytes, oid.bytes());
    assert_eq!(s, oid.to_string());
}

#[test]
fn oid_equals() {
    let oid_res = ObjectId::new();
    assert!(oid_res.is_ok());
    let oid = oid_res.unwrap();
    assert_eq!(oid, oid);
}

#[test]
fn oid_not_equals() {
    let oid1_res = ObjectId::new();
    let oid2_res = ObjectId::new();
    assert!(oid1_res.is_ok());
    assert!(oid2_res.is_ok());

    let oid1 = oid1_res.unwrap();
    let oid2 = oid2_res.unwrap();
    assert!(oid1 != oid2);
}

// check that the last byte in objectIDs is increasing
#[test]
fn counter_increasing() {
    let oid1_res = ObjectId::new();
    let oid2_res = ObjectId::new();
    assert!(oid1_res.is_ok());
    assert!(oid2_res.is_ok());

    let oid1_bytes = oid1_res.unwrap().bytes();
    let oid2_bytes = oid2_res.unwrap().bytes();
    assert!(oid1_bytes[11] < oid2_bytes[11]);
}
