#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate bson;
use bson::Document;

fuzz_target!(|buf: &[u8]| {
    let _ = bson::from_slice::<Document>(buf);
});
