use super::*;
use crate::{doc, spec::BinarySubtype, Binary, Bson, JavaScriptCodeWithScope, Regex, Timestamp};
use chrono::TimeZone;

fn to_bytes(doc: &crate::Document) -> Vec<u8> {
    let mut docbytes = Vec::new();
    doc.to_writer(&mut docbytes).unwrap();
    docbytes
}

#[test]
fn string_from_document() {
    let docbytes = to_bytes(&doc! {
        "this": "first",
        "that": "second",
        "something": "else",
    });
    let rawdoc = RawDocumentRef::new(&docbytes).unwrap();
    assert_eq!(
        rawdoc.get("that").unwrap().unwrap().as_str().unwrap(),
        "second",
    );
}

#[test]
fn nested_document() {
    let docbytes = to_bytes(&doc! {
        "outer": {
            "inner": "surprise",
        },
    });
    let rawdoc = RawDocumentRef::new(&docbytes).unwrap();
    assert_eq!(
        rawdoc
            .get("outer")
            .expect("get doc result")
            .expect("get doc option")
            .as_document()
            .expect("as doc")
            .get("inner")
            .expect("get str result")
            .expect("get str option")
            .as_str()
            .expect("as str"),
        "surprise",
    );
}

#[test]
fn iterate() {
    let docbytes = to_bytes(&doc! {
        "apples": "oranges",
        "peanut butter": "chocolate",
        "easy as": {"do": 1, "re": 2, "mi": 3},
    });
    let rawdoc = RawDocumentRef::new(&docbytes).expect("malformed bson document");
    let mut dociter = rawdoc.into_iter();
    let next = dociter.next().expect("no result").expect("invalid bson");
    assert_eq!(next.0, "apples");
    assert_eq!(next.1.as_str().expect("result was not a str"), "oranges");
    let next = dociter.next().expect("no result").expect("invalid bson");
    assert_eq!(next.0, "peanut butter");
    assert_eq!(next.1.as_str().expect("result was not a str"), "chocolate");
    let next = dociter.next().expect("no result").expect("invalid bson");
    assert_eq!(next.0, "easy as");
    let _doc = next.1.as_document().expect("result was a not a document");
    let next = dociter.next();
    assert!(next.is_none());
}

#[test]
fn rawdoc_to_doc() {
    let docbytes = to_bytes(&doc! {
        "f64": 2.5,
        "string": "hello",
        "document": {},
        "array": ["binary", "serialized", "object", "notation"],
        "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1, 2, 3] },
        "object_id": ObjectId::with_bytes([1, 2, 3, 4, 5,6,7,8,9,10, 11,12]),
        "boolean": true,
        "datetime": Utc::now(),
        "null": Bson::Null,
        "regex": Bson::RegularExpression(Regex { pattern: String::from(r"end\s*$"), options: String::from("i")}),
        "javascript": Bson::JavaScriptCode(String::from("console.log(console);")),
        "symbol": Bson::Symbol(String::from("artist-formerly-known-as")),
        "javascript_with_scope": Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope{ code: String::from("console.log(msg);"), scope: doc!{"ok": true}}),
        "int32": 23i32,
        "timestamp": Bson::Timestamp(Timestamp { time: 3542578, increment: 0 }),
        "int64": 46i64,
        "end": "END",
    });

    let rawdoc = RawDocumentRef::new(&docbytes).expect("invalid document");
    let _doc: crate::Document = rawdoc.try_into().expect("invalid bson");
}

#[test]
fn f64() {
    #![allow(clippy::float_cmp)]

    let rawdoc = RawDocument::from_document(&doc! {"f64": 2.5});
    assert_eq!(
        rawdoc
            .get("f64")
            .expect("error finding key f64")
            .expect("no key f64")
            .as_f64()
            .expect("result was not a f64"),
        2.5,
    );
}

#[test]
fn string() {
    let rawdoc = RawDocument::from_document(&doc! {"string": "hello"});

    assert_eq!(
        rawdoc
            .get("string")
            .expect("error finding key string")
            .expect("no key string")
            .as_str()
            .expect("result was not a string"),
        "hello",
    );
}
#[test]
fn document() {
    let rawdoc = RawDocument::from_document(&doc! {"document": {}});

    let doc = rawdoc
        .get("document")
        .expect("error finding key document")
        .expect("no key document")
        .as_document()
        .expect("result was not a document");
    assert_eq!(&doc.data, [5, 0, 0, 0, 0].as_ref()); // Empty document
}

#[test]
fn array() {
    let rawdoc = RawDocument::from_document(
        &doc! { "array": ["binary", "serialized", "object", "notation"]},
    );

    let array = rawdoc
        .get("array")
        .expect("error finding key array")
        .expect("no key array")
        .as_array()
        .expect("result was not an array");
    assert_eq!(array.get_str(0), Ok(Some("binary")));
    assert_eq!(array.get_str(3), Ok(Some("notation")));
    assert_eq!(array.get_str(4), Ok(None));
}

