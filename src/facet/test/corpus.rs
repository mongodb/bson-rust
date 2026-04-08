use std::{ffi::OsStr, fs::File, io::Read, path::PathBuf};

use crate::tests::corpus;

#[test]
fn run() {
    let base_path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "src",
        "tests",
        "spec",
        "json",
        "bson-corpus",
    ]
    .iter()
    .collect();

    for entry in std::fs::read_dir(&base_path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension() != Some(OsStr::new("json")) {
            continue;
        }
        let mut file = File::open(&path).unwrap();
        let mut buf = vec![];
        file.read_to_end(&mut buf).unwrap();
        drop(file);
    }
}
