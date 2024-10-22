use crate::{
    doc,
    document::ValueAccessError,
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
    assert_eq!(Err(ValueAccessError::NotPresent), doc.get_str("nonsense"));
    assert_eq!(
        Err(ValueAccessError::UnexpectedType),
        doc.get_str("floating_point")
    );

    assert_eq!(Some(&Bson::Double(10.0)), doc.get("floating_point"));
    assert_eq!(Ok(10.0), doc.get_f64("floating_point"));

    assert_eq!(
        Some(&Bson::String("a value".to_string())),
        doc.get("string")
    );
    assert_eq!(Ok("a value"), doc.get_str("string"));

    let array = vec![Bson::Int32(10), Bson::Int32(20), Bson::Int32(30)];
    assert_eq!(Some(&Bson::Array(array.clone())), doc.get("array"));
    assert_eq!(Ok(&array), doc.get_array("array"));

    let embedded = doc! { "key": 1 };
    assert_eq!(Some(&Bson::Document(embedded.clone())), doc.get("doc"));
    assert_eq!(Ok(&embedded), doc.get_document("doc"));

    assert_eq!(Some(&Bson::Boolean(true)), doc.get("bool"));
    assert_eq!(Ok(true), doc.get_bool("bool"));

    doc.insert("null".to_string(), Bson::Null);
    assert_eq!(Some(&Bson::Null), doc.get("null"));
    assert!(doc.is_null("null"));
    assert!(!doc.is_null("array"));

    assert_eq!(Some(&Bson::Int32(1)), doc.get("i32"));
    assert_eq!(Ok(1i32), doc.get_i32("i32"));

    assert_eq!(Some(&Bson::Int64(1)), doc.get("i64"));
    assert_eq!(Ok(1i64), doc.get_i64("i64"));

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
        Ok(Timestamp {
            time: 0,
            increment: 100,
        }),
        doc.get_timestamp("timestamp")
    );

    let dt = crate::DateTime::from_time_0_3(datetime);
    assert_eq!(Some(&Bson::DateTime(dt)), doc.get("datetime"));
    assert_eq!(Ok(&dt), doc.get_datetime("datetime"));

    let object_id = ObjectId::new();
    doc.insert("_id".to_string(), Bson::ObjectId(object_id));
    assert_eq!(Some(&Bson::ObjectId(object_id)), doc.get("_id"));
    assert_eq!(Ok(object_id), doc.get_object_id("_id"));

    assert_eq!(
        Some(&Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: binary.clone()
        })),
        doc.get("binary")
    );
    assert_eq!(Ok(&binary), doc.get_binary_generic("binary"));
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

    let doc_display_pretty_expectation = r#"{
  "hello": [
    1, 
    2, 
    3
  ]
}"#;
    let formatted = format!("{doc:#}");
    assert_eq!(doc_display_pretty_expectation, formatted);

    let nested_array_doc = doc! {
        "a": [1, [1, 2]]
    };

    let expectation = r#"{
  "a": [
    1, 
    [
      1, 
      2
    ]
  ]
}"#;
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
    let expected = r#"{
  "hello": "world!",
  "nested": {
    "key": "val",
    "double": {
      "a": "thing"
    }
  }
}"#;
    let formatted = format!("{d:#}");
    assert_eq!(
        expected, formatted,
        "expected:\n{expected}\ngot:\n{formatted}"
    );

    let d =
        doc! { "hello": "world!", "nested": { "key": "val", "double": { "a": [1, 2], "c": "d"} } };
    let expected = r#"{
  "hello": "world!",
  "nested": {
    "key": "val",
    "double": {
      "a": [
        1, 
        2
      ],
      "c": "d"
    }
  }
}"#;
    assert_eq!(expected, format!("{d:#}"));
}
