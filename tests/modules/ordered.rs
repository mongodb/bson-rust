use bson::{Bson, Document};
use bson::ValueAccessError;
use bson::spec::BinarySubtype;
use bson::oid::ObjectId;
use chrono::Utc;

#[test]
fn ordered_insert() {
    let mut doc = Document::new();
    doc.insert("first".to_owned(), Bson::I32(1));
    doc.insert("second".to_owned(), Bson::String("foo".to_owned()));
    doc.insert("alphanumeric".to_owned(), Bson::String("bar".to_owned()));

    let expected_keys = vec!(
        "first".to_owned(),
        "second".to_owned(),
        "alphanumeric".to_owned(),
    );

    let keys: Vec<_> = doc.iter().map(|(key, _)| key.to_owned()).collect();
    assert_eq!(expected_keys, keys);
}

#[test]
fn ordered_insert_shorthand() {
    let mut doc = Document::new();
    doc.insert("first", 1i32);
    doc.insert("second", "foo");
    doc.insert("alphanumeric", "bar".to_owned());

    let expected_keys = vec!(
        "first".to_owned(),
        "second".to_owned(),
        "alphanumeric".to_owned(),
    );

    let keys: Vec<_> = doc.iter().map(|(key, _)| key.to_owned()).collect();
    assert_eq!(expected_keys, keys);
}

#[test]
fn test_getters() {
    let datetime = Utc::now();
    let cloned_dt = datetime.clone();
    let binary = vec![0, 1, 2, 3, 4];
    let mut doc = doc! {
        "floating_point" => 10.0,
        "string" => "a value",
        "array" => [10, 20, 30],
        "doc" => { "key" => 1 },
        "bool" => true,
        "i32" => 1i32,
        "i64" => 1i64,
        "datetime" => cloned_dt,
        "binary" => (BinarySubtype::Generic, binary.clone())
    };

    assert_eq!(None, doc.get("nonsense"));
    assert_eq!(Err(ValueAccessError::NotPresent), doc.get_str("nonsense"));
    assert_eq!(Err(ValueAccessError::UnexpectedType), doc.get_str("floating_point"));

    assert_eq!(Some(&Bson::FloatingPoint(10.0)), doc.get("floating_point"));
    assert_eq!(Ok(10.0), doc.get_f64("floating_point"));

    assert_eq!(Some(&Bson::String("a value".to_string())), doc.get("string"));
    assert_eq!(Ok("a value"), doc.get_str("string"));

    let array = vec![Bson::I32(10), Bson::I32(20), Bson::I32(30)];
    assert_eq!(Some(&Bson::Array(array.clone())), doc.get("array"));
    assert_eq!(Ok(&array), doc.get_array("array"));

    let embedded = doc! { "key" => 1 };
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

    assert_eq!(Some(&Bson::UtcDatetime(datetime.clone())), doc.get("datetime"));
    assert_eq!(Ok(&datetime), doc.get_utc_datetime("datetime"));

    let object_id = ObjectId::new().unwrap();
    doc.insert("_id".to_string(), Bson::ObjectId(object_id.clone()));
    assert_eq!(Some(&Bson::ObjectId(object_id.clone())), doc.get("_id"));
    assert_eq!(Ok(&object_id), doc.get_object_id("_id"));

    assert_eq!(Some(&Bson::Binary(BinarySubtype::Generic, binary.clone())), doc.get("binary"));
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

    let expected_keys = vec!(
        "first",
        "alphanumeric",
    );

    let keys: Vec<_> = doc.iter().map(|(key, _)| key.to_owned()).collect();
    assert_eq!(expected_keys, keys);
}
