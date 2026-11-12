mod append;
mod props;

use super::*;
use crate::{
    Binary,
    Bson,
    DateTime,
    Decimal128,
    Document,
    Regex,
    Timestamp,
    Utf8Lossy,
    oid::ObjectId,
    spec::{BinarySubtype, ElementType},
};

#[test]
fn test_decimal128_doesnt_panic_on_bad_codepoint_boundary() {
    use std::str::FromStr;
    // idx 34 (Coefficient::MAX_DIGITS) on this string isn't a valid codepoint boundary
    assert!(
        Decimal128::from_str("111111111111111111111111111111111❤")
            .unwrap_err()
            .is_decimal128_unparseable()
    );
}

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
        "regex": Regex { pattern: cstr!(r"end\s*$").into(), options: cstr!("i").into()},
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
    #[cfg(feature = "serde")]
    {
        let round_tripped_bytes = crate::serialize_to_vec(&doc).expect("serialize should work");
        assert_eq!(round_tripped_bytes.as_slice(), rawdoc.as_bytes());
    }

    let vec_writer_bytes = doc.to_vec().expect("encode should work");
    assert_eq!(vec_writer_bytes, rawdoc.into_bytes());
}

#[test]
fn code_with_scope_invalid_utf8_lossy() {
    // the code here holds invalid utf-8, so its lossily decoded form is longer than the
    // bytes actually stored. the scope follows the raw code bytes, so decoding must not
    // use the replacement-decoded length to find it.
    let code = unsafe { String::from_utf8_unchecked(vec![0x80, 0xae]) };
    let rawdoc = rawdoc! {
        "c": RawJavaScriptCodeWithScope {
            code,
            scope: rawdoc! { "ok": true },
        },
    };

    let lossy: Utf8Lossy<Document> = (&*rawdoc)
        .try_into()
        .expect("lossy conversion should succeed");
    match lossy.0.get("c").expect("missing key") {
        Bson::JavaScriptCodeWithScope(jsc) => {
            assert_eq!(jsc.code, "\u{FFFD}\u{FFFD}");
            assert!(jsc.scope.get_bool("ok").expect("scope key"));
        }
        other => panic!("unexpected value: {:?}", other),
    }
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
    assert_eq!(array.get_str(0).unwrap(), "binary");
    assert_eq!(array.get_str(3).unwrap(), "notation");
    assert!(array.get_str(4).unwrap_err().is_value_access_not_present());
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
        "regex": Regex { pattern: cstr!(r"end\s*$").into(), options: cstr!("i").into()},
    };
    let regex = rawdoc
        .get("regex")
        .expect("error finding key regex")
        .expect("no key regex")
        .as_regex()
        .expect("was not regex");
    assert_eq!(regex.pattern, cstr!(r"end\s*$"));
    assert_eq!(regex.options, cstr!("i"));
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
        "regex": Regex { pattern: cstr!(r"end\s*$").into(), options: cstr!("i").into() },
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
            .collect::<Result<Vec<(_, _)>>>()
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

#[cfg(feature = "serde")]
#[test]
fn fuzz_oom() {
    let bytes: &[u8] = &[
        17, 0, 0, 0, 11, 36, 100, 97, 116, 101, 0, 111, 112, 101, 0, 4, 0,
    ];
    let _ = crate::deserialize_from_slice::<crate::Document>(bytes);
}

#[cfg(not(feature = "unbounded-depth"))]
mod nesting_test {
    use crate::{
        Document,
        RawArrayBuf,
        RawDocumentBuf,
        RawJavaScriptCodeWithScope,
        Utf8Lossy,
        cstr,
    };

    #[derive(Debug, Clone, Copy)]
    enum Nesting {
        Document,
        Array,
        CodeWithScope,
    }

    fn nested_bson(depth: usize, nesting: &[Nesting]) -> RawDocumentBuf {
        let mut doc = RawDocumentBuf::new();

        for level in 0..depth {
            let mut outer = RawDocumentBuf::new();
            match nesting[level % nesting.len()] {
                Nesting::Document => outer.append(cstr!("d"), doc),
                Nesting::Array => {
                    let mut array = RawArrayBuf::new();
                    array.push(doc);
                    outer.append(cstr!("a"), array);
                }
                Nesting::CodeWithScope => outer.append(
                    cstr!("c"),
                    RawJavaScriptCodeWithScope {
                        code: String::new(),
                        scope: doc,
                    },
                ),
            }
            doc = outer;
        }

        doc
    }

