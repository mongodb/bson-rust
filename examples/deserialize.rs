use std::fs::File;

use bson::Document;

fn main() {
    let mut f = File::open("examples/test.bson").unwrap();

    while let Ok(deserialized) = Document::from_reader(&mut f) {
        println!("{:?}", deserialized);
    }
}
