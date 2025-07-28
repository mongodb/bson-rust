#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate bson;
use bson::{RawBsonRef, RawDocument};
use std::convert::TryInto;

fuzz_target!(|buf: &[u8]| {
    if let Ok(doc) = RawDocument::from_bytes(buf) {
        for elem in doc.iter_elements().flatten() {
            let _: Result<RawBsonRef, _> = elem.try_into();
        }
    }
});