    #[test]
    fn nesting_limit() {
        // documents, arrays, alternating documents and arrays, and code-with-scope
        for nesting in [
            &[Nesting::Document][..],
            &[Nesting::Array][..],
            &[Nesting::Document, Nesting::Array][..],
            &[Nesting::CodeWithScope][..],
        ] {
            let raw = nested_bson(10_000, nesting);
            let bytes = raw.as_bytes();

            let err = Document::from_reader(bytes).unwrap_err();
            assert!(err.is_recursion_limit(), "{nesting:?}: {err:?}");

            let err = Document::try_from(raw.as_ref()).unwrap_err();
            assert!(err.is_recursion_limit(), "{nesting:?}: {err:?}");

            let err = Utf8Lossy::<Document>::try_from(raw.as_ref()).unwrap_err();
            assert!(err.is_recursion_limit(), "{nesting:?}: {err:?}");

            #[cfg(feature = "serde")]
            {
                let err = crate::deserialize_from_slice::<Document>(bytes).unwrap_err();
                assert!(err.is_recursion_limit(), "{nesting:?}: {err:?}");

                let err = crate::deserialize_from_slice::<Utf8Lossy<Document>>(bytes).unwrap_err();
                assert!(err.is_recursion_limit(), "{nesting:?}: {err:?}");
            }
        }
    }
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
        let bytes = doc.to_vec();
        prop_assert!(bytes.is_ok());
        let bytes = bytes.unwrap();
        #[cfg(feature = "serde")]
        {
            let raw = crate::serialize_to_vec(&doc);
            prop_assert!(raw.is_ok());
        }
        let raw = RawDocumentBuf::from_bytes(bytes);
        prop_assert!(raw.is_ok());
        let raw = raw.unwrap();
        let roundtrip: Result<crate::Document> = raw.try_into();
        prop_assert!(roundtrip.is_ok());
        let roundtrip = roundtrip.unwrap();
        prop_assert_eq!(doc, roundtrip);
    }
}

