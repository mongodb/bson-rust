use pretty_assertions::assert_eq;
use serde_json::json;

use serde::{Deserialize, Serialize};

use crate::{cstr, doc, Bson, JavaScriptCodeWithScope, RawArrayBuf, RawBson, RawDocumentBuf};

use super::util::AllTypes;

#[test]
fn all_types_json() {
    let (mut v, _) = AllTypes::fixtures();

    let code = match v.code {
        Bson::JavaScriptCode(ref c) => c.clone(),
        c => panic!("expected code, found {:?}", c),
    };

    let code_w_scope = JavaScriptCodeWithScope {
        code: "hello world".to_string(),
        scope: doc! { "x": 1 },
    };
    let scope_json = serde_json::json!({ "x": 1 });
    v.code_w_scope = code_w_scope.clone();

    let json = serde_json::json!({
        "x": 1,
        "y": 2,
        "s": "oke",
        "array": vec![
            serde_json::json!(true),
            serde_json::json!("oke".to_string()),
            serde_json::json!({ "12": 24 }),
        ],
        "bson": 1234.5,
        "oid": { "$oid": v.oid.to_hex() },
        "null": serde_json::Value::Null,
        "subdoc": { "k": true, "b": { "hello": "world" } },
        "b": true,
        "d": 12.5,
        "binary": v.binary.bytes,
        "binary_old": { "$binary": { "base64": crate::base64::encode(&v.binary_old.bytes), "subType": "02" } },
        "binary_other": { "$binary": { "base64": crate::base64::encode(&v.binary_old.bytes), "subType": "81" } },
        "date": { "$date": { "$numberLong": v.date.timestamp_millis().to_string() } },
        "regex": { "$regularExpression": { "pattern": v.regex.pattern, "options": v.regex.options } },
        "ts": { "$timestamp": { "t": 123, "i": 456 } },
        "i": { "a": v.i.a, "b": v.i.b },
        "undefined": { "$undefined": true },
        "code": { "$code": code },
        "code_w_scope": { "$code": code_w_scope.code, "$scope": scope_json },
        "decimal": { "$numberDecimal": v.decimal.to_string() },
        "symbol": { "$symbol": "ok" },
        "min_key": { "$minKey": 1 },
        "max_key": { "$maxKey": 1 },
    });

    assert_eq!(serde_json::to_value(&v).unwrap(), json);
}

#[test]
fn owned_raw_bson() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Foo {
        doc_buf: RawDocumentBuf,
        array_buf: RawArrayBuf,
        bson_array: RawBson,
        bson_doc: RawBson,
        bson_integer: RawBson,
        bson_string: RawBson,
        bson_bool: RawBson,
        bson_null: RawBson,
        bson_float: RawBson,
    }

    let json = json!({
        "doc_buf": {
            "a": "key",
            "number": 12,
            "bool": false,
            "nu": null
        },
        "array_buf": [
            json!(1),
            json!("string"),
        ],
        "bson_array": [
            json!(1),
            json!("string"),
        ],
        "bson_doc": {
            "first": true,
            "second": "string",
        },
        "bson_integer": 12,
        "bson_string": "String",
        "bson_bool": true,
        "bson_null": null,
        "bson_float": 123.4
    });

    let mut doc_buf = RawDocumentBuf::new();
    doc_buf.append(cstr!("a"), "key");
    doc_buf.append(cstr!("number"), 12);
    doc_buf.append(cstr!("bool"), false);
    doc_buf.append(cstr!("nu"), RawBson::Null);

    let mut array_buf = RawArrayBuf::new();
    array_buf.push(1);
    array_buf.push("string");

    let mut bson_doc = RawDocumentBuf::new();
    bson_doc.append(cstr!("first"), true);
    bson_doc.append(cstr!("second"), "string");

    let expected = Foo {
        doc_buf,
        array_buf: array_buf.clone(),
        bson_array: RawBson::Array(array_buf),
        bson_doc: RawBson::Document(bson_doc),
        bson_integer: RawBson::Int32(12),
        bson_string: RawBson::String("String".to_string()),
        bson_bool: RawBson::Boolean(true),
        bson_null: RawBson::Null,
        bson_float: RawBson::Double(123.4),
    };

    let f: Foo = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(f, expected);

    let round_trip = serde_json::to_value(&f).unwrap();
    assert_eq!(round_trip, json);
}
