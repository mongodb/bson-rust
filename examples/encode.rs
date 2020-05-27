use std::io::Cursor;

use bson::{oid, Array, Bson, Document};

fn main() {
    let mut doc = Document::new();
    doc.insert("foo".to_string(), Bson::String("bar".to_string()));

    let mut arr = Array::new();
    arr.push(Bson::String("blah".to_string()));
    arr.push(Bson::UtcDatetime(chrono::Utc::now()));
    arr.push(Bson::ObjectId(oid::ObjectId::with_bytes([
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
    ])));

    doc.insert("array".to_string(), Bson::Array(arr));

    let mut buf = Vec::new();
    doc.encode(&mut buf).unwrap();

    println!("Encoded: {:?}", buf);

    let doc = Document::decode(&mut Cursor::new(&buf[..])).unwrap();
    println!("Decoded: {:?}", doc);
}
