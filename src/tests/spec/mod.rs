mod corpus;
mod vector;

use std::{
    any::type_name,
    ffi::OsStr,
    fs::{self, File},
    path::PathBuf,
};

use crate::Bson;
use serde::de::DeserializeOwned;

pub(crate) fn run_spec_test<T, F>(spec: &[&str], run_test_file: F)
where
    F: Fn(T),
    T: DeserializeOwned,
{
    let base_path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "src", "tests", "spec", "json"]
        .iter()
        .chain(spec.iter())
        .collect();

    for entry in fs::read_dir(&base_path)
        .unwrap_or_else(|e| panic!("Failed to read directory at {:?}: {}", base_path, e))
    {
        let path = entry.unwrap().path();
        if path.extension() != Some(OsStr::new("json")) {
            continue;
        }

        let file = File::open(&path)
            .unwrap_or_else(|e| panic!("Failed to open file at {:?}: {}", path, e));

        let mut json_deserializer = serde_json::Deserializer::from_reader(file);
        let test_bson: Bson = serde_path_to_error::deserialize(&mut json_deserializer)
            .unwrap_or_else(|e| {
                panic!(
                    "Failed to deserialize test JSON to BSON in {:?}: {}",
                    path, e
                )
            });
        let bson_deserializer = crate::Deserializer::new(test_bson);
        let test: T = serde_path_to_error::deserialize(bson_deserializer).unwrap_or_else(|e| {
            panic!(
                "Failed to deserialize test BSON to {} in {:?}: {}",
                type_name::<T>(),
                path,
                e
            )
        });

        run_test_file(test)
    }
}
