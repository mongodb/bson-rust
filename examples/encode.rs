extern crate bson;
extern crate chrono;

use std::io::Cursor;
use bson::{Bson, Document, Array, Encoder, Decoder};

fn main() {
    let mut doc = Document::new();
    doc.insert("foo".to_string(), Bson::String("bar".to_string()));

    let mut arr = Array::new();
    arr.push(Bson::String("blah".to_string()));
    arr.push(Bson::UtcDatetime(chrono::UTC::now()));
    arr.push(Bson::ObjectId([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]));

    doc.insert("array".to_string(), Bson::Array(arr));

    let mut buf = Vec::new();
    {
        let mut enc = Encoder::new(&mut buf);
        enc.encode_document(&doc).unwrap();
    }

    println!("Encoded: {:?}", buf);

    let mut r = Cursor::new(&buf[..]);
    {
        let mut dec = Decoder::new(&mut r);
        let doc = dec.decode_document().unwrap();

        println!("Decoded: {:?}", doc);
    }
}
