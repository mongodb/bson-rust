mod append;
mod props;

use super::*;
use crate::{
    doc,
    oid::ObjectId,
    raw::error::ValueAccessErrorKind,
    spec::BinarySubtype,
    Binary,
    Bson,
    DateTime,
    JavaScriptCodeWithScope,
    Regex,
    Timestamp,
};
use chrono::{TimeZone, Utc};

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
    let rawdoc = RawDocument::from_bytes(&docbytes).unwrap();
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
            "i64": 6_i64,
        },
    });
    let rawdoc = RawDocument::from_bytes(&docbytes).unwrap();
    let subdoc = rawdoc
        .get("outer")
        .expect("get doc result")
        .expect("get doc option")
        .as_document()
        .expect("as doc");
    assert_eq!(
        subdoc
            .get("inner")
            .expect("get str result")
            .expect("get str option")
            .as_str()
            .expect("as str"),
        "surprise",
    );

    assert_eq!(
        subdoc
            .get("i64")
            .expect("get i64 result")
            .expect("get i64 option")
            .as_i64()
            .expect("as i64 result"),
        6
    );
}

#[test]
fn iterate() {
    let docbytes = to_bytes(&doc! {
        "apples": "oranges",
        "peanut butter": "chocolate",
        "easy as": {"do": 1, "re": 2, "mi": 3},
    });
    let rawdoc = RawDocument::from_bytes(&docbytes).expect("malformed bson document");
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
        "object_id": ObjectId::from_bytes([1, 2, 3, 4, 5,6,7,8,9,10, 11,12]),
        "boolean": true,
        "datetime": DateTime::now(),
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

    let rawdoc = RawDocument::from_bytes(&docbytes).expect("invalid document");
    let doc: crate::Document = rawdoc.try_into().expect("invalid bson");
    let round_tripped_bytes = crate::to_vec(&doc).expect("serialize should work");
    assert_eq!(round_tripped_bytes, docbytes);

    let mut vec_writer_bytes = vec![];
    doc.to_writer(&mut vec_writer_bytes)
        .expect("to writer should work");
    assert_eq!(vec_writer_bytes, docbytes);
}

#[test]
fn f64() {
    #![allow(clippy::float_cmp)]

    let rawdoc = RawDocumentBuf::from_document(&doc! { "f64": 2.5 }).unwrap();
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
    let rawdoc = RawDocumentBuf::from_document(&doc! {"string": "hello"}).unwrap();

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
    let rawdoc = RawDocumentBuf::from_document(&doc! {"document": {}}).unwrap();

    let doc = rawdoc
        .get("document")
        .expect("error finding key document")
        .expect("no key document")
        .as_document()
        .expect("result was not a document");
    assert_eq!(doc.as_bytes(), [5u8, 0, 0, 0, 0].as_ref()); // Empty document
}

#[test]
fn array() {
    let rawdoc = RawDocumentBuf::from_document(
        &doc! { "array": ["binary", "serialized", "object", "notation"]},
    )
    .unwrap();

    let array = rawdoc
        .get("array")
        .expect("error finding key array")
        .expect("no key array")
        .as_array()
        .expect("result was not an array");
    assert_eq!(array.get_str(0), Ok("binary"));
    assert_eq!(array.get_str(3), Ok("notation"));
    assert_eq!(
        array.get_str(4).unwrap_err().kind,
        ValueAccessErrorKind::NotPresent
    );
}

#[test]
fn binary() {
    let rawdoc = RawDocumentBuf::from_document(&doc! {
        "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] }
    })
    .unwrap();
    let binary: bson_ref::RawBinaryRef<'_> = rawdoc
        .get("binary")
        .expect("error finding key binary")
        .expect("no key binary")
        .as_binary()
        .expect("result was not a binary object");
    assert_eq!(binary.subtype, BinarySubtype::Generic);
    assert_eq!(binary.bytes, &[1, 2, 3]);
}

#[test]
fn object_id() {
    let rawdoc = RawDocumentBuf::from_document(&doc! {
        "object_id": ObjectId::from_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
    })
    .unwrap();
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
    let rawdoc = RawDocumentBuf::from_document(&doc! {
        "boolean": true,
    })
    .unwrap();

    let boolean = rawdoc
        .get("boolean")
        .expect("error finding key boolean")
        .expect("no key boolean")
        .as_bool()
        .expect("result was not boolean");

    assert!(boolean);
}

#[test]
fn datetime() {
    let rawdoc = RawDocumentBuf::from_document(&doc! {
        "boolean": true,
        "datetime": DateTime::from_chrono(Utc.ymd(2000,10,31).and_hms(12, 30, 45)),
    })
    .unwrap();
    let datetime = rawdoc
        .get("datetime")
        .expect("error finding key datetime")
        .expect("no key datetime")
        .as_datetime()
        .expect("result was not datetime");
    assert_eq!(datetime.to_rfc3339_string(), "2000-10-31T12:30:45Z");
}

