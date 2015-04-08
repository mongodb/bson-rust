extern crate bson;

use std::fs::File;

fn main() {
    let mut f = File::open("examples/test.bson").unwrap();

    while let Ok(decoded) = bson::Decoder::new(&mut f).decode_document() {
        println!("{:?}", decoded);
    }
}
