use std::io::{Cursor, Read};
use std::convert::TryInto;
use bson::{Document, raw::RawBsonDoc, doc, decode_document};
use criterion::{
    BenchmarkId,
    Criterion,
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

fn access_deep_from_bytes(c: &mut Criterion) {
    let mut group = c.benchmark_group("access-deep-from-bytes");
    for depth in &[10, 100, 1000] {
        let depth = *depth;
        let inbytes = {
            let doc = construct_deep_doc(depth);
            let mut bytes = Vec::new();
            bson::encode_document(&mut bytes, &doc).unwrap();
            bytes
        };
        group.bench_with_input(BenchmarkId::new("raw", depth), &inbytes,
            |b, inbytes| b.iter(|| {
                let mut reader = Cursor::new(inbytes);
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes).unwrap();
                let mut rawdoc = RawBsonDoc::new(&bytes).expect("invalid document");
                while let Ok(val) = rawdoc.get_document("value") {
                    rawdoc = val;
                }
                rawdoc.get_i64("value").unwrap(); 
            }),
        );
        group.bench_with_input(BenchmarkId::new("parsed", depth), &inbytes,
            |b, inbytes| b.iter(|| {
                let mut reader = Cursor::new(inbytes);
                let doc = decode_document(&mut reader).unwrap();
                let mut doc = &doc;
                while let Ok(val) = doc.get_document("value") {
                    doc = val;
                }
                doc.get_i64("value").unwrap();
            }),
        );
    }
    group.finish();
}

fn access_broad_from_bytes(c: &mut Criterion) {
    const SIZE: usize = 1000;
    let mut group = c.benchmark_group("access-broad-from-bytes");
    let inbytes: Vec<u8> = {
        let doc = construct_broad_doc(SIZE);
        let mut bytes = Vec::new();
        bson::encode_document(&mut bytes, &doc).unwrap();
        bytes
    };
    let inbytes = &inbytes;
    for count in &[1, 10, 20, 30, 40, 50] {
        let count = *count;
        let keys_to_get: Vec<_> = ((SIZE - count)..SIZE).map(|i| format!("key {}", i)).collect();
        group.bench_with_input(BenchmarkId::new("raw", count), &keys_to_get,
            |b, keys_to_get| {
            b.iter(|| {
                let mut reader = Cursor::new(&inbytes);
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes).unwrap();
                let rawdoc = RawBsonDoc::new(&bytes).expect("invalid document");
                for key in keys_to_get {
                    rawdoc.get_str(&key).unwrap();
                }
            });
        });
        group.bench_with_input(BenchmarkId::new("parsed", count), &keys_to_get,
            |b, keys_to_get| {
            b.iter( || {
                    let mut reader = Cursor::new(&inbytes);
                    let doc = decode_document(&mut reader).unwrap();
                    for key in keys_to_get {
                        doc.get_str(&key).unwrap();
                    }
            });
        });
    }
    group.finish();
}

fn iter_broad_from_bytes(c: &mut Criterion) {
    const SIZE: usize = 1000;
    let mut group = c.benchmark_group("iter-broad-from-bytes");
    let inbytes: Vec<u8> = {
        let doc = construct_broad_doc(SIZE);
        let mut bytes = Vec::new();
        bson::encode_document(&mut bytes, &doc).unwrap();
        bytes
    };
    let inbytes = &inbytes;
    group.bench_function("raw", |b| b.iter(|| {
                let mut reader = Cursor::new(&inbytes);
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes).unwrap();
                let rawdoc = RawBsonDoc::new(&bytes).expect("invalid document");
                let mut i = 0;
                for result in rawdoc {
                    if result.is_ok() {
                        i += 1;
                    }
                }
                assert_eq!(i, SIZE);
            }));
    group.bench_function("parsed", |b| b.iter(|| {
                let mut reader = Cursor::new(&inbytes);
                let doc = decode_document(&mut reader).unwrap();
                let mut i = 0;
                for (key, value) in doc {
                    i += 1;
                }
                assert_eq!(i, SIZE);
            }));
    group.finish();
}

fn access_deep_from_type(c: &mut Criterion) {
    let mut group = c.benchmark_group("access-deep-from-type");
    for depth in &[10, 100, 1000] {
        let depth = *depth;
        let inbytes = {
            let doc = construct_deep_doc(depth);
            let mut bytes = Vec::new();
            bson::encode_document(&mut bytes, &doc).unwrap();
            bytes
        };
        group.bench_with_input(BenchmarkId::new("raw", depth), &inbytes,
            |b, inbytes| b.iter(|| {
                let mut reader = Cursor::new(inbytes);
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes).unwrap();
                let mut rawdoc = RawBsonDoc::new(&bytes).expect("invalid document");
                while let Ok(val) = rawdoc.get_document("value") {
                    rawdoc = val;
                }
                rawdoc.get_i64("value").unwrap(); 
            }),
        );
        group.bench_with_input(BenchmarkId::new("parsed", depth), &inbytes,
            |b, inbytes| b.iter(|| {
                let mut reader = Cursor::new(inbytes);
                let doc = decode_document(&mut reader).unwrap();
                let mut doc = &doc;
                while let Ok(val) = doc.get_document("value") {
                    doc = val;
                }
                doc.get_i64("value").unwrap();
            }),
        );
    }
    group.finish();
}

