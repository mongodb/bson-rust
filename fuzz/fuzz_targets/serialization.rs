#![no_main]
#[macro_use]
extern crate libfuzzer_sys;
extern crate bson;
use bson::{RawArrayBuf, RawBson, RawBsonRef, RawDocument, RawDocumentBuf};

fn convert_bson_ref(bson_ref: RawBsonRef) -> Option<RawBson> {
    match bson_ref {
        RawBsonRef::Double(d) => Some(RawBson::Double(d)),
        RawBsonRef::String(s) => Some(RawBson::String(s.to_string())),
        RawBsonRef::Document(d) => Some(RawBson::Document(
            RawDocumentBuf::from_bytes(d.as_bytes().to_vec()).unwrap_or_default(),
        )),
        RawBsonRef::Array(a) => {
            let mut array_buf = RawArrayBuf::new();
            if let Ok(array_doc) = RawDocument::from_bytes(a.as_bytes()) {
                for array_elem in array_doc.iter_elements().flatten() {
                    if let Ok(array_bson_ref) = array_elem.try_into() {
                        if let Some(array_bson) = convert_bson_ref(array_bson_ref) {
                            array_buf.push(array_bson);
                        }
                    }
                }
            }
            Some(RawBson::Array(array_buf))
        }
        RawBsonRef::Binary(b) => Some(RawBson::Binary(bson::Binary {
            subtype: b.subtype,
            bytes: b.bytes.to_vec(),
        })),
        RawBsonRef::Boolean(b) => Some(RawBson::Boolean(b)),
        RawBsonRef::Null => Some(RawBson::Null),
        RawBsonRef::Int32(i) => Some(RawBson::Int32(i)),
        RawBsonRef::Int64(i) => Some(RawBson::Int64(i)),
        RawBsonRef::DateTime(dt) => Some(RawBson::DateTime(dt)),
        _ => None,
    }
}

fuzz_target!(|buf: &[u8]| {
    if let Ok(doc) = RawDocument::from_bytes(buf) {
        let mut doc_buf = RawDocumentBuf::new();
        for elem in doc.iter_elements().flatten() {
            let key = elem.key();
            if let Ok(bson_ref) = elem.try_into() {
                if let Some(bson) = convert_bson_ref(bson_ref) {
                    doc_buf.append(key, bson);
                }
            }
        }
        let _ = doc_buf.into_bytes();
    }
});