#[test]
#[cfg(feature = "sfp-internal")]
fn max_cstr_parse_len() {
    let key = cstr!("aaaaaaaa");
    let doc = rawdoc! { key: "b" };

    let mut iter = doc.iter().max_cstr_parse_len(key.len());
    let (k, _) = iter.next().unwrap().unwrap();
    assert_eq!(k, key);

    let mut iter = doc.iter().max_cstr_parse_len(key.len() - 1);
    let error = iter.next().unwrap().unwrap_err();
    match error.kind {
        ErrorKind::TooLongCStr {
            max_parse_len,
            bytes,
        } => {
            assert_eq!(max_parse_len, key.len() - 1);
            assert_eq!(bytes.as_slice(), &key.as_str().bytes().collect::<Vec<_>>());
        }
        other => panic!("expected TooLongCStr, got {}", other),
    }

    let mut iter = doc.iter_elements().max_cstr_parse_len(key.len() - 5);
    let Err(error) = iter.next().unwrap() else {
        panic!("expected error");
    };
    assert!(matches!(error.kind, ErrorKind::TooLongCStr { .. }));

    let b = doc
        .get_with_max_cstr_parse_len(key.as_str(), key.len())
        .unwrap()
        .unwrap()
        .as_str()
        .unwrap();
    assert_eq!(b, "b");

    let error = doc
        .get_with_max_cstr_parse_len(key.as_str(), key.len() - 2)
        .unwrap_err();
    assert!(matches!(error.kind, ErrorKind::TooLongCStr { .. }));

    // ensure we don't panic on OOB if the max parse len exceeds the document's length
    let mut iter = doc.iter().max_cstr_parse_len(doc.as_bytes().len() + 1);
    iter.next().unwrap().unwrap();

    // a long key in a nested document shouldn't impact parsing
    let nested_doc = rawdoc! { "nested": doc, "after": "c" };
    for result in nested_doc.iter().max_cstr_parse_len(key.len() - 1) {
        result.unwrap();
    }

    // a too-long key earlier in the doc should cause an error
    let longer_key = cstr!("aaaaaaaaaaaaaaaa");
    let doc = rawdoc! { longer_key: "b", key: "c" };
    let error = doc.get_with_max_cstr_parse_len(key, key.len()).unwrap_err();
    assert!(matches!(error.kind, ErrorKind::TooLongCStr { .. }));

    let mut bytes = rawdoc! { "array": { key: "element" } }.into_bytes();
    // change the type id for the value from document to array
    bytes[4] = 4;
    let doc_with_array = RawDocumentBuf::from_bytes(bytes).unwrap();
    let array = doc_with_array.get_array("array").unwrap();

    let mut iter = array.into_iter().max_cstr_parse_len(key.len());
    let val = iter.next().unwrap().unwrap().as_str().unwrap();
    assert_eq!(val, "element");

    let mut iter = array.into_iter().max_cstr_parse_len(key.len() - 1);
    let error = iter.next().unwrap().unwrap_err();
    assert!(matches!(error.kind, ErrorKind::TooLongCStr { .. }));

    let pattern = cstr!("pattern");
    let options = cstr!("abc");
    let key = cstr!("r");
    let regex = RawRegexRef { pattern, options };
    let doc_with_regex = rawdoc! { key: RawBsonRef::from(regex) };

    let mut iter = doc_with_regex.iter().max_cstr_parse_len(pattern.len());
    let (key_from_doc, bson) = iter.next().unwrap().unwrap();
    let regex_from_doc = bson.as_regex().unwrap();
    assert_eq!(key_from_doc, key);
    assert_eq!(regex_from_doc, regex);

    let mut iter = doc_with_regex.iter().max_cstr_parse_len(pattern.len() - 1);
    let error = iter.next().unwrap().unwrap_err();
    match error.kind {
        ErrorKind::TooLongCStr {
            max_parse_len,
            bytes,
        } => {
            assert_eq!(max_parse_len, pattern.len() - 1);
            assert_eq!(bytes, pattern.as_str().bytes().collect::<Vec<_>>());
        }
        other => panic!("expected TooLongCStr, got {}", other),
    }

    let mut iter = doc_with_regex
        .iter_elements()
        .max_cstr_parse_len(pattern.len() - 1);
    // an error occurs here rather than when accessing the value because the iterator needs to read
    // the regex's cstrings to determine its overall length
    let Err(error) = iter.next().unwrap() else {
        panic!("expected error");
    };
    assert!(matches!(error.kind, ErrorKind::TooLongCStr { .. }));
}

// helpers for the element-view tests: build complete document bytes from element bytes

fn element_view_doc_bytes(elements: &[u8]) -> Vec<u8> {
    let mut out = ((elements.len() + 5) as i32).to_le_bytes().to_vec();
    out.extend(elements);
    out.push(0);
    out
}

fn element_view_len_encode(payload: &[u8]) -> Vec<u8> {
    let mut out = ((payload.len() + 1) as i32).to_le_bytes().to_vec();
    out.extend(payload);
    out.push(0);
    out
}

fn element_view_cstring(payload: &[u8]) -> Vec<u8> {
    let mut out = payload.to_vec();
    out.push(0);
    out
}

#[test]
fn element_view_iter_keys_and_values() {
    // {"s": "hello", "i": 42}
    let mut elements = Vec::new();
    elements.push(0x02);
    elements.extend(element_view_cstring(b"s"));
    elements.extend(element_view_len_encode(b"hello"));
    elements.push(0x10);
    elements.extend(element_view_cstring(b"i"));
    elements.extend(42i32.to_le_bytes());

    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();
    let mut iter = doc.iter_element_views();

    let element = iter.next().unwrap().unwrap();
    assert_eq!(element.key_bytes(), b"s");
    assert_eq!(element.element_type(), ElementType::String);
    // value_bytes includes framing: the length prefix and null terminator
    assert_eq!(element.value_bytes(), &element_view_len_encode(b"hello"));
    // strict parsing is still available as an explicit step
    assert_eq!(element.value().unwrap().as_str(), Some("hello"));

    let element = iter.next().unwrap().unwrap();
    assert_eq!(element.key_bytes(), b"i");
    assert_eq!(element.value().unwrap().as_i32(), Some(42));

    assert!(iter.next().is_none());
}

