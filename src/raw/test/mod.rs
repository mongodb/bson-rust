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
    Regex,
    Timestamp,
};

#[test]
fn string_from_document() {
    let rawdoc = rawdoc! {
        "this": "first",
        "that": "second",
        "something": "else",
    };
    assert_eq!(
        rawdoc.get("that").unwrap().unwrap().as_str().unwrap(),
        "second",
    );
}

#[test]
fn nested_document() {
    let rawdoc = rawdoc! {
        "outer": {
            "inner": "surprise",
            "i64": 6_i64
        }
    };
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
    let rawdoc = rawdoc! {
        "apples": "oranges",
        "peanut butter": "chocolate",
        "easy as": {"do": 1, "re": 2, "mi": 3},
    };
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
    let rawdoc = rawdoc! {
        "f64": 2.5,
        "string": "hello",
        "document": {},
        "array": ["binary", "serialized", "object", "notation"],
        "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1, 2, 3] },
        "object_id": ObjectId::from_bytes([1, 2, 3, 4, 5,6,7,8,9,10, 11,12]),
        "boolean": true,
        "datetime": DateTime::now(),
        "null": RawBson::Null,
        "regex": Regex { pattern: String::from(r"end\s*$"), options: String::from("i")},
        "javascript": RawBson::JavaScriptCode(String::from("console.log(console);")),
        "symbol": RawBson::Symbol(String::from("artist-formerly-known-as")),
        "javascript_with_scope": RawJavaScriptCodeWithScope {
            code: String::from("console.log(msg);"),
            scope: rawdoc! { "ok": true }
        },
        "int32": 23i32,
        "timestamp": Timestamp { time: 3542578, increment: 0 },
        "int64": 46i64,
        "end": "END",
    };

    let doc: crate::Document = rawdoc.clone().try_into().expect("invalid bson");
    let round_tripped_bytes = crate::to_vec(&doc).expect("serialize should work");
    assert_eq!(round_tripped_bytes.as_slice(), rawdoc.as_bytes());

    let mut vec_writer_bytes = vec![];
    doc.to_writer(&mut vec_writer_bytes)
        .expect("to writer should work");
    assert_eq!(vec_writer_bytes, rawdoc.into_bytes());
}

#[test]
fn f64() {
    #![allow(clippy::float_cmp)]

    let rawdoc = rawdoc! { "f64": 2.5 };
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
    let rawdoc = rawdoc! { "string": "hello" };

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
    let rawdoc = rawdoc! {"document": {}};

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
    let rawdoc = rawdoc! { "array": ["binary", "serialized", "object", "notation"] };
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
    let rawdoc = rawdoc! {
        "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] }
    };
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
    let rawdoc = rawdoc! {
        "object_id": ObjectId::from_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
    };
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
    let rawdoc = rawdoc! {
        "boolean": true,
    };

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
    use time::macros::datetime;

    let rawdoc = rawdoc! {
        "boolean": true,
        "datetime": DateTime::from_time_0_3(datetime!(2000-10-31 12:30:45 UTC)),
    };
    let datetime = rawdoc
        .get("datetime")
        .expect("error finding key datetime")
        .expect("no key datetime")
        .as_datetime()
        .expect("result was not datetime");
    assert_eq!(
        datetime.try_to_rfc3339_string().unwrap(),
        "2000-10-31T12:30:45Z"
    );
}

#[test]
fn null() {
    let rawdoc = rawdoc! {
        "null": null,
    };
    rawdoc
        .get("null")
        .expect("error finding key null")
        .expect("no key null")
        .as_null()
        .expect("was not null");
}

#[test]
fn regex() {
    let rawdoc = rawdoc! {
        "regex": Regex { pattern: String::from(r"end\s*$"), options: String::from("i")},
    };
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
    let rawdoc = rawdoc! {
        "javascript": RawBson::JavaScriptCode(String::from("console.log(console);")),
    };
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
    let rawdoc = rawdoc! {
        "symbol": RawBson::Symbol(String::from("artist-formerly-known-as")),
    };

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
    let rawdoc = rawdoc! {
        "javascript_with_scope": RawJavaScriptCodeWithScope {
            code: String::from("console.log(msg);"),
            scope: rawdoc! { "ok": true }
        },
    };
    let js_with_scope = rawdoc
        .get("javascript_with_scope")
        .expect("error finding key javascript_with_scope")
        .expect("no key javascript_with_scope")
        .as_javascript_with_scope()
        .expect("was not javascript with scope");
    assert_eq!(js_with_scope.code, "console.log(msg);");
    let (scope_key, scope_value_bson) = js_with_scope
        .scope
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
    let rawdoc = rawdoc! {
        "int32": 23i32,
    };
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
    let rawdoc = rawdoc! {
        "timestamp": Timestamp { time: 3542578, increment: 7 },
    };
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
    let rawdoc = rawdoc! {
        "int64": 46i64,
    };
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
    let rawdoc = rawdoc! {
        "f64": 2.5,
        "string": "hello",
        "document": {},
        "array": ["binary", "serialized", "object", "notation"],
        "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] },
        "object_id": ObjectId::from_bytes([1, 2, 3, 4, 5,6,7,8,9,10, 11,12]),
        "boolean": true,
        "datetime": DateTime::now(),
        "null": RawBson::Null,
        "regex": Regex { pattern: String::from(r"end\s*$"), options: String::from("i") },
        "javascript": RawBson::JavaScriptCode(String::from("console.log(console);")),
        "symbol": RawBson::Symbol(String::from("artist-formerly-known-as")),
        "javascript_with_scope": RawJavaScriptCodeWithScope {
            code: String::from("console.log(msg);"),
            scope: rawdoc! { "ok": true }
        },
        "int32": 23i32,
        "timestamp": Timestamp { time: 3542578, increment: 0 },
        "int64": 46i64,
        "end": "END",
    };

    assert_eq!(
        rawdoc
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
    let rawdoc = rawdoc! {
        "f64": 2.5,
        "string": "hello",
        "document": {},
        "array": ["binary", "serialized", "object", "notation"],
        "object_id": ObjectId::from_bytes([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
        "binary": Binary { subtype: BinarySubtype::Generic, bytes: vec![1u8, 2, 3] },
        "boolean": false,
    };
    let rawbson = RawBsonRef::Document(RawDocument::from_bytes(rawdoc.as_bytes()).unwrap());
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
        let doc = doc! { "bson": bson };
        let raw = crate::to_vec(&doc);
        prop_assert!(raw.is_ok());
        let raw = RawDocumentBuf::from_bytes(raw.unwrap());
        prop_assert!(raw.is_ok());
        let raw = raw.unwrap();
        let roundtrip: Result<crate::Document> = raw.try_into();
        prop_assert!(roundtrip.is_ok());
        let roundtrip = roundtrip.unwrap();
        prop_assert_eq!(doc, roundtrip);
    }
}
