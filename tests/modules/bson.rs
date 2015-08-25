use bson::{Bson, Document};

#[test]
fn to_json() {
    let mut doc = Document::new();
    doc.insert("first".to_owned(), Bson::I32(1));
    doc.insert("second".to_owned(), Bson::String("foo".to_owned()));
    doc.insert("alphanumeric".to_owned(), Bson::String("bar".to_owned()));
    let data = Bson::Document(doc).to_json();

    assert!(data.is_object());
    let obj = data.as_object().unwrap();

    let first = obj.get("first").unwrap();
    assert!(first.is_number());
    assert_eq!(first.as_i64().unwrap(), 1);

    let second = obj.get("second").unwrap();
    assert!(second.is_string());
    assert_eq!(second.as_string().unwrap(), "foo");

    let alphanumeric = obj.get("alphanumeric").unwrap();
    assert!(alphanumeric.is_string());
    assert_eq!(alphanumeric.as_string().unwrap(), "bar");
}