#[test]
fn element_view_iter_invalid_utf8_key() {
    let mut elements = Vec::new();
    elements.push(0x02);
    elements.extend(element_view_cstring(b"\xFF\xFE"));
    elements.extend(element_view_len_encode(b"value"));

    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();

    // the existing iterator stops at the invalid key
    assert!(doc.iter_elements().next().unwrap().is_err());

    // the view iterator yields the raw key bytes
    let mut iter = doc.iter_element_views();
    let element = iter.next().unwrap().unwrap();
    assert_eq!(element.key_bytes(), b"\xFF\xFE");
    assert_eq!(element.value().unwrap().as_str(), Some("value"));
    assert!(iter.next().is_none());

    // validating the key is an explicit, fallible step
    let mut iter = doc.iter_element_views();
    let element = iter.next().unwrap().unwrap();
    assert!(RawElement::try_from(element).is_err());

    // a valid key round-trips into a RawElement
    let mut elements = Vec::new();
    elements.push(0x02);
    elements.extend(element_view_cstring(b"ok"));
    elements.extend(element_view_len_encode(b"value"));
    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();
    let element = doc.iter_element_views().next().unwrap().unwrap();
    let raw_element = RawElement::try_from(element).unwrap();
    assert_eq!(raw_element.key(), "ok");
    assert_eq!(raw_element.value().unwrap().as_str(), Some("value"));
}

#[test]
fn element_view_iter_rejects_underfilled_body() {
    // a valid one-element document...
    let mut elements = Vec::new();
    elements.push(0x10);
    elements.extend(element_view_cstring(b"i"));
    elements.extend(1i32.to_le_bytes());
    let mut bytes = element_view_doc_bytes(&elements);
    // ...with one extra trailing byte appended (and the length prefix adjusted)
    let length = i32::from_le_bytes(bytes[..4].try_into().unwrap()) + 1;
    bytes[..4].copy_from_slice(&length.to_le_bytes());
    bytes.push(0);

    let doc = RawDocument::from_bytes(&bytes).unwrap();
    let mut iter = doc.iter_element_views();

    // the element is located...
    assert!(iter.next().unwrap().is_ok());
    // ...but the unconsumed body byte is an error, not a clean end of iteration
    assert!(iter.next().unwrap().is_err());
    // and iteration has ended
    assert!(iter.next().is_none());
}

#[test]
fn element_view_iter_empty_document() {
    let doc = RawDocument::from_bytes(b"\x05\0\0\0\0").unwrap();
    let mut iter = doc.iter_element_views();
    assert!(iter.next().is_none());
}

#[test]
fn element_view_iter_malformed_documents() {
    // invalid type tag
    let mut elements = Vec::new();
    elements.push(0x63);
    elements.extend(element_view_cstring(b"k"));
    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();
    let mut iter = doc.iter_element_views();
    let err = iter.next().unwrap().unwrap_err();
    assert!(err.to_string().contains("invalid tag"));
    assert!(iter.next().is_none());

    // string value that overruns the document
    let mut elements = Vec::new();
    elements.push(0x02);
    elements.extend(element_view_cstring(b"s"));
    elements.extend([99, 0, 0, 0]); // declares 99 bytes of string
    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();
    let mut iter = doc.iter_element_views();
    assert!(iter.next().unwrap().is_err());
    assert!(iter.next().is_none());

    // key that is not null terminated
    let mut elements = Vec::new();
    elements.push(0x10);
    elements.extend(b"unterminated");
    elements.extend(1i32.to_le_bytes());
    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();
    let mut iter = doc.iter_element_views();
    assert!(iter.next().unwrap().is_err());
    assert!(iter.next().is_none());
}

