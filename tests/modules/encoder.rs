extern crate bson;

use bson::{Document, Bson, encode_document};

#[test]
fn test_encode_floating_point() {
    let src = 1020.123;
    let dst = [18, 0, 0, 0, 1, 107, 101, 121, 0, 68, 139, 108, 231, 251, 224, 143, 64, 0];

    let mut doc = Document::new();
    doc.insert("key".to_owned(), Bson::FloatingPoint(src));

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(&buf, &dst);
}

#[test]
fn test_encode_utf8_string() {
    let src = "test你好吗".to_owned();
    let dst = [28, 0, 0, 0, 2, 107, 101, 121, 0, 14, 0, 0, 0, 116, 101, 115, 116, 228, 189, 160, 229, 165, 189, 229, 144, 151, 0, 0];

    let mut doc = Document::new();
    doc.insert("key".to_owned(), Bson::String(src));

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(&buf, &dst);
}

#[test]
fn test_encode_array() {
    let src = vec![Bson::FloatingPoint(1.01), Bson::String("xyz".to_owned())];
    let dst = [37, 0, 0, 0, 4, 107, 101, 121, 0, 27, 0, 0, 0, 1, 48, 0, 41, 92, 143, 194, 245, 40, 240, 63, 2, 49, 0, 4, 0, 0, 0, 120, 121, 122, 0, 0, 0];

    let mut doc = Document::new();
    doc.insert("key".to_owned(), Bson::Array(src));

    let mut buf = Vec::new();
    encode_document(&mut buf, &doc).unwrap();

    assert_eq!(&buf[..], &dst[..]);
}
