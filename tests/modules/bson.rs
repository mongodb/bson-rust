extern crate serde_json;

use self::serde_json::{Value, json};
use bson::{Bson, Document, oid::ObjectId, spec::BinarySubtype};

#[test]
fn to_json() {
    let mut doc = Document::new();
    doc.insert("_id", Bson::ObjectId(ObjectId::with_bytes(*b"abcdefghijkl")));
    doc.insert("first", Bson::I32(1));
    doc.insert("second", Bson::String("foo".to_owned()));
    doc.insert("alphanumeric", Bson::String("bar".to_owned()));
    let data: Value = Bson::Document(doc).clone().into();

    assert!(data.is_object());
    let obj = data.as_object().unwrap();

    let id = obj.get("_id").unwrap();
    assert!(id.is_object());
    let id_val = id.get("$oid").unwrap();
    assert!(id_val.is_string());
    assert_eq!(id_val, "6162636465666768696a6b6c");

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

#[test]
fn from_impls() {
    assert_eq!(Bson::from(1.5f32), Bson::FloatingPoint(1.5));
    assert_eq!(Bson::from(2.25f64), Bson::FloatingPoint(2.25));
    assert_eq!(Bson::from("data"), Bson::String(String::from("data")));
    assert_eq!(Bson::from(String::from("data")), Bson::String(String::from("data")));
    assert_eq!(Bson::from(doc!{}), Bson::Document(Document::new()));
    assert_eq!(Bson::from(false), Bson::Boolean(false));
    assert_eq!(Bson::from((String::from("\\s+$"), String::from("i"))), Bson::RegExp(String::from("\\s+$"), String::from("i")));
    assert_eq!(Bson::from((String::from("alert(\"hi\");"), doc!{})), Bson::JavaScriptCodeWithScope(String::from("alert(\"hi\");"), doc!{}));
    //
    assert_eq!(Bson::from((BinarySubtype::Generic, vec![1, 2, 3])), Bson::Binary(BinarySubtype::Generic, vec![1, 2, 3]));
    assert_eq!(Bson::from(-48i32), Bson::I32(-48));
    assert_eq!(Bson::from(-96i64), Bson::I64(-96));
    assert_eq!(Bson::from(152u32), Bson::I32(152));
    assert_eq!(Bson::from(4096u64), Bson::I64(4096));

    let oid = ObjectId::new().unwrap();
    assert_eq!(Bson::from(b"abcdefghijkl"), Bson::ObjectId(ObjectId::with_bytes(*b"abcdefghijkl")));
    assert_eq!(Bson::from(oid.clone()), Bson::ObjectId(oid.clone()));
    assert_eq!(Bson::from(vec![1, 2, 3]), Bson::Array(vec![Bson::I32(1), Bson::I32(2), Bson::I32(3)]));
    assert_eq!(Bson::from(json!({"_id": {"$oid": oid.to_hex()}, "name": ["bson-rs"]})), Bson::Document(doc!{"_id": &oid, "name": ["bson-rs"]}));

    // References
    assert_eq!(Bson::from(&24i32), Bson::I32(24));
    assert_eq!(Bson::from(&String::from("data")), Bson::String(String::from("data")));
    assert_eq!(Bson::from(&oid), Bson::ObjectId(oid));
    assert_eq!(Bson::from(&doc!{"a": "b"}), Bson::Document(doc!{"a": "b"}));

}