#[test]
fn element_view_iter_full_shape() {
    let oid = ObjectId::from_bytes([7u8; 12]);
    let mut elements = Vec::new();
    // "s": "hello"
    elements.push(0x02);
    elements.extend(element_view_cstring(b"s"));
    elements.extend(element_view_len_encode(b"hello"));
    // "sym": "sym"
    elements.push(0x0E);
    elements.extend(element_view_cstring(b"sym"));
    elements.extend(element_view_len_encode(b"sym"));
    // "code": "code"
    elements.push(0x0D);
    elements.extend(element_view_cstring(b"code"));
    elements.extend(element_view_len_encode(b"code"));
    // "re": /ab/b
    elements.push(0x0B);
    elements.extend(element_view_cstring(b"re"));
    elements.extend(element_view_cstring(b"ab"));
    elements.extend(element_view_cstring(b"b"));
    // "dp": dbpointer
    elements.push(0x0C);
    elements.extend(element_view_cstring(b"dp"));
    elements.extend(element_view_len_encode(b"ns"));
    elements.extend([7u8; 12]);
    // "bin": binary
    elements.push(0x05);
    elements.extend(element_view_cstring(b"bin"));
    elements.extend(3i32.to_le_bytes()); // payload length
    elements.push(0x80); // subtype
    elements.extend([1, 2, 3]); // payload
    // "doc": {}
    elements.push(0x03);
    elements.extend(element_view_cstring(b"doc"));
    elements.extend(element_view_doc_bytes(b""));
    // "arr": []
    elements.push(0x04);
    elements.extend(element_view_cstring(b"arr"));
    elements.extend(element_view_doc_bytes(b""));
    // "i32": 42
    elements.push(0x10);
    elements.extend(element_view_cstring(b"i32"));
    elements.extend(42i32.to_le_bytes());
    // "f64": 2.5
    elements.push(0x01);
    elements.extend(element_view_cstring(b"f64"));
    elements.extend(2.5f64.to_le_bytes());
    // "bool": true
    elements.push(0x08);
    elements.extend(element_view_cstring(b"bool"));
    elements.push(1);
    // "date": 0
    elements.push(0x09);
    elements.extend(element_view_cstring(b"date"));
    elements.extend(0i64.to_le_bytes());
    // "ts": 0
    elements.push(0x11);
    elements.extend(element_view_cstring(b"ts"));
    elements.extend(0u64.to_le_bytes());
    // "oid": oid
    elements.push(0x07);
    elements.extend(element_view_cstring(b"oid"));
    elements.extend([7u8; 12]);
    // "nul": null
    elements.push(0x0A);
    elements.extend(element_view_cstring(b"nul"));

    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();
    let mut iter = doc.iter_element_views();

    let element = iter.next().unwrap().unwrap();
    assert_eq!(element.key_bytes(), b"s");
    assert_eq!(element.element_type(), ElementType::String);
    assert_eq!(
        element.value_view().unwrap(),
        RawBsonRefView::String(b"hello")
    );

    let element = iter.next().unwrap().unwrap();
    assert_eq!(
        element.value_view().unwrap(),
        RawBsonRefView::Symbol(b"sym")
    );

    let element = iter.next().unwrap().unwrap();
    assert_eq!(
        element.value_view().unwrap(),
        RawBsonRefView::JavaScriptCode(b"code")
    );

    let element = iter.next().unwrap().unwrap();
    match element.value_view().unwrap() {
        RawBsonRefView::RegularExpression { pattern, options } => {
            assert_eq!(pattern, b"ab");
            assert_eq!(options, b"b");
        }
        other => panic!("expected regex, got {other:?}"),
    }

    let element = iter.next().unwrap().unwrap();
    match element.value_view().unwrap() {
        RawBsonRefView::DbPointer { namespace, id } => {
            assert_eq!(namespace, b"ns");
            assert_eq!(id, oid);
        }
        other => panic!("expected dbpointer, got {other:?}"),
    }

    let element = iter.next().unwrap().unwrap();
    match element.value_view().unwrap() {
        RawBsonRefView::Binary(binary) => {
            assert_eq!(binary.bytes, &[1, 2, 3]);
            assert_eq!(binary.subtype, BinarySubtype::UserDefined(0x80));
        }
        other => panic!("expected binary, got {other:?}"),
    }

    let element = iter.next().unwrap().unwrap();
    match element.value_view().unwrap() {
        RawBsonRefView::Document(nested) => {
            assert_eq!(nested.as_bytes(), b"\x05\0\0\0\0");
            assert!(nested.iter_element_views().next().is_none());
        }
        other => panic!("expected document, got {other:?}"),
    }

    let element = iter.next().unwrap().unwrap();
    match element.value_view().unwrap() {
        RawBsonRefView::Array(array) => {
            assert!(array.iter_element_views().next().is_none());
        }
        other => panic!("expected array, got {other:?}"),
    }

    let element = iter.next().unwrap().unwrap();
    assert_eq!(element.value_view().unwrap(), RawBsonRefView::Int32(42));

    let element = iter.next().unwrap().unwrap();
    assert_eq!(element.value_view().unwrap(), RawBsonRefView::Double(2.5));

    let element = iter.next().unwrap().unwrap();
    assert_eq!(element.value_view().unwrap(), RawBsonRefView::Boolean(true));

    let element = iter.next().unwrap().unwrap();
    assert_eq!(
        element.value_view().unwrap(),
        RawBsonRefView::DateTime(DateTime::from_millis(0))
    );

    let element = iter.next().unwrap().unwrap();
    match element.value_view().unwrap() {
        RawBsonRefView::Timestamp(ts) => {
            assert_eq!(
                ts,
                Timestamp {
                    time: 0,
                    increment: 0
                }
            );
        }
        other => panic!("expected timestamp, got {other:?}"),
    }

    let element = iter.next().unwrap().unwrap();
    assert_eq!(element.value_view().unwrap(), RawBsonRefView::ObjectId(oid));

    let element = iter.next().unwrap().unwrap();
    assert_eq!(element.value_view().unwrap(), RawBsonRefView::Null);

    assert!(iter.next().is_none());
}

