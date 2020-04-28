use std::fs::File;

fn main() {
    let mut f = File::open("examples/test.bson").unwrap();

    while let Ok(decoded) = bson::decode_document(&mut f) {
        println!("{:?}", decoded);
    }
}
