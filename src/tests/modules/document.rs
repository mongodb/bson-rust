use crate::{
    doc,
    oid::ObjectId,
    spec::BinarySubtype,
    tests::LOCK,
    Binary,
    Bson,
    Document,
    Timestamp,
};
use time::OffsetDateTime;

#[test]
fn ordered_insert() {
    let _guard = LOCK.run_concurrently();
    let mut doc = Document::new();
    doc.insert("first".to_owned(), Bson::Int32(1));
    doc.insert("second".to_owned(), Bson::String("foo".to_owned()));
    doc.insert("alphanumeric".to_owned(), Bson::String("bar".to_owned()));

    let expected_keys = vec![
        "first".to_owned(),
        "second".to_owned(),
        "alphanumeric".to_owned(),
    ];

    let keys: Vec<_> = doc.iter().map(|(key, _)| key.to_owned()).collect();
    assert_eq!(expected_keys, keys);
}

#[test]
fn ordered_insert_shorthand() {
    let _guard = LOCK.run_concurrently();
    let mut doc = Document::new();
    doc.insert("first", 1i32);
    doc.insert("second", "foo");
    doc.insert("alphanumeric", "bar".to_owned());

    let expected_keys = vec![
        "first".to_owned(),
        "second".to_owned(),
        "alphanumeric".to_owned(),
    ];

    let keys: Vec<_> = doc.iter().map(|(key, _)| key.to_owned()).collect();
    assert_eq!(expected_keys, keys);
}

#[test]
fn test_getters() {
    let _guard = LOCK.run_concurrently();
    let datetime = OffsetDateTime::now_utc();
    let cloned_dt = crate::DateTime::from_time_0_3(datetime);
    let binary = vec![0, 1, 2, 3, 4];
    let mut doc = doc! {
        "floating_point": 10.0,
        "string": "a value",
        "array": [10, 20, 30],
        "doc": { "key": 1 },
        "bool": true,
        "i32": 1i32,
        "i64": 1i64,
        "datetime": cloned_dt,
        "binary": Binary { subtype: BinarySubtype::Generic, bytes: binary.clone() }
    };

    assert_eq!(None, doc.get("nonsense"));
    assert!(doc
        .get_str("nonsense")
        .unwrap_err()
        .is_value_access_not_present());
    assert!(doc
        .get_str("floating_point")
        .unwrap_err()
        .is_value_access_unexpected_type());

    assert_eq!(Some(&Bson::Double(10.0)), doc.get("floating_point"));
    assert_eq!(10.0, doc.get_f64("floating_point").unwrap());

    assert_eq!(
        Some(&Bson::String("a value".to_string())),
        doc.get("string")
    );
    assert_eq!("a value", doc.get_str("string").unwrap());

    let array = vec![Bson::Int32(10), Bson::Int32(20), Bson::Int32(30)];
    assert_eq!(Some(&Bson::Array(array.clone())), doc.get("array"));
    assert_eq!(&array, doc.get_array("array").unwrap());

    let embedded = doc! { "key": 1 };
    assert_eq!(Some(&Bson::Document(embedded.clone())), doc.get("doc"));
    assert_eq!(&embedded, doc.get_document("doc").unwrap());

    assert_eq!(Some(&Bson::Boolean(true)), doc.get("bool"));
    assert!(doc.get_bool("bool").unwrap());

    doc.insert("null".to_string(), Bson::Null);
    assert_eq!(Some(&Bson::Null), doc.get("null"));
    assert_eq!(doc.get_null("null").unwrap(), Bson::Null);
    assert!(doc.get_null("array").is_err());

    assert_eq!(Some(&Bson::Int32(1)), doc.get("i32"));
    assert_eq!(1i32, doc.get_i32("i32").unwrap());

    assert_eq!(Some(&Bson::Int64(1)), doc.get("i64"));
    assert_eq!(1i64, doc.get_i64("i64").unwrap());

    doc.insert(
        "timestamp".to_string(),
        Bson::Timestamp(Timestamp {
            time: 0,
            increment: 100,
        }),
    );
    assert_eq!(
        Some(&Bson::Timestamp(Timestamp {
            time: 0,
            increment: 100
        })),
        doc.get("timestamp")
    );
    assert_eq!(
        Timestamp {
            time: 0,
            increment: 100,
        },
        doc.get_timestamp("timestamp").unwrap()
    );

    let dt = crate::DateTime::from_time_0_3(datetime);
    assert_eq!(Some(&Bson::DateTime(dt)), doc.get("datetime"));
    assert_eq!(&dt, doc.get_datetime("datetime").unwrap());

    let object_id = ObjectId::new();
    doc.insert("_id".to_string(), Bson::ObjectId(object_id));
    assert_eq!(Some(&Bson::ObjectId(object_id)), doc.get("_id"));
    assert_eq!(object_id, doc.get_object_id("_id").unwrap());

    assert_eq!(
        Some(&Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: binary.clone()
        })),
        doc.get("binary")
    );
    assert_eq!(&binary, doc.get_binary_generic("binary").unwrap());
}

