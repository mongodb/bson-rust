#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate bson;

use bson::Document;
use std::io::Cursor;

fuzz_target!(|buf: &[u8]| {
    let _ = Document::from_reader(&mut Cursor::new(&buf[..]));
});
