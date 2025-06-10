#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate bson;
use bson::Document;

fuzz_target!(|buf: &[u8]| {
    if let Ok(doc) = bson::from_slice::<Document>(buf) {
        let mut vec = Vec::with_capacity(buf.len());
        let _ = doc.to_writer(&mut vec);
    }
});
