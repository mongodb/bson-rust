use bson::{Bson, Document};

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
    doc.insert("first", Bson::I32(1));
    doc.insert("second", Bson::String("foo".to_owned()));
    doc.insert("alphanumeric", Bson::String("bar".to_owned()));

    let expected_keys = vec!(
        "first".to_owned(),
        "second".to_owned(),
        "alphanumeric".to_owned(),
    );

    let keys: Vec<_> = doc.iter().map(|(key, _)| key.to_owned()).collect();
    assert_eq!(expected_keys, keys);
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