#[test]
fn element_view_iter_code_with_scope() {
    // "cws": code with scope {"x": 1}, code contains invalid utf-8
    let mut scope_elements = Vec::new();
    scope_elements.push(0x10);
    scope_elements.extend(element_view_cstring(b"x"));
    scope_elements.extend(1i32.to_le_bytes());
    let scope = element_view_doc_bytes(&scope_elements);
    let code = element_view_len_encode(b"va\xFFr");
    let mut value = ((4 + code.len() + scope.len()) as i32)
        .to_le_bytes()
        .to_vec();
    value.extend(&code);
    value.extend(&scope);

    let mut elements = Vec::new();
    elements.push(0x0F);
    elements.extend(element_view_cstring(b"cws"));
    elements.extend(&value);

    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();
    let mut iter = doc.iter_element_views();

    let element = iter.next().unwrap().unwrap();
    match element.value_view().unwrap() {
        RawBsonRefView::JavaScriptCodeWithScope { code, scope } => {
            assert_eq!(code, b"va\xFFr");
            assert_eq!(scope.as_bytes(), &element_view_doc_bytes(&scope_elements));
            // the scope can be recursed into with the same iterator
            let mut scope_iter = scope.iter_element_views();
            let scope_element = scope_iter.next().unwrap().unwrap();
            assert_eq!(scope_element.key_bytes(), b"x");
            assert_eq!(
                scope_element.value_view().unwrap(),
                RawBsonRefView::Int32(1)
            );
            assert!(scope_iter.next().is_none());
        }
        other => panic!("expected code with scope, got {other:?}"),
    }
    assert!(iter.next().is_none());
}

#[test]
fn element_view_iter_invalid_utf8_string_value() {
    let mut elements = Vec::new();
    elements.push(0x02);
    elements.extend(element_view_cstring(b"s"));
    elements.extend(element_view_len_encode(b"h\xFFllo"));

    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();

    let mut iter = doc.iter_element_views();
    let element = iter.next().unwrap().unwrap();
    // strict parsing validates utf-8
    assert!(element.value().is_err());
    // shape parsing leaves the payload bytes untouched
    assert_eq!(
        element.value_view().unwrap(),
        RawBsonRefView::String(b"h\xFFllo")
    );
}

#[test]
fn element_view_iter_invalid_utf8_regex_and_dbpointer() {
    // regex with invalid utf-8 pattern
    let mut elements = Vec::new();
    elements.push(0x0B);
    elements.extend(element_view_cstring(b"re"));
    elements.extend(element_view_cstring(b"\xFFp"));
    elements.extend(element_view_cstring(b"i"));
    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();
    let element = doc.iter_element_views().next().unwrap().unwrap();
    assert!(element.value().is_err());
    match element.value_view().unwrap() {
        RawBsonRefView::RegularExpression { pattern, options } => {
            assert_eq!(pattern, b"\xFFp");
            assert_eq!(options, b"i");
        }
        other => panic!("expected regex, got {other:?}"),
    }

    // dbpointer with invalid utf-8 namespace
    let mut elements = Vec::new();
    elements.push(0x0C);
    elements.extend(element_view_cstring(b"dp"));
    elements.extend(element_view_len_encode(b"\xFFns"));
    elements.extend([9u8; 12]);
    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();
    let element = doc.iter_element_views().next().unwrap().unwrap();
    assert!(element.value().is_err());
    match element.value_view().unwrap() {
        RawBsonRefView::DbPointer { namespace, id } => {
            assert_eq!(namespace, b"\xFFns");
            assert_eq!(id, ObjectId::from_bytes([9u8; 12]));
        }
        other => panic!("expected dbpointer, got {other:?}"),
    }
}

