use bson::{doc, Bson, Document};
use std::fs;
use std::path::Path;

fn main() -> std::io::Result<()> {
    let corpus_dir = Path::new("fuzz/corpus");
    fs::create_dir_all(corpus_dir)?;

    // Generate edge cases for each fuzz target
    generate_length_edge_cases(corpus_dir)?;
    generate_type_marker_cases(corpus_dir)?;
    generate_string_edge_cases(corpus_dir)?;
    generate_serialization_cases(corpus_dir)?;
    Ok(())
}

fn generate_length_edge_cases(dir: &Path) -> std::io::Result<()> {
    let target_dir = dir.join("malformed_length");
    fs::create_dir_all(&target_dir)?;

    // Minimal valid document
    let min_doc = doc! {};
    fs::write(target_dir.join("min_doc"), min_doc.to_raw_document_buf().as_bytes())?;

    // Document with length near i32::MAX
    let large_doc = doc! { "a": "b".repeat(i32::MAX as usize / 2) };
    fs::write(target_dir.join("large_doc"), large_doc.to_raw_document_buf().as_bytes())?;

    Ok(())
}

fn generate_type_marker_cases(dir: &Path) -> std::io::Result<()> {
    let target_dir = dir.join("type_markers");
    fs::create_dir_all(&target_dir)?;

    // Document with all BSON types
    let all_types = doc! {
        "double": 1.0f64,
        "string": "test",
        "document": doc! {},
        "array": vec![1, 2, 3],
        "binary": Bson::Binary(bson::Binary { subtype: bson::spec::BinarySubtype::Generic, bytes: vec![1, 2, 3] }),
        "object_id": bson::oid::ObjectId::new(),
        "bool": true,
        "date": bson::DateTime::now(),
        "null": Bson::Null,
        "regex": Bson::RegularExpression(bson::Regex { pattern: "pattern".into(), options: "i".into() }),
        "int32": 123i32,
        "timestamp": bson::Timestamp { time: 12345, increment: 1 },
        "int64": 123i64
    };
    fs::write(target_dir.join("all_types"), all_types.to_raw_document_buf().as_bytes())?;

    Ok(())
}

fn generate_string_edge_cases(dir: &Path) -> std::io::Result<()> {
    let target_dir = dir.join("string_handling");
    fs::create_dir_all(&target_dir)?;

    // UTF-8 edge cases
    let utf8_cases = doc! {
        "empty": "",
        "null_bytes": "hello\0world",
        "unicode": "🦀💻🔒",
        "surrogate": "\u{D800}",
        "invalid_continuation": vec![0x80u8, 0x80u8, 0x80u8],
        "overlong": vec![0xC0u8, 0x80u8], // Overlong encoding of NUL
    };
    fs::write(target_dir.join("utf8_cases"), utf8_cases.to_raw_document_buf().as_bytes())?;

    Ok(())
}

fn generate_serialization_cases(dir: &Path) -> std::io::Result<()> {
    let target_dir = dir.join("serialization");
    fs::create_dir_all(&target_dir)?;

    // Deeply nested document
    let mut nested_doc = doc! {};
    let mut current = &mut nested_doc;
    for i in 0..100 {
        let next_doc = doc! {};
        current.insert(i.to_string(), next_doc);
        current = current.get_mut(&i.to_string()).unwrap().as_document_mut().unwrap();
    }
    fs::write(target_dir.join("nested_doc"), nested_doc.to_raw_document_buf().as_bytes())?;

    // Document with large binary data
    let large_binary = doc! {
        "binary": Bson::Binary(bson::Binary {
            subtype: bson::spec::BinarySubtype::Generic,
            bytes: vec![0xFF; 1024 * 1024] // 1MB of data
        })
    };
    fs::write(target_dir.join("large_binary"), large_binary.to_raw_document_buf().as_bytes())?;

    Ok(())
}