#[test]
fn remove() {
    let _guard = LOCK.run_concurrently();

    let mut doc = Document::new();
    doc.insert("first", 1i32);
    doc.insert("second", "foo");
    doc.insert("third", "bar".to_owned());
    doc.insert("fourth", "bar".to_owned());

    let mut expected_keys = vec![
        "first".to_owned(),
        "second".to_owned(),
        "third".to_owned(),
        "fourth".to_owned(),
    ];

    let keys: Vec<_> = doc.iter().map(|(key, _)| key.to_owned()).collect();
    assert_eq!(expected_keys, keys);

    assert_eq!(doc.remove("none"), None);

    assert!(doc.remove("second").is_some());
    expected_keys.remove(1);
    let keys: Vec<_> = doc.iter().map(|(key, _)| key.to_owned()).collect();
    assert_eq!(keys, expected_keys);

    assert!(doc.remove("first").is_some());
    expected_keys.remove(0);
    let keys: Vec<_> = doc.iter().map(|(key, _)| key.to_owned()).collect();
    assert_eq!(keys, expected_keys);
}

#[test]
fn entry() {
    let _guard = LOCK.run_concurrently();
    let mut doc = doc! {
        "first": 1i32,
        "second": "foo",
        "alphanumeric": "bar",
    };

    {
        let first_entry = doc.entry("first".to_owned());
        assert_eq!(first_entry.key(), "first");

        let v = first_entry.or_insert_with(|| {
            Bson::Timestamp(Timestamp {
                time: 0,
                increment: 27,
            })
        });
        assert_eq!(v, &mut Bson::Int32(1));
    }

    {
        let fourth_entry = doc.entry("fourth".to_owned());
        assert_eq!(fourth_entry.key(), "fourth");

        let v = fourth_entry.or_insert(Bson::Null);
        assert_eq!(v, &mut Bson::Null);
    }

    assert_eq!(
        doc,
        doc! {
            "first": 1i32,
            "second": "foo",
            "alphanumeric": "bar",
            "fourth": Bson::Null,
        },
    );
}

#[test]
fn extend() {
    let _guard = LOCK.run_concurrently();
    let mut doc1 = doc! {
        "first": 1,
        "second": "data",
        "subdoc": doc! { "a": 1, "b": 2 },
    };

    let doc2 = doc! {
        "third": "abcdefg",
        "first": 2,
        "subdoc": doc! { "c": 3 },
    };

    doc1.extend(doc2);

    assert_eq!(
        doc1,
        doc! {
            "first": 2,
            "second": "data",
            "third": "abcdefg",
            "subdoc": doc! { "c": 3 },
        },
    );
}

#[test]
fn test_display_empty_doc() {
    let empty_expectation = "{}";
    let doc = doc! {};
    let doc_display = format!("{doc}");
    assert_eq!(empty_expectation, doc_display);

    let doc_display_pretty = format!("{doc:#}");
    assert_eq!(empty_expectation, doc_display_pretty);
}