#[test]
fn binary() {
    let rawdoc = RawDocument::from_document(&doc! {
        "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] }
    });
    let binary: elem::RawBinary<'_> = rawdoc
        .get("binary")
        .expect("error finding key binary")
        .expect("no key binary")
        .as_binary()
        .expect("result was not a binary object");
    assert_eq!(binary.subtype, BinarySubtype::Generic);
    assert_eq!(binary.data, &[1, 2, 3]);
}

#[test]
fn object_id() {
    let rawdoc = RawDocument::from_document(&doc! {
        "object_id": ObjectId::with_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
    });
    let oid = rawdoc
        .get("object_id")
        .expect("error finding key object_id")
        .expect("no key object_id")
        .as_object_id()
        .expect("result was not an object id");
    assert_eq!(oid.to_hex(), "0102030405060708090a0b0c");
}

#[test]
fn boolean() {
    let rawdoc = RawDocument::from_document(&doc! {
        "boolean": true,
    });

    let boolean = rawdoc
        .get("boolean")
        .expect("error finding key boolean")
        .expect("no key boolean")
        .as_bool()
        .expect("result was not boolean");

    assert_eq!(boolean, true);
}

#[test]
fn datetime() {
    let rawdoc = RawDocument::from_document(&doc! {
        "boolean": true,
        "datetime": Utc.ymd(2000,10,31).and_hms(12, 30, 45),
    });
    let datetime = rawdoc
        .get("datetime")
        .expect("error finding key datetime")
        .expect("no key datetime")
        .as_datetime()
        .expect("result was not datetime");
    assert_eq!(datetime.to_rfc3339(), "2000-10-31T12:30:45+00:00");
}

#[test]
fn null() {
    let rawdoc = RawDocument::from_document(&doc! {
        "null": null,
    });
    let () = rawdoc
        .get("null")
        .expect("error finding key null")
        .expect("no key null")
        .as_null()
        .expect("was not null");
}

#[test]
fn regex() {
    let rawdoc = RawDocument::from_document(&doc! {
        "regex": Bson::RegularExpression(Regex { pattern: String::from(r"end\s*$"), options: String::from("i")}),
    });
    let regex = rawdoc
        .get("regex")
        .expect("error finding key regex")
        .expect("no key regex")
        .as_regex()
        .expect("was not regex");
    assert_eq!(regex.pattern, r"end\s*$");
    assert_eq!(regex.options, "i");
}
#[test]
fn javascript() {
    let rawdoc = RawDocument::from_document(&doc! {
        "javascript": Bson::JavaScriptCode(String::from("console.log(console);")),
    });
    let js = rawdoc
        .get("javascript")
        .expect("error finding key javascript")
        .expect("no key javascript")
        .as_javascript()
        .expect("was not javascript");
    assert_eq!(js, "console.log(console);");
}

#[test]
fn symbol() {
    let rawdoc = RawDocument::from_document(&doc! {
        "symbol": Bson::Symbol(String::from("artist-formerly-known-as")),
    });

    let symbol = rawdoc
        .get("symbol")
        .expect("error finding key symbol")
        .expect("no key symbol")
        .as_symbol()
        .expect("was not symbol");
    assert_eq!(symbol, "artist-formerly-known-as");
}

#[test]
fn javascript_with_scope() {
    let rawdoc = RawDocument::from_document(&doc! {
        "javascript_with_scope": Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope{ code: String::from("console.log(msg);"), scope: doc!{"ok": true}}),
    });
    let (js, scopedoc) = rawdoc
        .get("javascript_with_scope")
        .expect("error finding key javascript_with_scope")
        .expect("no key javascript_with_scope")
        .as_javascript_with_scope()
        .expect("was not javascript with scope");
    assert_eq!(js, "console.log(msg);");
    let (scope_key, scope_value_bson) = scopedoc
        .into_iter()
        .next()
        .expect("no next value in scope")
        .expect("invalid element");
    assert_eq!(scope_key, "ok");
    let scope_value = scope_value_bson.as_bool().expect("not a boolean");
    assert_eq!(scope_value, true);
}

#[test]
fn int32() {
    let rawdoc = RawDocument::from_document(&doc! {
        "int32": 23i32,
    });
    let int32 = rawdoc
        .get("int32")
        .expect("error finding key int32")
        .expect("no key int32")
        .as_i32()
        .expect("was not int32");
    assert_eq!(int32, 23i32);
}