fn access_broad_from_type(c: &mut Criterion) {
    const SIZE: usize = 1000;
    let mut group = c.benchmark_group("access-broad-from-type");
    let inbytes: Vec<u8> = {
        let doc = construct_broad_doc(SIZE);
        let mut bytes = Vec::new();
        bson::encode_document(&mut bytes, &doc).unwrap();
        bytes
    };
    let inbytes = &inbytes;
    for count in &[1, 10, 20, 30, 40, 50] {
        let count = *count;
        let keys_to_get: Vec<_> = ((SIZE - count)..SIZE).map(|i| format!("key {}", i)).collect();
        group.bench_with_input(BenchmarkId::new("raw", count), &keys_to_get,
            |b, keys_to_get| {
                let mut reader = Cursor::new(inbytes);
                let mut bytes = Vec::new();
                reader.read_to_end(&mut bytes).unwrap();
                let rawdoc = RawBsonDoc::new(&bytes).expect("invalid document");

                b.iter(|| {
                    for key in keys_to_get {
                        rawdoc.get_str(&key).unwrap();
                    }
                }
            );
        });
        group.bench_with_input(BenchmarkId::new("parsed", count), &keys_to_get,
            |b, keys_to_get| {
                let mut reader = Cursor::new(inbytes);
                let doc = decode_document(&mut reader).unwrap();

                b.iter( || {
                        for key in keys_to_get {
                            doc.get_str(&key).unwrap();
                        }
                }
            );
        });
    }
    group.finish();
}

fn iter_broad_from_type(c: &mut Criterion) {
    const SIZE: usize = 1000;
    let mut group = c.benchmark_group("iter-broad-from-type");
    let inbytes: Vec<u8> = {
        let doc = construct_broad_doc(SIZE);
        let mut bytes = Vec::new();
        bson::encode_document(&mut bytes, &doc).unwrap();
        bytes
    };
    let inbytes = &inbytes;
    group.bench_function("raw", |b| {
        let mut reader = Cursor::new(inbytes);
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        let rawdoc = RawBsonDoc::new(&bytes).expect("invalid document");
                
        b.iter(|| {
                let mut i = 0;
                for result in rawdoc {
                    let (key, value) = result.expect("invalid bson");
                    i += 1;
                }
                assert_eq!(i, SIZE);
        });
    });
    group.bench_function("parsed", |b| {
        let mut reader = Cursor::new(&inbytes);
        let doc = decode_document(&mut reader).unwrap();
        let doc = &doc;
        b.iter(|| {
            let mut i = 0;
            for (key, value) in doc {
                i += 1;
            }
            assert_eq!(i, SIZE);
        })
    });
    group.finish();
}

fn construct_bson_deep(c: &mut Criterion) {
    const SIZE: usize = 1000;
    let mut group = c.benchmark_group("construct-bson-deep");
    let inbytes: Vec<u8> = {
        let doc = construct_deep_doc(SIZE);
        let mut bytes = Vec::new();
        bson::encode_document(&mut bytes, &doc).unwrap();
        bytes
    };
    let inbytes = &inbytes;
    group.bench_function("direct", |b| b.iter(|| {
        let mut reader = Cursor::new(&inbytes);
        let doc: Document = decode_document(&mut reader).unwrap();
    }));
    group.bench_function("via-raw", |b| b.iter(|| {
        let mut reader = Cursor::new(inbytes);
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        let rawdoc = RawBsonDoc::new(&bytes).expect("invalid document");
        let doc: Document = rawdoc.try_into().expect("could not convert document");
    }));
    group.finish();
}

fn construct_bson_broad(c: &mut Criterion) {
    const SIZE: usize = 1000;
    let mut group = c.benchmark_group("construct-bson-broad");
    let inbytes: Vec<u8> = {
        let doc = construct_broad_doc(SIZE);
        let mut bytes = Vec::new();
        bson::encode_document(&mut bytes, &doc).unwrap();
        bytes
    };
    let inbytes = &inbytes;
    group.bench_function("direct", |b| b.iter(|| {
        let mut reader = Cursor::new(&inbytes);
        let doc: Document = decode_document(&mut reader).unwrap();
    }));
    group.bench_function("via-raw", |b| b.iter(|| {
        let mut reader = Cursor::new(inbytes);
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).unwrap();
        let rawdoc = RawBsonDoc::new(&bytes).expect("invalid document");
        let doc: Document = rawdoc.try_into().expect("invalid document");
    }));
    group.finish();
}

criterion_group!(
    benches,
    access_deep_from_bytes,
    access_broad_from_bytes,
    iter_broad_from_bytes,
    access_deep_from_type,
    access_broad_from_type,
    iter_broad_from_type,
    construct_bson_deep,
    construct_bson_broad,
);

criterion_main!(benches);