#[test]
fn null() {
    let rawdoc = RawDocumentBuf::from_document(&doc! {
        "null": null,
    })
    .unwrap();
    let () = rawdoc
        .get("null")
        .expect("error finding key null")
        .expect("no key null")
        .as_null()
        .expect("was not null");
}

#[test]
fn regex() {
    let rawdoc = RawDocumentBuf::from_document(&doc! {
        "regex": Bson::RegularExpression(Regex { pattern: String::from(r"end\s*$"), options: String::from("i")}),
    }).unwrap();
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
    let rawdoc = RawDocumentBuf::from_document(&doc! {
        "javascript": Bson::JavaScriptCode(String::from("console.log(console);")),
    })
    .unwrap();
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
    let rawdoc = RawDocumentBuf::from_document(&doc! {
        "symbol": Bson::Symbol(String::from("artist-formerly-known-as")),
    })
    .unwrap();

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
    let rawdoc = RawDocumentBuf::from_document(&doc! {
        "javascript_with_scope": Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope {
            code: String::from("console.log(msg);"),
            scope: doc! { "ok": true }
        }),
    })
    .unwrap();
    let js_with_scope = rawdoc
        .get("javascript_with_scope")
        .expect("error finding key javascript_with_scope")
        .expect("no key javascript_with_scope")
        .as_javascript_with_scope()
        .expect("was not javascript with scope");
    assert_eq!(js_with_scope.code(), "console.log(msg);");
    let (scope_key, scope_value_bson) = js_with_scope
        .scope()
        .into_iter()
        .next()
        .expect("no next value in scope")
        .expect("invalid element");
    assert_eq!(scope_key, "ok");
    let scope_value = scope_value_bson.as_bool().expect("not a boolean");
    assert!(scope_value);
}

#[test]
fn int32() {
    let rawdoc = RawDocumentBuf::from_document(&doc! {
        "int32": 23i32,
    })
    .unwrap();
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
    let rawdoc = RawDocumentBuf::from_document(&doc! {
        "timestamp": Bson::Timestamp(Timestamp { time: 3542578, increment: 7 }),
    })
    .unwrap();
    let ts = rawdoc
        .get("timestamp")
        .expect("error finding key timestamp")
        .expect("no key timestamp")
        .as_timestamp()
        .expect("was not a timestamp");

    assert_eq!(ts.increment, 7);
    assert_eq!(ts.time, 3542578);
}

#[test]
fn int64() {
    let rawdoc = RawDocumentBuf::from_document(&doc! {
        "int64": 46i64,
    })
    .unwrap();
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
    let doc = doc! {
        "f64": 2.5,
        "string": "hello",
        "document": {},
        "array": ["binary", "serialized", "object", "notation"],
        "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] },
        "object_id": ObjectId::from_bytes([1, 2, 3, 4, 5,6,7,8,9,10, 11,12]),
        "boolean": true,
        "datetime": DateTime::now(),
        "null": Bson::Null,
        "regex": Bson::RegularExpression(Regex { pattern: String::from(r"end\s*$"), options: String::from("i")}),
        "javascript": Bson::JavaScriptCode(String::from("console.log(console);")),
        "symbol": Bson::Symbol(String::from("artist-formerly-known-as")),
        "javascript_with_scope": Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope{ code: String::from("console.log(msg);"), scope: doc!{"ok": true}}),
        "int32": 23i32,
        "timestamp": Bson::Timestamp(Timestamp { time: 3542578, increment: 0 }),
        "int64": 46i64,
        "end": "END",
    };
    let rawdoc = RawDocumentBuf::from_document(&doc).unwrap();
    let rawdocref = rawdoc.as_ref();

    assert_eq!(
        rawdocref
            .into_iter()
            .collect::<Result<Vec<(&str, _)>>>()
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
        "object_id": ObjectId::from_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
        "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] },
        "boolean": false,
    });
    let rawbson = RawBsonRef::Document(RawDocument::from_bytes(docbytes.as_slice()).unwrap());
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
        Bson::ObjectId(ObjectId::from_bytes([
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

use props::arbitrary_bson;
use proptest::prelude::*;
use std::convert::TryInto;

proptest! {
    #[test]
    fn no_crashes(s: Vec<u8>) {
        let _ = RawDocumentBuf::from_bytes(s);
    }

    #[test]
    fn roundtrip_bson(bson in arbitrary_bson()) {
        let doc = doc!{"bson": bson};
        let raw = to_bytes(&doc);
        let raw = RawDocumentBuf::from_bytes(raw);
        prop_assert!(raw.is_ok());
        let raw = raw.unwrap();
        let roundtrip: Result<crate::Document> = raw.try_into();
        prop_assert!(roundtrip.is_ok());
        let roundtrip = roundtrip.unwrap();
        prop_assert_eq!(doc, roundtrip);
    }
}
