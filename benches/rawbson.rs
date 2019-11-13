use std::io::{Cursor, Read, Seek};
use bson::{Document, raw::RawBsonDoc, doc, decode_document, encode_document};
use criterion::{
    Criterion,
    black_box,
    criterion_group,
    criterion_main,
};

fn construct_deep_doc(depth: usize) -> bson::Document {
    let mut doc = doc!{"value": 23i64};
    for _ in 0..depth {
        doc = doc!{"value": doc};
    }
    doc
}

fn construct_broad_doc(size: usize) -> bson::Document {
    let mut doc = Document::new();
    for i in 0..size {
        doc.insert(format!("key {}", i), "lorem ipsum");
    }
    doc
}

fn raw_access_deep(c: &mut Criterion) {

    c.bench_function("raw-access-deep", |b| {
        let mut reader = {
            let doc = construct_deep_doc(1000);
            let mut bytes = Vec::new();
            bson::encode_document(&mut bytes, &doc).unwrap();
            Cursor::new(bytes)
        };
        b.iter(|| {
            reader.seek(std::io::SeekFrom::Start(0));
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).unwrap();
            let mut rawdoc = RawBsonDoc::new(&bytes);
            while let Ok(val) = rawdoc.get_document("value") {
                rawdoc = val;
            }
            match rawdoc.get_i64("value") {
                Ok(n) => {},
                err => {
                    err.unwrap();
                }
            }
        });
    });
}


fn parsed_access_deep(c: &mut Criterion) {
    c.bench_function("parsed-access-deep", |b| {
        let mut reader = {
            let doc = construct_deep_doc(1000);
            let mut bytes = Vec::new();
            bson::encode_document(&mut bytes, &doc).unwrap();
            Cursor::new(bytes)
        };
        b.iter(|| {
            reader.seek(std::io::SeekFrom::Start(0));
            let doc = decode_document(&mut reader).unwrap();
            let mut doc = &doc;
            while let Ok(val) = doc.get_document("value") {
                doc = val;
            }
            doc.get_i64("value").unwrap();
        });
    });
}

fn raw_access_broad(c: &mut Criterion) {
    c.bench_function("raw-access-broad", |b| {
        let mut reader = {
            let doc = construct_broad_doc(1000);
            let mut bytes = Vec::new();
            bson::encode_document(&mut bytes, &doc).unwrap();
            Cursor::new(bytes)
        };
        let count = 100;
        let keys_to_get: Vec<_> = ((1000 - count)..1000).map(|i| format!("key {}", i)).collect();
        b.iter(|| {
            reader.seek(std::io::SeekFrom::Start(0));
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).unwrap();
            let mut rawdoc = RawBsonDoc::new(&bytes);
            for key in &keys_to_get {
                rawdoc.get_str(key) .unwrap();
            }
        });
    });
}

fn parsed_access_broad(c: &mut Criterion) {
    c.bench_function("parsed-access-broad", |b| {
        let mut reader = {
            let doc = construct_broad_doc(1000);
            let mut bytes = Vec::new();
            bson::encode_document(&mut bytes, &doc).unwrap();
            Cursor::new(bytes)
        };
        let count = 100;
        let keys_to_get: Vec<_> = ((1000 - count)..1000).map(|i| format!("key {}", i)).collect();
        b.iter(|| {
            reader.seek(std::io::SeekFrom::Start(0));
            let doc = decode_document(&mut reader).unwrap();
            let mut doc = &doc;
            for key in &keys_to_get {
                doc.get_str(key) .unwrap();
            }
        });
    });
}

criterion_group!(
    benches,
    raw_access_deep,
    parsed_access_deep,
    raw_access_broad,
    parsed_access_broad,
);
criterion_main!(benches);

