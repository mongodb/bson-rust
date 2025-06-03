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
        if let Some(val2) = doc2.get(key) {
            if !compare_values(value, val2) {
                return false;
            }
        } else {
            return false;
        }
    }
    true
}

fn compare_values(val1: &Bson, val2: &Bson) -> bool {
    match (val1, val2) {
        (Bson::Double(d1), Bson::Double(d2)) => (d1.is_nan() && d2.is_nan()) || d1 == d2,
        (Bson::Document(doc1), Bson::Document(doc2)) => compare_docs(doc1, doc2),
        (Bson::Array(arr1), Bson::Array(arr2)) => {
            if arr1.len() != arr2.len() {
                return false;
            }
            for (subval1, subval2) in std::iter::zip(arr1, arr2) {
                if !compare_values(subval1, subval2) {
                    return false;
                }
            }
            true
        }
        (Bson::JavaScriptCodeWithScope(jsc1), Bson::JavaScriptCodeWithScope(jsc2)) => {
            jsc1.code == jsc2.code && compare_docs(&jsc1.scope, &jsc2.scope)
        }
        (v1, v2) => v1 == v2,
    }
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
