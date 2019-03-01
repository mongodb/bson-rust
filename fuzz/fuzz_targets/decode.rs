#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate bson;

use bson::decode_document;
use std::io::Cursor;

fuzz_target!(|buf: &[u8]| {
    let _ = decode_document(&mut Cursor::new(&buf[..]));
});
