#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate bson;
use bson::{RawBsonRef, RawDocument, RawWriter};
use std::convert::TryInto;

fuzz_target!(|buf: &[u8]| {
    if let Ok(doc) = RawDocument::from_bytes(buf) {
        let mut vec = Vec::with_capacity(buf.len());
        vec.extend_from_slice(&[5, 0, 0, 0, 0]); // minimal valid doc
        let mut writer = RawWriter::new(&mut vec);
        for elem in doc.iter_elements().flatten() {
            if let Ok(elem) = elem {
                if let Ok(bson) = elem.try_into::<RawBsonRef>() {
                    let _ = writer.append(elem.key(), bson);
                }
            }
        }
    }
});
