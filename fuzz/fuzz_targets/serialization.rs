#![no_main]
use bson::{
    raw::{RawDocument, RawDocumentBuf},
    Bson,
    Document,
};
use libfuzzer_sys::fuzz_target;

fn compare_docs(doc1: &Document, doc2: &Document) -> bool {
    if doc1.len() != doc2.len() {
        return false;
    }
    for (key, value) in doc1 {
        if !doc2.contains_key(key) {
            return false;
        }
        if let Some(val2) = doc2.get(key) {
            match (value, val2) {
                (Bson::Double(d1), Bson::Double(d2)) => {
                    if (!d1.is_nan() || !d2.is_nan()) && d1 != d2 {
                        return false;
                    }
                }
                (v1, v2) => {
                    if v1 != v2 {
                        return false;
                    }
                }
            }
        }
    }
    true
}

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
                if !compare_docs(&doc, &reserialized_doc) {
                    panic!(
                        "Reserialized document is not the same as the original document: {:?} != \
                         {:?}",
                        doc, reserialized_doc
                    );
                }
            }
        }
    }
});
