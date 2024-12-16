#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate bson;
use bson::{RawBsonRef, RawDocument};
use std::convert::TryInto;

fuzz_target!(|buf: &[u8]| {
    if let Ok(doc) = RawDocument::from_bytes(buf) {
        for elem in doc.iter_elements().flatten() {
            // Convert to RawBsonRef and check string-related types
            if let Ok(bson) = elem.try_into() {
                match bson {
                    RawBsonRef::String(s) => {
                        let _ = s.len();
                        let _ = s.chars().count();
                    }
                    _ => {}
                }
            }
        }
    }
});
