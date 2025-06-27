use crate::{
    oid::ObjectId,
    raw::{cstr, RawJavaScriptCodeWithScope},
    spec::BinarySubtype,
    tests::LOCK,
    Binary,
    Bson,
    DateTime,
    DbPointer,
    Decimal128,
    Document,
    JavaScriptCodeWithScope,
    RawArrayBuf,
    RawBson,
    RawDocumentBuf,
    Regex,
    Timestamp,
};

use pretty_assertions::assert_eq;

fn append_test(expected: Document, append: impl FnOnce(&mut RawDocumentBuf)) {
    let bytes = expected.encode_to_vec().unwrap();
    let mut buf = RawDocumentBuf::new();
    append(&mut buf);
    assert_eq!(buf.as_bytes(), bytes);
}

#[test]
fn i32() {
    let expected = doc! {
        "a": -1_i32,
        "b": 123_i32,
        "c": 0_i32
    };
    append_test(expected, |doc| {
        doc.append(cstr!("a"), -1_i32);
        doc.append(cstr!("b"), 123_i32);
        doc.append(cstr!("c"), 0_i32);
    });
}

#[test]
fn i64() {
    let expected = doc! {
        "a": -1_i64,
        "b": 123_i64,
        "c": 0_i64
    };
    append_test(expected, |doc| {
        doc.append(cstr!("a"), -1_i64);
        doc.append(cstr!("b"), 123_i64);
        doc.append(cstr!("c"), 0_i64);
    });
}

#[test]
fn str() {
    let expected = doc! {
        "first": "the quick",
        "second": "brown fox",
        "third": "jumped over",
        "last": "the lazy sheep dog",
    };
    append_test(expected, |doc| {
        doc.append(cstr!("first"), "the quick");
        doc.append(cstr!("second"), "brown fox");
        doc.append(cstr!("third"), "jumped over");
        doc.append(cstr!("last"), "the lazy sheep dog");
    });
}

#[test]
fn double() {
    let expected = doc! {
        "positive": 12.5,
        "0": 0.0,
        "negative": -123.24,
        "nan": f64::NAN,
        "inf": f64::INFINITY,
    };
    append_test(expected, |doc| {
        doc.append(cstr!("positive"), 12.5);
        doc.append(cstr!("0"), 0.0);
        doc.append(cstr!("negative"), -123.24);
        doc.append(cstr!("nan"), f64::NAN);
        doc.append(cstr!("inf"), f64::INFINITY);
    });
}

#[test]
fn boolean() {
    let expected = doc! {
        "true": true,
        "false": false,
    };
    append_test(expected, |doc| {
        doc.append(cstr!("true"), true);
        doc.append(cstr!("false"), false);
    });
}

#[test]
fn null() {
    let expected = doc! {
        "null": null,
    };
    append_test(expected, |doc| doc.append(cstr!("null"), RawBson::Null));
}

#[test]
fn document() {
    let expected = doc! {
        "empty": {},
        "subdoc": {
            "a": 1_i32,
            "b": true,
        }
    };
    append_test(expected, |doc| {
        doc.append(cstr!("empty"), RawDocumentBuf::new());
        let mut buf = RawDocumentBuf::new();
        buf.append(cstr!("a"), 1_i32);
        buf.append(cstr!("b"), true);
        doc.append(cstr!("subdoc"), buf);
    });
}

#[test]
fn array() {
    let expected = doc! {
        "empty": [],
        "array": [
            true,
            "string",
            { "a": "subdoc" },
            123_i32
        ]
    };
    append_test(expected, |doc| {
        doc.append(cstr!("empty"), RawArrayBuf::new());
        let mut buf = RawArrayBuf::new();
        buf.push(true);
        buf.push("string");
        let mut subdoc = RawDocumentBuf::new();
        subdoc.append(cstr!("a"), "subdoc");
        buf.push(subdoc);
        buf.push(123_i32);
        doc.append(cstr!("array"), buf);
    });
}

#[test]
fn oid() {
    let _guard = LOCK.run_concurrently();

    let oid = ObjectId::new();
    let expected = doc! {
        "oid": oid,
    };
    append_test(expected, |doc| doc.append(cstr!("oid"), oid));
}

#[test]
fn datetime() {
    let dt = DateTime::now();
    let old = DateTime::from_millis(-1);

    let expected = doc! {
        "now": dt,
        "old": old
    };

    append_test(expected, |doc| {
        doc.append(cstr!("now"), dt);
        doc.append(cstr!("old"), old);
    });
}

#[test]
fn timestamp() {
    let ts = Timestamp {
        time: 123,
        increment: 2,
    };

    let expected = doc! {
        "ts": ts,
    };

    append_test(expected, |doc| doc.append(cstr!("ts"), ts));
}

#[test]
fn binary() {
    let bytes = vec![1, 2, 3, 4];

    let bin = Binary {
        bytes: bytes.clone(),
        subtype: BinarySubtype::Generic,
    };

    let old = Binary {
        bytes,
        subtype: BinarySubtype::BinaryOld,
    };

    let expected = doc! {
        "generic": bin.clone(),
        "binary_old": old.clone(),
    };

    append_test(expected, |doc| {
        doc.append(cstr!("generic"), bin);
        doc.append(cstr!("binary_old"), old);
    });
}

