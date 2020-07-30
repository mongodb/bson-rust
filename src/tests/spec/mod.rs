mod corpus;

use std::{
    convert::TryFrom,
    ffi::OsStr,
    fs::{self, File},
    path::PathBuf,
};

use crate::{from_bson, Bson};
use serde::de::DeserializeOwned;
use serde_json::Value;

pub(crate) fn run_spec_test<T, F>(spec: &[&str], run_test_file: F)
where
    F: Fn(T),
    T: DeserializeOwned,
{
    let base_path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "src", "tests", "spec", "json"]
        .iter()
        .chain(spec.iter())
        .collect();

    for entry in fs::read_dir(&base_path).unwrap() {
        let test_file = entry.unwrap();

        if !test_file.file_type().unwrap().is_file() {
            continue;
        }

        let test_file_path = PathBuf::from(test_file.file_name());
        if test_file_path.extension().and_then(OsStr::to_str) != Some("json") {
            continue;
        }

        let test_file_full_path = base_path.join(&test_file_path);
        let json: Value =
            serde_json::from_reader(File::open(test_file_full_path.as_path()).unwrap()).unwrap();

        run_test_file(from_bson(Bson::try_from(json).unwrap()).unwrap())
    }
}
