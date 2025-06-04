use std::{
    convert::TryFrom,
    io::{Cursor, Write},
};

use serde::{Deserialize, Serialize};

use crate::{
    de::from_document,
    doc,
    oid::ObjectId,
    ser::Error,
    spec::BinarySubtype,
    tests::LOCK,
    to_document,
    Binary,
    Bson,
    Decimal128,
    Document,
    JavaScriptCodeWithScope,
    RawDocumentBuf,
    Regex,
    Timestamp,
};
use serde_json::json;

#[test]
fn test_serialize_deserialize_floating_point() {
    let _guard = LOCK.run_concurrently();
    let src = 1020.123;
    let dst = vec![
        18, 0, 0, 0, 1, 107, 101, 121, 0, 68, 139, 108, 231, 251, 224, 143, 64, 0,
    ];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_utf8_string() {
    let _guard = LOCK.run_concurrently();
    let src = "test你好吗".to_owned();
    let dst = vec![
        28, 0, 0, 0, 2, 107, 101, 121, 0, 14, 0, 0, 0, 116, 101, 115, 116, 228, 189, 160, 229, 165,
        189, 229, 144, 151, 0, 0,
    ];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_encode_decode_utf8_string_invalid() {
    let bytes = b"\x80\xae".to_vec();
    let src = unsafe { String::from_utf8_unchecked(bytes) };

    let doc = doc! { "key": &src, "subdoc": { "subkey": &src } };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    let expected = doc! { "key": "��", "subdoc": { "subkey": "��" } };
    let decoded = RawDocumentBuf::from_reader(&mut Cursor::new(buf))
        .unwrap()
        .to_document_utf8_lossy()
        .unwrap();
    assert_eq!(decoded, expected);
}

#[test]
fn test_serialize_deserialize_array() {
    let _guard = LOCK.run_concurrently();
    let src = vec![Bson::Double(1.01), Bson::String("xyz".to_owned())];
    let dst = vec![
        37, 0, 0, 0, 4, 107, 101, 121, 0, 27, 0, 0, 0, 1, 48, 0, 41, 92, 143, 194, 245, 40, 240,
        63, 2, 49, 0, 4, 0, 0, 0, 120, 121, 122, 0, 0, 0,
    ];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize() {
    let _guard = LOCK.run_concurrently();
    let src = doc! { "subkey": 1 };
    let dst = vec![
        27, 0, 0, 0, 3, 107, 101, 121, 0, 17, 0, 0, 0, 16, 115, 117, 98, 107, 101, 121, 0, 1, 0, 0,
        0, 0, 0,
    ];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_boolean() {
    let _guard = LOCK.run_concurrently();
    let src = true;
    let dst = vec![11, 0, 0, 0, 8, 107, 101, 121, 0, 1, 0];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_null() {
    let _guard = LOCK.run_concurrently();
    let src = Bson::Null;
    let dst = vec![10, 0, 0, 0, 10, 107, 101, 121, 0, 0];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_regexp() {
    let _guard = LOCK.run_concurrently();
    let src = Bson::RegularExpression(Regex {
        pattern: "1".to_owned(),
        options: "2".to_owned(),
    });
    let dst = vec![14, 0, 0, 0, 11, 107, 101, 121, 0, 49, 0, 50, 0, 0];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_javascript_code() {
    let _guard = LOCK.run_concurrently();
    let src = Bson::JavaScriptCode("1".to_owned());
    let dst = vec![16, 0, 0, 0, 13, 107, 101, 121, 0, 2, 0, 0, 0, 49, 0, 0];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_javascript_code_with_scope() {
    let _guard = LOCK.run_concurrently();
    let src = Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope {
        code: "1".to_owned(),
        scope: doc! {},
    });
    let dst = vec![
        25, 0, 0, 0, 15, 107, 101, 121, 0, 15, 0, 0, 0, 2, 0, 0, 0, 49, 0, 5, 0, 0, 0, 0, 0,
    ];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_i32() {
    let _guard = LOCK.run_concurrently();
    let src = 100i32;
    let dst = vec![14, 0, 0, 0, 16, 107, 101, 121, 0, 100, 0, 0, 0, 0];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_i64() {
    let _guard = LOCK.run_concurrently();
    let src = 100i64;
    let dst = vec![
        18, 0, 0, 0, 18, 107, 101, 121, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_timestamp() {
    let _guard = LOCK.run_concurrently();
    let src = Bson::Timestamp(Timestamp {
        time: 0,
        increment: 100,
    });
    let dst = vec![
        18, 0, 0, 0, 17, 107, 101, 121, 0, 100, 0, 0, 0, 0, 0, 0, 0, 0,
    ];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_binary_generic() {
    let _guard = LOCK.run_concurrently();
    let src = Binary {
        subtype: BinarySubtype::Generic,
        bytes: vec![0, 1, 2, 3, 4],
    };
    let dst = vec![
        20, 0, 0, 0, 5, 107, 101, 121, 0, 5, 0, 0, 0, 0, 0, 1, 2, 3, 4, 0,
    ];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_object_id() {
    let _guard = LOCK.run_concurrently();
    let src = ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap();
    let dst = vec![
        22, 0, 0, 0, 7, 107, 101, 121, 0, 80, 127, 31, 119, 188, 248, 108, 215, 153, 67, 144, 17, 0,
    ];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_utc_date_time() {
    #[cfg(feature = "chrono-0_4")]
    use chrono::offset::TimeZone;
    let _guard = LOCK.run_concurrently();
    #[cfg(not(any(feature = "chrono-0_4", feature = "time-0_3")))]
    let src = crate::DateTime::from_time_0_3(
        time::OffsetDateTime::from_unix_timestamp(1_286_705_410).unwrap(),
    );
    #[cfg(feature = "time-0_3")]
    #[allow(unused)]
    let src = time::OffsetDateTime::from_unix_timestamp(1_286_705_410).unwrap();
    #[cfg(feature = "chrono-0_4")]
    let src = chrono::Utc.timestamp_opt(1_286_705_410, 0).unwrap();
    let dst = vec![
        18, 0, 0, 0, 9, 107, 101, 121, 0, 208, 111, 158, 149, 43, 1, 0, 0, 0,
    ];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_symbol() {
    let _guard = LOCK.run_concurrently();
    let symbol = Bson::Symbol("abc".to_owned());
    let dst = vec![
        18, 0, 0, 0, 14, 107, 101, 121, 0, 4, 0, 0, 0, 97, 98, 99, 0, 0,
    ];

    let doc = doc! { "key": symbol };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_deserialize_utc_date_time_overflows() {
    let _guard = LOCK.run_concurrently();
    let t: i64 = 1_530_492_218 * 1_000 + 999;

    let mut raw0 = vec![0x09, b'A', 0x00];
    raw0.write_all(&t.to_le_bytes()).unwrap();

    let mut raw = vec![];
    raw.write_all(&((raw0.len() + 4 + 1) as i32).to_le_bytes())
        .unwrap();
    raw.write_all(&raw0).unwrap();
    raw.write_all(&[0]).unwrap();

    let deserialized = Document::from_reader(&mut Cursor::new(raw)).unwrap();

    let expected = doc! { "A": crate::DateTime::from_time_0_3(time::OffsetDateTime::from_unix_timestamp(1_530_492_218).unwrap() + time::Duration::nanoseconds(999 * 1_000_000))};
    assert_eq!(deserialized, expected);
}

#[test]
fn test_deserialize_invalid_utf8_string_issue64() {
    let _guard = LOCK.run_concurrently();
    let buffer = b"\x13\x00\x00\x00\x02\x01\x00\x00\x00\x00\x00\x00\x00foo\x00\x13\x05\x00\x00\x00";

    assert!(Document::from_reader(&mut Cursor::new(buffer)).is_err());
}

#[test]
fn test_deserialize_multiply_overflows_issue64() {
    let _guard = LOCK.run_concurrently();
    let buffer = b"*\xc9*\xc9\t\x00\x00\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\xca\x01\t\x00\x00\x01\x10";

    assert!(Document::from_reader(&mut Cursor::new(&buffer[..])).is_err());
}

#[test]
fn test_serialize_deserialize_decimal128() {
    let _guard = LOCK.run_concurrently();
    let val = Bson::Decimal128(Decimal128 {
        bytes: [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 34],
    });
    let dst = vec![
        26, 0, 0, 0, 19, 107, 101, 121, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 34, 0,
    ];

    let doc = doc! { "key": val };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_illegal_size() {
    let _guard = LOCK.run_concurrently();
    let buffer = [
        0x06, 0xcc, 0xf9, 0x0a, 0x05, 0x00, 0x00, 0x03, 0x00, 0xff, 0xff,
    ];
    assert!(Document::from_reader(&mut Cursor::new(&buffer[..])).is_err());
}

#[test]
fn test_serialize_deserialize_undefined() {
    let _guard = LOCK.run_concurrently();
    let src = Bson::Undefined;
    let dst = vec![10, 0, 0, 0, 6, 107, 101, 121, 0, 0];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_min_key() {
    let _guard = LOCK.run_concurrently();
    let src = Bson::MinKey;
    let dst = vec![10, 0, 0, 0, 255, 107, 101, 121, 0, 0];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_max_key() {
    let _guard = LOCK.run_concurrently();
    let src = Bson::MaxKey;
    let dst = vec![10, 0, 0, 0, 127, 107, 101, 121, 0, 0];

    let doc = doc! {"key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_db_pointer() {
    let _guard = LOCK.run_concurrently();
    let src = Bson::try_from(json!({
        "$dbPointer": {
            "$ref": "db.coll",
            "$id": { "$oid": "507f1f77bcf86cd799439011" },
        }
    }))
    .unwrap();
    let dst = vec![
        34, 0, 0, 0, 12, 107, 101, 121, 0, 8, 0, 0, 0, 100, 98, 46, 99, 111, 108, 108, 0, 80, 127,
        31, 119, 188, 248, 108, 215, 153, 67, 144, 17, 0,
    ];

    let doc = doc! { "key": src };

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    assert_eq!(buf, dst);

    let deserialized = Document::from_reader(&mut Cursor::new(buf)).unwrap();
    assert_eq!(deserialized, doc);
}

#[test]
fn test_serialize_deserialize_document() {
    let _guard = LOCK.run_concurrently();

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct Point {
        x: i32,
        y: i32,
    }
    let src = Point { x: 1, y: 2 };

    let doc = to_document(&src).unwrap();
    assert_eq!(doc, doc! { "x": 1, "y": 2 });

    let point: Point = from_document(doc).unwrap();
    assert_eq!(src, point);

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct Line {
        p1: Point,
        p2: Point,
    }
    let src = Line {
        p1: Point { x: 0, y: 0 },
        p2: Point { x: 1, y: 1 },
    };

    let doc = to_document(&src).unwrap();
    assert_eq!(
        doc,
        doc! { "p1": { "x": 0, "y": 0 }, "p2": { "x": 1, "y": 1 } }
    );

    let line: Line = from_document(doc).unwrap();
    assert_eq!(src, line);

    let x = 1;
    let err = to_document(&x).unwrap_err();
    match err {
        Error::SerializationError { message } => {
            assert!(message.contains("Could not be serialized to Document"));
        }
        e => panic!("expected SerializationError, got {}", e),
    }

    let bad_point = doc! { "x": "one", "y": "two" };
    let bad_point: Result<Point, crate::de::Error> = from_document(bad_point);
    assert!(bad_point.is_err());
}

/// [RUST-713](https://jira.mongodb.org/browse/RUST-713)
#[test]
fn test_deserialize_invalid_array_length() {
    let _guard = LOCK.run_concurrently();
    let buffer = b"\n\x00\x00\x00\x04\x00\x00\x00\x00\x00";
    Document::from_reader(&mut std::io::Cursor::new(buffer))
        .expect_err("expected deserialization to fail");
}

/// [RUST-713](https://jira.mongodb.org/browse/RUST-713)
#[test]
fn test_deserialize_invalid_old_binary_length() {
    let _guard = LOCK.run_concurrently();
    let buffer = b"\x0F\x00\x00\x00\x05\x00\x00\x00\x00\x00\x02\xFC\xFF\xFF\xFF";
    Document::from_reader(&mut std::io::Cursor::new(buffer))
        .expect_err("expected deserialization to fail");

    let buffer = b".\x00\x00\x00\x05\x01\x00\x00\x00\x00\x00\x02\xfc\xff\xff\xff\xff\xff\xff\xff\x00\x00*\x00h\x0e\x10++\x00h\x0e++\x00\x00\t\x00\x00\x00\x00\x00*\x0e\x10++";
    Document::from_reader(&mut std::io::Cursor::new(buffer))
        .expect_err("expected deserialization to fail");
}
