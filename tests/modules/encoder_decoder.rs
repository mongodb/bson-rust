use bson::oid::ObjectId;
use bson::spec::BinarySubtype;
use bson::{decode_document, decode_document_utf8_lossy, encode_document, Bson};
use byteorder::{LittleEndian, WriteBytesExt};
use bson::decimal128::Decimal128;
use chrono::offset::TimeZone;
use chrono::Utc;
use std::io::{Cursor, Write};

#[test]
fn test_encode_decode_floating_point() {
    let src = 1020.123;
    let dst = vec![18, 0, 0, 0, 1, 107, 101, 121, 0, 68, 139, 108, 231, 251, 224, 143, 64, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_decode_utf8_string() {
    let src = "test你好吗".to_owned();
    let dst = vec![28, 0, 0, 0, 2, 107, 101, 121, 0, 14, 0, 0, 0, 116, 101, 115, 116, 228, 189, 160, 229, 165, 189,
                   229, 144, 151, 0, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_decode_utf8_string_invalid() {
    let bytes = b"\x80\xae".to_vec();
    let src = unsafe { String::from_utf8_unchecked(bytes) };

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    let expected = doc!{ "key": "��" };
    let decoded = decode_document_utf8_lossy(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, expected);
}

#[test]
fn test_encode_decode_array() {
    let src = vec![Bson::FloatingPoint(1.01), Bson::String("xyz".to_owned())];
    let dst = vec![37, 0, 0, 0, 4, 107, 101, 121, 0, 27, 0, 0, 0, 1, 48, 0, 41, 92, 143, 194, 245, 40, 240, 63, 2, 49,
                   0, 4, 0, 0, 0, 120, 121, 122, 0, 0, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_decode_document() {
    let src = doc! { "subkey": 1 };
    let dst = vec![27, 0, 0, 0, 3, 107, 101, 121, 0, 17, 0, 0, 0, 16, 115, 117, 98, 107, 101, 121, 0, 1, 0, 0, 0, 0, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_decode_boolean() {
    let src = true;
    let dst = vec![11, 0, 0, 0, 8, 107, 101, 121, 0, 1, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_decode_null() {
    let src = Bson::Null;
    let dst = vec![10, 0, 0, 0, 10, 107, 101, 121, 0, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_decode_regexp() {
    let src = Bson::RegExp("1".to_owned(), "2".to_owned());
    let dst = vec![14, 0, 0, 0, 11, 107, 101, 121, 0, 49, 0, 50, 0, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_decode_javascript_code() {
    let src = Bson::JavaScriptCode("1".to_owned());
    let dst = vec![16, 0, 0, 0, 13, 107, 101, 121, 0, 2, 0, 0, 0, 49, 0, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_decode_javascript_code_with_scope() {
    let src = Bson::JavaScriptCodeWithScope("1".to_owned(), doc!{});
    let dst = vec![25, 0, 0, 0, 15, 107, 101, 121, 0, 15, 0, 0, 0, 2, 0, 0, 0, 49, 0, 5, 0, 0, 0, 0, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_decode_i32() {
    let src = 100i32;
    let dst = vec![14, 0, 0, 0, 16, 107, 101, 121, 0, 100, 0, 0, 0, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_decode_i64() {
    let src = 100i64;
    let dst = vec![18, 0, 0, 0, 18, 107, 101, 121, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_decode_timestamp() {
    let src = Bson::TimeStamp(100);
    let dst = vec![18, 0, 0, 0, 17, 107, 101, 121, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_binary_generic() {
    let src = (BinarySubtype::Generic, vec![0, 1, 2, 3, 4]);
    let dst = vec![20, 0, 0, 0, 5, 107, 101, 121, 0, 5, 0, 0, 0, 0, 0, 1, 2, 3, 4, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_decode_object_id() {
    let src = ObjectId::with_string("507f1f77bcf86cd799439011").unwrap();
    let dst = vec![22, 0, 0, 0, 7, 107, 101, 121, 0, 80, 127, 31, 119, 188, 248, 108, 215, 153, 67, 144, 17, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_utc_date_time() {
    let src = Utc.timestamp(1286705410, 0);
    let dst = vec![18, 0, 0, 0, 9, 107, 101, 121, 0, 208, 111, 158, 149, 43, 1, 0, 0, 0];

    let doc = doc!{ "key": src };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_encode_decode_symbol() {
    let symbol = Bson::Symbol("abc".to_owned());
    let dst = vec![18, 0, 0, 0, 14, 107, 101, 121, 0, 4, 0, 0, 0, 97, 98, 99, 0, 0];

    let doc = doc!{ "key": symbol };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_decode_utc_date_time_overflows() {
    let t = 1530492218 * 1_000 + 999;

    let mut raw0 = vec![0x09, b'A', 0x00];
    raw0.write_i64::<LittleEndian>(t).unwrap();

    let mut raw = vec![];
    raw.write_i32::<LittleEndian>((raw0.len() + 4 + 1) as i32).unwrap();
    raw.write_all(&raw0).unwrap();
    raw.write_u8(0).unwrap();

    let decoded = decode_document(&mut Cursor::new(raw)).unwrap();

    let expected = doc! { "A" => Utc.timestamp(1530492218, 999 * 1_000_000)};
    assert_eq!(decoded, expected);
}

#[test]
fn test_decode_invalid_utf8_string_issue64() {
    let buffer = b"\x13\x00\x00\x00\x02\x01\x00\x00\x00\x00\x00\x00\x00foo\x00\x13\x05\x00\x00\x00";

    assert!(decode_document(&mut Cursor::new(buffer)).is_err());
}

#[test]
fn test_decode_multiply_overflows_issue64() {
    let buffer = b"*\xc9*\xc9\t\x00\x00\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\x01\t\x00\x00\x01\x10";

    assert!(decode_document(&mut Cursor::new(&buffer[..])).is_err());
}

#[test]
fn test_encode_decode_decimal128() {
    let val = Bson::Decimal128(Decimal128::from_i32(0));
    let dst = vec![26, 0, 0, 0, 19, 107, 101, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8,
                   34, 0];

    let doc = doc! { "key" => val };

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(buf, dst);

    let decoded = decode_document(&mut Cursor::new(buf)).unwrap();
    assert_eq!(decoded, doc);
}

#[test]
fn test_illegal_size() {
    let buffer = [0x06, 0xcc, 0xf9, 0x0a, 0x05, 0x00, 0x00, 0x03, 0x00, 0xff, 0xff];
    assert!(decode_document(&mut Cursor::new(&buffer[..])).is_err());
}
