#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate bson;
use bson::RawDocument;

fuzz_target!(|buf: &[u8]| {
    if let Ok(doc) = RawDocument::from_bytes(buf) {
        let mut iter = doc.iter_elements();
        while let Some(element) = iter.next_view() {
            if let Ok(view) = element {
                let _ = view.key_bytes();
                let _ = view.element_type();
                let _ = view.value_view();
                let _ = view.value();
            }
        }
    }
});
