use crate::{
    oid::ObjectId,
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
    RawBinary,
    RawBson,
    RawDbPointer,
    RawDocument,
    RawDocumentBuf,
    RawJavaScriptCodeWithScope,
    RawRegex,
    Regex,
    Timestamp,
};

use pretty_assertions::assert_eq;

fn append_test(expected: Document, append: impl FnOnce(&mut RawDocumentBuf)) {
    let bytes = crate::to_vec(&expected).unwrap();
    let mut buf = RawDocumentBuf::empty();
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
        doc.append("a", -1_i32);
        doc.append("b", 123_i32);
        doc.append("c", 0_i32);
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
        doc.append("a", -1_i64);
        doc.append("b", 123_i64);
        doc.append("c", 0_i64);
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
        doc.append("first", "the quick");
        doc.append("second", "brown fox");
        doc.append("third", "jumped over");
        doc.append("last", "the lazy sheep dog");
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
        doc.append("positive", 12.5);
        doc.append("0", 0.0);
        doc.append("negative", -123.24);
        doc.append("nan", f64::NAN);
        doc.append("inf", f64::INFINITY);
    });
}

#[test]
fn boolean() {
    let expected = doc! {
        "true": true,
        "false": false,
    };
    append_test(expected, |doc| {
        doc.append("true", true);
        doc.append("false", false);
    });
}

#[test]
fn null() {
    let expected = doc! {
        "null": null,
    };
    append_test(expected, |doc| {
        doc.append("null", RawBson::Null);
    });
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
        doc.append("empty", &RawDocumentBuf::empty());
        let mut buf = RawDocumentBuf::empty();
        buf.append("a", 1_i32);
        buf.append("b", true);
        doc.append("subdoc", &buf);
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
        doc.append("empty", &RawArrayBuf::new());
        let mut buf = RawArrayBuf::new();
        buf.append(true);
        buf.append("string");
        let mut subdoc = RawDocumentBuf::empty();
        subdoc.append("a", "subdoc");
        buf.append(&subdoc);
        buf.append(123_i32);
        doc.append("array", &buf);
    });
}

#[test]
fn oid() {
    let _guard = LOCK.run_concurrently();

    let oid = ObjectId::new();
    let expected = doc! {
        "oid": oid,
    };
    append_test(expected, |doc| doc.append("oid", oid));
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
        doc.append("now", dt);
        doc.append("old", old);
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

    append_test(expected, |doc| {
        doc.append("ts", ts);
    });
}

#[test]
fn binary() {
    let bytes = vec![1, 2, 3, 4];

    let bin = Binary {
        bytes: bytes.clone(),
        subtype: BinarySubtype::Generic,
    };

    let old = Binary {
        bytes: bytes.clone(),
        subtype: BinarySubtype::BinaryOld,
    };

    let expected = doc! {
        "generic": bin.clone(),
        "binary_old": old.clone(),
    };

    append_test(expected, |doc| {
        doc.append("generic", &bin);
        doc.append("binary_old", &old);
    });
}

#[test]
fn min_max_key() {
    let expected = doc! {
        "min": Bson::MinKey,
        "max": Bson::MaxKey
    };

    append_test(expected, |doc| {
        doc.append("min", RawBson::MinKey);
        doc.append("max", RawBson::MaxKey);
    });
}

#[test]
fn undefined() {
    let expected = doc! {
        "undefined": Bson::Undefined,
    };

    append_test(expected, |doc| {
        doc.append("undefined", RawBson::Undefined);
    });
}

#[test]
fn regex() {
    let expected = doc! {
        "regex": Regex::new("some pattern", "abc"),
    };

    append_test(expected, |doc| {
        doc.append(
            "regex",
            RawRegex {
                pattern: "some pattern",
                options: "abc",
            },
        )
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
        doc.append("code", RawBson::JavaScriptCode("some code"));

        let mut scope = RawDocumentBuf::empty();
        scope.append("a", 1_i32);
        scope.append("b", true);
        doc.append(
            "code_w_scope",
            RawJavaScriptCodeWithScope {
                code: "some code",
                scope: &scope,
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
        doc.append("symbol", RawBson::Symbol("symbol"));
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
            "symbol",
            RawBson::DbPointer(RawDbPointer {
                namespace: "ns",
                id,
            }),
        );
    });
}

#[test]
fn decimal128() {
    let decimal = Decimal128 { bytes: [1; 16] };
    let expected = doc! {
        "decimal": decimal
    };

    append_test(expected, |doc| {
        doc.append("decimal", decimal);
    });
}
