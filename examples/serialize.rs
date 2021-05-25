use std::io::Cursor;

use bson::{oid, Bson, DateTime, Document};

fn main() {
    let mut doc = Document::new();
    doc.insert("foo".to_string(), Bson::String("bar".to_string()));

    let arr = vec![
        Bson::String("blah".to_string()),
        Bson::DateTime(DateTime::now()),
        Bson::ObjectId(oid::ObjectId::from_bytes([
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
        ])),
    ];

    doc.insert("array".to_string(), Bson::Array(arr));

    let mut buf = Vec::new();
    doc.to_writer(&mut buf).unwrap();

    println!("Serialized: {:?}", buf);

    let doc = Document::from_reader(&mut Cursor::new(&buf[..])).unwrap();
    println!("Deserialized: {:?}", doc);
}