#[test]
fn timestamp() {
    let rawdoc = RawDocument::from_document(&doc! {
        "timestamp": Bson::Timestamp(Timestamp { time: 3542578, increment: 7 }),
    });
    let ts = rawdoc
        .get("timestamp")
        .expect("error finding key timestamp")
        .expect("no key timestamp")
        .as_timestamp()
        .expect("was not a timestamp");

    assert_eq!(ts.increment(), 7);
    assert_eq!(ts.time(), 3542578);
}

#[test]
fn int64() {
    let rawdoc = RawDocument::from_document(&doc! {
        "int64": 46i64,
    });
    let int64 = rawdoc
        .get("int64")
        .expect("error finding key int64")
        .expect("no key int64")
        .as_i64()
        .expect("was not int64");
    assert_eq!(int64, 46i64);
}
#[test]
fn document_iteration() {
    let docbytes = to_bytes(&doc! {
        "f64": 2.5,
        "string": "hello",
        "document": {},
        "array": ["binary", "serialized", "object", "notation"],
        "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] },
        "object_id": ObjectId::with_bytes([1, 2, 3, 4, 5,6,7,8,9,10, 11,12]),
        "boolean": true,
        "datetime": Utc::now(),
        "null": Bson::Null,
        "regex": Bson::RegularExpression(Regex { pattern: String::from(r"end\s*$"), options: String::from("i")}),
        "javascript": Bson::JavaScriptCode(String::from("console.log(console);")),
        "symbol": Bson::Symbol(String::from("artist-formerly-known-as")),
        "javascript_with_scope": Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope{ code: String::from("console.log(msg);"), scope: doc!{"ok": true}}),
        "int32": 23i32,
        "timestamp": Bson::Timestamp(Timestamp { time: 3542578, increment: 0 }),
        "int64": 46i64,
        "end": "END",
    });
    let rawdoc = unsafe { RawDocumentRef::new_unchecked(&docbytes) };

    assert_eq!(
        rawdoc
            .into_iter()
            .collect::<Result<Vec<(&str, _)>, Error>>()
            .expect("collecting iterated doc")
            .len(),
        17
    );
    let end = rawdoc
        .get("end")
        .expect("error finding key end")
        .expect("no key end")
        .as_str()
        .expect("was not str");
    assert_eq!(end, "END");
}

#[test]
fn into_bson_conversion() {
    let docbytes = to_bytes(&doc! {
        "f64": 2.5,
        "string": "hello",
        "document": {},
        "array": ["binary", "serialized", "object", "notation"],
        "object_id": ObjectId::with_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
        "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] },
        "boolean": false,
    });
    let rawbson = elem::RawBson::new(ElementType::EmbeddedDocument, &docbytes);
    let b: Bson = rawbson.try_into().expect("invalid bson");
    let doc = b.as_document().expect("not a document");
    assert_eq!(*doc.get("f64").expect("f64 not found"), Bson::Double(2.5));
    assert_eq!(
        *doc.get("string").expect("string not found"),
        Bson::String(String::from("hello"))
    );
    assert_eq!(
        *doc.get("document").expect("document not found"),
        Bson::Document(doc! {})
    );
    assert_eq!(
        *doc.get("array").expect("array not found"),
        Bson::Array(
            vec!["binary", "serialized", "object", "notation"]
                .into_iter()
                .map(|s| Bson::String(String::from(s)))
                .collect()
        )
    );
    assert_eq!(
        *doc.get("object_id").expect("object_id not found"),
        Bson::ObjectId(ObjectId::with_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12
        ]))
    );
    assert_eq!(
        *doc.get("binary").expect("binary not found"),
        Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: vec![1, 2, 3]
        })
    );
    assert_eq!(
        *doc.get("boolean").expect("boolean not found"),
        Bson::Boolean(false)
    );
}

use proptest::prelude::*;
use std::convert::TryInto;

use super::{props::arbitrary_bson, RawDocument};
use crate::doc;

fn to_bytes(doc: &crate::Document) -> Vec<u8> {
    let mut docbytes = Vec::new();
    doc.to_writer(&mut docbytes).unwrap();
    docbytes
}

proptest! {
    #[test]
    fn no_crashes(s: Vec<u8>) {
        let _ = RawDocument::new(s);
    }

    #[test]
    fn roundtrip_bson(bson in arbitrary_bson()) {
        println!("{:?}", bson);
        let doc = doc!{"bson": bson};
        let raw = to_bytes(&doc);
        let raw = RawDocument::new(raw);
        prop_assert!(raw.is_ok());
        let raw = raw.unwrap();
        let roundtrip: Result<crate::Document, _> = raw.try_into();
        prop_assert!(roundtrip.is_ok());
        let roundtrip = roundtrip.unwrap();
        prop_assert_eq!(doc, roundtrip);
    }
}
