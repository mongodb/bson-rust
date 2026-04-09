use std::{ffi::OsStr, fs::File, io::Read, ops::Deref, path::PathBuf};

use crate::{Document, facet::ExtJson, tests::corpus::TestFile};

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

        let buf = {
            let mut buf = vec![];
            let mut file = File::open(&path).unwrap();
            file.read_to_end(&mut buf).unwrap();
            buf
        };

        let test = facet_json::from_slice::<TestFile>(&buf).expect(path.to_string_lossy().deref());
        for v in &test.valid {
            let description = format!("{}: {}", test.description, v.description);

            let canonical_bson_bytes = hex::decode(&v.canonical_bson).expect(&description);
            let canonical_bson_doc =
                Document::from_reader(canonical_bson_bytes.as_slice()).expect(&description);

            let canonical_extjson =
                facet_json::from_str::<ExtJson>(&v.canonical_extjson).expect(&description);
            let canonical_extjson_doc = Document::try_from(canonical_extjson).expect(&description);

            // NaN never compares equal, and lossy tests produce output different from input, so for
            // those we check that they parse but skip the equality test.
            if v.description.contains("NaN") || v.lossy == Some(true) {
                continue;
            }

            assert_eq!(canonical_bson_doc, canonical_extjson_doc, "{description}");

            let facet_bytes =
                crate::facet::format::to_vec(&canonical_extjson_doc).expect(&description);
            assert_eq!(canonical_bson_bytes, facet_bytes, "{description}");
        }
    }
}
