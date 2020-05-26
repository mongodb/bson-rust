use std::fs::File;

use bson::Document;

fn main() {
    let mut f = File::open("examples/test.bson").unwrap();

    while let Ok(decoded) = Document::decode_document(&mut f) {
        println!("{:?}", decoded);
    }
}