/// The decode loop a driver codec would write on top of the view API: no BSON framing logic is
/// duplicated on the caller side, and string decoding is entirely the caller's policy.
#[test]
fn element_view_iter_decode_loop() {
    // {"k\xFF": "v\xFF", "nested": {"sub": 1}, "arr": [2], "code": "c\xFF"}
    let mut nested_elements = Vec::new();
    nested_elements.push(0x10);
    nested_elements.extend(element_view_cstring(b"sub"));
    nested_elements.extend(1i32.to_le_bytes());
    let nested = element_view_doc_bytes(&nested_elements);
    let mut arr_elements = Vec::new();
    arr_elements.push(0x10);
    arr_elements.extend(element_view_cstring(b"0"));
    arr_elements.extend(2i32.to_le_bytes());
    let arr = element_view_doc_bytes(&arr_elements);
    let mut elements = Vec::new();
    elements.push(0x02);
    elements.extend(element_view_cstring(b"k\xFF"));
    elements.extend(element_view_len_encode(b"v\xFF"));
    elements.push(0x03);
    elements.extend(element_view_cstring(b"nested"));
    elements.extend(&nested);
    elements.push(0x04);
    elements.extend(element_view_cstring(b"arr"));
    elements.extend(&arr);
    elements.push(0x0D);
    elements.extend(element_view_cstring(b"code"));
    elements.extend(element_view_len_encode(b"c\xFF"));

    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();

    // a caller-chosen decode policy: lossy for strings, raw for keys
    fn decode(bytes: &[u8]) -> String {
        String::from_utf8_lossy(bytes).into_owned()
    }

    let mut out = std::collections::BTreeMap::new();
    let mut dollar_key = false;
    for element in doc.iter_element_views() {
        let element = element.unwrap();
        let key = element.key_bytes();
        dollar_key |= key.first() == Some(&b'$');
        let value = match element.value_view().unwrap() {
            RawBsonRefView::String(payload) => format!("string:{}", decode(payload)),
            RawBsonRefView::JavaScriptCode(payload) => format!("code:{}", decode(payload)),
            RawBsonRefView::Document(nested) => {
                let mut iter = nested.iter_element_views();
                let element = iter.next().unwrap().unwrap();
                assert!(iter.next().is_none());
                format!("doc:{{{:?}}}", element.value_view().unwrap(),)
            }
            RawBsonRefView::Array(array) => {
                let mut iter = array.iter_element_views();
                let element = iter.next().unwrap().unwrap();
                assert!(iter.next().is_none());
                format!(
                    "arr:[{}]",
                    match element.value_view().unwrap() {
                        RawBsonRefView::Int32(v) => v.to_string(),
                        other => panic!("unexpected {other:?}"),
                    }
                )
            }
            other => panic!("unexpected {other:?}"),
        };
        out.insert(decode(key), value);
    }

    assert!(!dollar_key);
    assert_eq!(out.get("k\u{FFFD}").unwrap(), "string:v\u{FFFD}");
    assert_eq!(out.get("nested").unwrap(), "doc:{Int32(1)}");
    assert_eq!(out.get("arr").unwrap(), "arr:[2]");
    assert_eq!(out.get("code").unwrap(), "code:c\u{FFFD}");
}

#[test]
fn element_view_iter_rejects_element_overrunning_terminator() {
    // {"k": "v"} where the string's null terminator doubles as the document's, so the element
    // consumes every byte of the buffer and the iteration overruns the terminating null
    let bytes: &[u8] = b"\x0d\0\0\0\x02k\0\x02\0\0\0v\0";
    let doc = RawDocument::from_bytes(bytes).unwrap();
    let mut iter = doc.iter_element_views();
    assert_eq!(
        iter.next().unwrap().unwrap().value().unwrap().as_str(),
        Some("v")
    );
    let err = iter.next().unwrap().unwrap_err();
    assert!(err.to_string().contains("iteration overflowed document"));
    assert!(iter.next().is_none());
}

