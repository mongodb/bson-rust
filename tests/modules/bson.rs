extern crate serde_json;

use bson::{Bson, Document};
use self::serde_json::Value;

#[test]
fn to_json() {
    let mut doc = Document::new();
    doc.insert("first", Bson::I32(1));
    doc.insert("second", Bson::String("foo".to_owned()));
    doc.insert("alphanumeric", Bson::String("bar".to_owned()));
    let data: Value = Bson::Document(doc).clone().into();

    assert!(data.is_object());
    let obj = data.as_object().unwrap();

    let first = obj.get("first").unwrap();
    assert!(first.is_number());
    assert_eq!(first.as_i64().unwrap(), 1);

    let second = obj.get("second").unwrap();
    assert!(second.is_string());
    assert_eq!(second.as_str().unwrap(), "foo");

    let alphanumeric = obj.get("alphanumeric").unwrap();
    assert!(alphanumeric.is_string());
    assert_eq!(alphanumeric.as_str().unwrap(), "bar");
}

#[test]
fn bson_default() {
    let bson1 = Bson::default();
    assert_eq!(bson1, Bson::Null);
}

#[test]
fn document_default() {
    let doc1 = Document::default();
    assert_eq!(doc1.keys().count(), 0);
    assert_eq!(doc1, Document::new());
}
