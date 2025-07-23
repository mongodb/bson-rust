#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate bson;
use bson::{Document, Utf8Lossy};

fuzz_target!(|buf: &[u8]| {
    if let Ok(doc) = bson::deserialize_from_slice::<Utf8Lossy<Document>>(buf) {
        let _ = bson::serialize_to_vec(&doc.0);
    }
});
