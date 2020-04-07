#[cfg(feature = "decimal128")]
use bson::decimal128::Decimal128;
use bson::{doc, oid::ObjectId, spec::BinarySubtype, Bson, Document, ValueAccessError};
use chrono::Utc;

#[test]
fn ordered_insert() {
    let mut doc = Document::new();
    doc.insert("first".to_owned(), Bson::I32(1));
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

#[cfg(feature = "decimal128")]
fn test_decimal128(doc: &mut Document) {
    let dec = Decimal128::from_str("968E+1");
    doc.insert("decimal128".to_string(), Bson::Decimal128(dec.clone()));
    assert_eq!(Some(&Bson::Decimal128(dec.clone())), doc.get("decimal128"));
    assert_eq!(Ok(&dec), doc.get_decimal128("decimal128"));
}

#[cfg(not(feature = "decimal128"))]
fn test_decimal128(_doc: &mut Document) {}

#[test]
fn test_getters() {
    let datetime = Utc::now();
    let cloned_dt = datetime;
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
        "binary": (BinarySubtype::Generic, binary.clone())
    };

    assert_eq!(None, doc.get("nonsense"));
    assert_eq!(Err(ValueAccessError::NotPresent), doc.get_str("nonsense"));
    assert_eq!(
        Err(ValueAccessError::UnexpectedType),
        doc.get_str("floating_point")
    );

    assert_eq!(Some(&Bson::FloatingPoint(10.0)), doc.get("floating_point"));
    assert_eq!(Ok(10.0), doc.get_f64("floating_point"));

    assert_eq!(
        Some(&Bson::String("a value".to_string())),
        doc.get("string")
    );
    assert_eq!(Ok("a value"), doc.get_str("string"));

    let array = vec![Bson::I32(10), Bson::I32(20), Bson::I32(30)];
    assert_eq!(Some(&Bson::Array(array.clone())), doc.get("array"));
    assert_eq!(Ok(&array), doc.get_array("array"));

    let embedded = doc! { "key": 1 };
    assert_eq!(Some(&Bson::Document(embedded.clone())), doc.get("doc"));
    assert_eq!(Ok(&embedded), doc.get_document("doc"));

    assert_eq!(Some(&Bson::Boolean(true)), doc.get("bool"));
    assert_eq!(Ok(true), doc.get_bool("bool"));

    doc.insert("null".to_string(), Bson::Null);
    assert_eq!(Some(&Bson::Null), doc.get("null"));
    assert_eq!(true, doc.is_null("null"));
    assert_eq!(false, doc.is_null("array"));

    assert_eq!(Some(&Bson::I32(1)), doc.get("i32"));
    assert_eq!(Ok(1i32), doc.get_i32("i32"));

    assert_eq!(Some(&Bson::I64(1)), doc.get("i64"));
    assert_eq!(Ok(1i64), doc.get_i64("i64"));

    doc.insert("timestamp".to_string(), Bson::TimeStamp(100));
    assert_eq!(Some(&Bson::TimeStamp(100)), doc.get("timestamp"));
    assert_eq!(Ok(100i64), doc.get_time_stamp("timestamp"));

    assert_eq!(Some(&Bson::UtcDatetime(datetime)), doc.get("datetime"));
    assert_eq!(Ok(&datetime), doc.get_utc_datetime("datetime"));

    test_decimal128(&mut doc);

    assert_eq!(Some(&Bson::UtcDatetime(datetime)), doc.get("datetime"));
    assert_eq!(Ok(&datetime), doc.get_utc_datetime("datetime"));

    let object_id = ObjectId::new();
    doc.insert("_id".to_string(), Bson::ObjectId(object_id.clone()));
    assert_eq!(Some(&Bson::ObjectId(object_id.clone())), doc.get("_id"));
    assert_eq!(Ok(&object_id), doc.get_object_id("_id"));

    assert_eq!(
        Some(&Bson::Binary(BinarySubtype::Generic, binary.clone())),
        doc.get("binary")
    );
    assert_eq!(Ok(&binary), doc.get_binary_generic("binary"));
}

#[test]
fn remove() {
    let mut doc = Document::new();
    doc.insert("first", Bson::I32(1));
    doc.insert("second", Bson::String("foo".to_owned()));
    doc.insert("alphanumeric", Bson::String("bar".to_owned()));

    assert!(doc.remove("second").is_some());
    assert!(doc.remove("none").is_none());

    let expected_keys = vec!["first", "alphanumeric"];

    let keys: Vec<_> = doc.iter().map(|(key, _)| key.to_owned()).collect();
    assert_eq!(expected_keys, keys);
}

#[test]
fn entry() {
    let mut doc = doc! {
        "first": 1i32,
        "second": "foo",
        "alphanumeric": "bar",
    };

    {
        let first_entry = doc.entry("first".to_owned());
        assert_eq!(first_entry.key(), "first");

        let v = first_entry.or_insert_with(|| Bson::TimeStamp(27));
        assert_eq!(v, &mut Bson::I32(1));
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