#[test]
fn raw_value_parse_rejects_string_missing_terminator() {
    // a string value whose final byte is not the null terminator must be an error;
    // the strict path historically dropped the trailing byte silently
    let mut value = element_view_len_encode(b"payload");
    *value.last_mut().unwrap() = 0xFF;
    let error = value::RawValue::new_at(ElementType::String, &value, 0)
        .parse()
        .unwrap_err();
    assert!(matches!(error.kind, ErrorKind::MalformedBytes { .. }));
}

#[test]
fn value_lossy_regex_locates_options_with_raw_pattern_length() {
    // a regex pattern containing invalid utf-8: the options cstring must be located using
    // the raw pattern bytes; using the lossily decoded length (three bytes per replacement
    // character) shifts the offset past the options
    let mut value = Vec::new();
    value.extend(element_view_cstring(b"b\xC3")); // pattern: "b" + invalid continuation byte
    value.extend(element_view_cstring(b"x")); // options

    let lossy = value::RawValue::new_at(ElementType::RegularExpression, &value, 0)
        .parse_utf8_lossy()
        .unwrap()
        .unwrap();
    match lossy {
        Utf8LossyBson::RegularExpression(regex) => {
            assert_eq!(regex.pattern.as_str(), "b\u{FFFD}");
            assert_eq!(regex.options.as_str(), "x");
        }
        _ => panic!("expected regex"),
    }
}

#[test]
fn element_view_into_validated_matches_strict_parse() {
    // validating the view's framing must produce the same value as the strict parse for
    // every type, string-like and otherwise
    let mut scope_elements = Vec::new();
    scope_elements.push(0x10);
    scope_elements.extend(element_view_cstring(b"x"));
    scope_elements.extend(1i32.to_le_bytes());
    let scope = element_view_doc_bytes(&scope_elements);
    let code = element_view_len_encode(b"var");

    let mut elements = Vec::new();
    for (tag, key, value) in [
        (0x02, &b"s"[..], element_view_len_encode(b"hello")), // string
        (0x0E, &b"y"[..], element_view_len_encode(b"sym")),   // symbol
        (0x0D, &b"c"[..], element_view_len_encode(b"code")),  // javascript code
        (0x0B, &b"re"[..], {
            let mut v = element_view_cstring(b"pat");
            v.extend(element_view_cstring(b"i"));
            v
        }), // regular expression
        (0x0C, &b"dp"[..], {
            let mut v = element_view_len_encode(b"ns");
            v.extend([0u8; 12]); // object id
            v
        }), // dbpointer
        (0x10, &b"i"[..], 42i32.to_le_bytes().to_vec()),      // int32
    ] {
        let mut element = Vec::new();
        element.push(tag);
        element.extend(element_view_cstring(key));
        element.extend(value);
        elements.extend(element);
    }
    let mut element = Vec::new();
    element.push(0x0F);
    element.extend(element_view_cstring(b"cws"));
    element.extend(((4 + code.len() + scope.len()) as i32).to_le_bytes());
    element.extend(&code);
    element.extend(&scope);
    elements.extend(element);

    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();
    for element in doc.iter_element_views() {
        let element = element.unwrap();
        assert_eq!(
            element.value_view().unwrap().into_validated().unwrap(),
            element.value().unwrap(),
        );
    }
}

#[test]
fn element_view_into_validated_rejects_invalid_utf8() {
    // the view hands string-like payloads back as raw bytes; the strict conversion is the
    // utf-8 validation policy
    let mut elements = Vec::new();
    elements.push(0x02);
    elements.extend(element_view_cstring(b"s"));
    elements.extend(element_view_len_encode(b"h\xFFllo"));

    let bytes = element_view_doc_bytes(&elements);
    let doc = RawDocument::from_bytes(&bytes).unwrap();
    let element = doc.iter_element_views().next().unwrap().unwrap();
    let error = element.value_view().unwrap().into_validated().unwrap_err();
    assert!(matches!(error.kind, ErrorKind::Utf8Encoding { .. }));
}
