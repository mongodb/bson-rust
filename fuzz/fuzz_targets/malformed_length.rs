#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate bson;
use bson::RawDocument;

fuzz_target!(|buf: &[u8]| {
    if buf.len() >= 4 {
        // Focus on document length field manipulation
        let _ = RawDocument::from_bytes(buf);
    }
});