#[test]
fn test_display_doc() {
    let doc = doc! {
        "hello": "world"
    };

    let doc_display_expectation = "{ \"hello\": \"world\" }";
    assert_eq!(doc_display_expectation, format!("{doc}"));

    let doc_display_pretty_expectation = r#"{
  "hello": "world"
}"#;
    assert_eq!(doc_display_pretty_expectation, format!("{doc:#}"));
}

#[test]
fn test_display_nested_doc() {
    let doc = doc! {
        "hello": {
            "hello": 2
        }
    };

    let doc_display_expectation = "{ \"hello\": { \"hello\": 2 } }";
    assert_eq!(doc_display_expectation, format!("{doc}"));

    let doc_display_pretty_expectation = r#"{
  "hello": {
    "hello": 2
  }
}"#;
    let formatted = format!("{doc:#}");
    assert_eq!(doc_display_pretty_expectation, formatted);
}

#[test]
fn test_display_doc_with_array() {
    let doc = doc! {
        "hello": [1, 2, 3]
    };

    let doc_display_expectation = "{ \"hello\": [1, 2, 3] }";
    assert_eq!(doc_display_expectation, format!("{doc}"));

    let doc_display_pretty_expectation = "{\n  \"hello\": [\n    1, \n    2, \n    3\n  ]\n}";
    let formatted = format!("{doc:#}");
    assert_eq!(doc_display_pretty_expectation, formatted);

    let nested_array_doc = doc! {
        "a": [1, [1, 2]]
    };

    let expectation = "{\n  \"a\": [\n    1, \n    [\n      1, \n      2\n    ]\n  ]\n}";
    assert_eq!(expectation, format!("{nested_array_doc:#}"));
}

#[test]
fn test_pretty_printing() {
    let d = doc! { "hello": "world!", "world": "hello", "key": "val" };
    let expected = r#"{ "hello": "world!", "world": "hello", "key": "val" }"#;
    let formatted = format!("{d}");
    assert_eq!(
        expected, formatted,
        "expected:\n{expected}\ngot:\n{formatted}"
    );

    let d = doc! { "hello": "world!", "nested": { "key": "val", "double": { "a": "thing" } } };
    #[rustfmt::skip]
    let expected =  "{\n  \"hello\": \"world!\",\n  \"nested\": {\n    \"key\": \"val\",\n    \"double\": {\n      \"a\": \"thing\"\n    }\n  }\n}";
    let formatted = format!("{d:#}");
    assert_eq!(formatted, expected);

    let d =
        doc! { "hello": "world!", "nested": { "key": "val", "double": { "a": [1, 2], "c": "d"} } };
    #[rustfmt::skip]
    let expected = "{\n  \"hello\": \"world!\",\n  \"nested\": {\n    \"key\": \"val\",\n    \"double\": {\n      \"a\": [\n        1, \n        2\n      ],\n      \"c\": \"d\"\n    }\n  }\n}";
    assert_eq!(format!("{d:#}"), expected);
}

#[test]
fn test_indexing() {
    let d = doc! {"x": 1};
    let val = d["x"].as_i32().unwrap();
    assert_eq!(val, 1);

    let d = doc! {"x": {"y": 100}};
    let val = d["x"]["y"].as_i32().unwrap();
    assert_eq!(val, 100);

    let d = doc! {"x" : true};
    let val = d["x"].as_bool().unwrap();
    assert!(val);

    let d = doc! {"x": "y"};
    let val = d["x"].as_str().unwrap();
    assert_eq!(val, "y");

    let d = doc! {"x": 1.9};
    let val = d["x"].as_f64().unwrap();
    assert_eq!(val, 1.9);

    let d = doc! {"x": [1, 2, 3]};
    let val = d["x"].as_array().unwrap();
    assert_eq!(val.len(), 3);
}

#[test]
fn test_indexing_key_not_found() {
    let d = doc! {"x": 1};
    let val = &d["y"];
    assert!(val.as_null().is_some());

    let d = doc! {"x": {"y": 1}};
    let val = &d["x"]["z"];
    assert!(val.as_null().is_some());
}

#[test]
fn test_indexing_on_wrong_bson_type() {
    let d = doc! {"x": {"y": 1}};
    let val = &d["x"]["y"]["z"];
    assert!(val.as_null().is_some());
}
