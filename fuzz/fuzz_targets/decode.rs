#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate bson;

use bson::Document;
use std::io::Cursor;

fuzz_target!(|buf: &[u8]| {
    if let Ok(doc) = Document::decode_from_reader(&mut Cursor::new(&buf[..])) {
        let mut vec = Vec::with_capacity(buf.len());
        let _ = doc.encode_to_writer(&mut vec);
    }
});