#[test]
fn min_max_key() {
    let expected = doc! {
        "min": Bson::MinKey,
        "max": Bson::MaxKey
    };

    append_test(expected, |doc| {
        doc.append(cstr!("min"), RawBson::MinKey);
        doc.append(cstr!("max"), RawBson::MaxKey);
    });
}

#[test]
fn undefined() {
    let expected = doc! {
        "undefined": Bson::Undefined,
    };

    append_test(expected, |doc| {
        doc.append(cstr!("undefined"), RawBson::Undefined)
    });
}

#[test]
fn regex() {
    let expected = doc! {
        "regex": Regex::new("some pattern", "abc").unwrap(),
    };

    append_test(expected, |doc| {
        doc.append(cstr!("regex"), Regex::new("some pattern", "abc").unwrap())
    });
}

#[test]
fn code() {
    let code_w_scope = JavaScriptCodeWithScope {
        code: "some code".to_string(),
        scope: doc! { "a": 1_i32, "b": true },
    };

    let expected = doc! {
        "code": Bson::JavaScriptCode("some code".to_string()),
        "code_w_scope": code_w_scope,
    };

    append_test(expected, |doc| {
        doc.append(
            cstr!("code"),
            RawBson::JavaScriptCode("some code".to_string()),
        );

        let mut scope = RawDocumentBuf::new();
        scope.append(cstr!("a"), 1_i32);
        scope.append(cstr!("b"), true);
        doc.append(
            cstr!("code_w_scope"),
            RawJavaScriptCodeWithScope {
                code: "some code".to_string(),
                scope,
            },
        );
    });
}

#[test]
fn symbol() {
    let expected = doc! {
        "symbol": Bson::Symbol("symbol".to_string())
    };

    append_test(expected, |doc| {
        doc.append(cstr!("symbol"), RawBson::Symbol("symbol".to_string()))
    });
}

#[test]
fn dbpointer() {
    let _guard = LOCK.run_concurrently();

    let id = ObjectId::new();

    let expected = doc! {
        "symbol": Bson::DbPointer(DbPointer {
            namespace: "ns".to_string(),
            id
        })
    };

    append_test(expected, |doc| {
        doc.append(
            cstr!("symbol"),
            RawBson::DbPointer(DbPointer {
                namespace: "ns".to_string(),
                id,
            }),
        )
    });
}

#[test]
fn decimal128() {
    let decimal = Decimal128 { bytes: [1; 16] };
    let expected = doc! {
        "decimal": decimal
    };

    append_test(expected, |doc| doc.append(cstr!("decimal"), decimal));
}

#[test]
fn general() {
    let dt = DateTime::now();
    let expected = doc! {
        "a": true,
        "second key": 123.4,
        "third": 15_i64,
        "32": -100101_i32,
        "subdoc": {
            "a": "subkey",
            "another": { "subdoc": dt }
        },
        "array": [1_i64, true, { "doc": 23_i64 }, ["another", "array"]],
    };

    append_test(expected, |doc| {
        doc.append(cstr!("a"), true);
        doc.append(cstr!("second key"), 123.4);
        doc.append(cstr!("third"), 15_i64);
        doc.append(cstr!("32"), -100101_i32);

        let mut subdoc = RawDocumentBuf::new();
        subdoc.append(cstr!("a"), "subkey");

        let mut subsubdoc = RawDocumentBuf::new();
        subsubdoc.append(cstr!("subdoc"), dt);
        subdoc.append(cstr!("another"), subsubdoc);
        doc.append(cstr!("subdoc"), subdoc);

        let mut array = RawArrayBuf::new();
        array.push(1_i64);
        array.push(true);

        let mut array_subdoc = RawDocumentBuf::new();
        array_subdoc.append(cstr!("doc"), 23_i64);
        array.push(array_subdoc);

        let mut sub_array = RawArrayBuf::new();
        sub_array.push("another");
        sub_array.push("array");
        array.push(sub_array);

        doc.append(cstr!("array"), array);
    });
}

#[test]
fn from_iter() {
    let doc_buf = RawDocumentBuf::from_iter([
        (
            cstr!("array"),
            RawBson::Array(RawArrayBuf::from_iter([
                RawBson::Boolean(true),
                RawBson::Document(RawDocumentBuf::from_iter([
                    (cstr!("ok"), RawBson::Boolean(false)),
                    (cstr!("other"), RawBson::String("hello".to_string())),
                ])),
            ])),
        ),
        (cstr!("bool"), RawBson::Boolean(true)),
        (cstr!("string"), RawBson::String("some string".to_string())),
    ]);

    let doc = doc! {
        "array": [
            true,
            {
                "ok": false,
                "other": "hello"
            }
        ],
        "bool": true,
        "string": "some string"
    };

    let expected = doc! { "expected": doc };
    append_test(expected, |doc| doc.append(cstr!("expected"), doc_buf));
}

#[test]
fn array_buf() {
    let mut arr_buf = RawArrayBuf::new();
    arr_buf.push(true);

    let mut doc_buf = RawDocumentBuf::new();
    doc_buf.append(cstr!("x"), 3_i32);
    doc_buf.append(cstr!("string"), "string");
    arr_buf.push(doc_buf);

    let mut sub_arr = RawArrayBuf::new();
    sub_arr.push("a string");
    arr_buf.push(sub_arr);

    let arr = rawbson!([
        true,
        { "x": 3_i32, "string": "string" },
        [ "a string" ]
    ]);

    assert_eq!(arr_buf.as_ref(), arr.as_array().unwrap());
}
