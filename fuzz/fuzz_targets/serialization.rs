#![no_main]
use bson::{
    raw::{RawDocument, RawDocumentBuf},
    Document,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    if let Ok(rawdoc) = RawDocument::from_bytes(&input) {
        if let Ok(doc) = Document::try_from(rawdoc) {
            let out = RawDocumentBuf::try_from(&doc).unwrap();
            let out_bytes = out.as_bytes();
            if input != out_bytes {
                let reserialized = RawDocument::from_bytes(&out_bytes).unwrap();
                let reserialized_doc = Document::try_from(reserialized).unwrap();
                // Ensure that the reserialized document is the same as the original document, the
                // bytes can differ while still resulting in the same Document.
                assert_eq!(doc, reserialized_doc, "reserialization failed");
            }
        }
    }
});
