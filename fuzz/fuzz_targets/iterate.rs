#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate bson;
use bson::RawDocument;

fuzz_target!(|buf: &[u8]| {
    if let Ok(doc) = RawDocument::from_bytes(buf) {
        for _ in doc {}
    }
});
