use std::{
    convert::{TryFrom, TryInto},
    str::FromStr,
};

use crate::{tests::LOCK, Bson, Document};
use pretty_assertions::assert_eq;
use serde::Deserialize;

use super::run_spec_test;

#[derive(Debug, Deserialize)]
struct TestFile {
    description: String,
    bson_type: String,
    test_key: Option<String>,

    #[serde(default)]
    valid: Vec<Valid>,

    #[serde(rename = "decodeErrors")]
    #[serde(default)]
    decode_errors: Vec<DecodeError>,

    #[serde(rename = "parseErrors")]
    #[serde(default)]
    parse_errors: Vec<ParseError>,

    deprecated: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct Valid {
    description: String,
    canonical_bson: String,
    canonical_extjson: String,
    relaxed_extjson: Option<String>,
    degenerate_bson: Option<String>,
    degenerate_extjson: Option<String>,
    converted_bson: Option<String>,
    converted_extjson: Option<String>,
    lossy: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct DecodeError {
    description: String,
    bson: String,
}

#[derive(Debug, Deserialize)]
struct ParseError {
    description: String,
    string: String,
}

fn run_test(test: TestFile) {
    let _guard = LOCK.run_concurrently();
    for valid in test.valid {
        let description = format!("{}: {}", test.description, valid.description);

        let bson_to_native_cb = Document::from_reader(
            &mut hex::decode(&valid.canonical_bson)
                .expect(&description)
                .as_slice(),
        )
        .expect(&description);

        let mut native_to_bson_bson_to_native_cv = Vec::new();
        bson_to_native_cb
            .to_writer(&mut native_to_bson_bson_to_native_cv)
            .expect(&description);

        // TODO RUST-36: Enable decimal128 tests.
        // extJSON not implemented for decimal128 without the feature flag, so we must stop here.
        if test.bson_type == "0x13" && !cfg!(feature = "decimal128") {
            continue;
        }

        let cej: serde_json::Value =
            serde_json::from_str(&valid.canonical_extjson).expect(&description);

        // native_to_bson( bson_to_native(cB) ) = cB

        assert_eq!(
            hex::encode(native_to_bson_bson_to_native_cv).to_lowercase(),
            valid.canonical_bson.to_lowercase(),
            "{}",
            description,
        );

        // native_to_canonical_extended_json( bson_to_native(cB) ) = cEJ

        let mut cej_updated_float = cej.clone();

        // Rust doesn't format f64 with exponential notation by default, and the spec doesn't give
        // guidance on when to use it, so we manually parse any $numberDouble fields with
        // exponential notation and replace them with non-exponential notation.
        if let Some(ref key) = test.test_key {
            if let Some(serde_json::Value::Object(subdoc)) = cej_updated_float.get_mut(key) {
                if let Some(&mut serde_json::Value::String(ref mut s)) =
                    subdoc.get_mut("$numberDouble")
                {
                    if s.to_lowercase().contains('e') {
                        let d = f64::from_str(s).unwrap();
                        let mut fixed_string = format!("{}", d);

                        if d.fract() == 0.0 {
                            fixed_string.push_str(".0");
                        }

                        *s = fixed_string;
                    }
                }
            }
        }

        // TODO RUST-36: Enable decimal128 tests.
        if test.bson_type != "0x13" {
            assert_eq!(
                Bson::Document(bson_to_native_cb.clone()).into_canonical_extjson(),
                cej_updated_float,
                "{}",
                description
            );
        }

        // native_to_relaxed_extended_json( bson_to_native(cB) ) = cEJ

        if let Some(ref relaxed_extjson) = valid.relaxed_extjson {
            let rej: serde_json::Value = serde_json::from_str(relaxed_extjson).expect(&description);

            assert_eq!(
                Bson::Document(bson_to_native_cb.clone()).into_relaxed_extjson(),
                rej,
                "{}",
                description
            );
        }

        // native_to_canonical_extended_json( json_to_native(cEJ) ) = cEJ

        let json_to_native_cej: Bson = cej.clone().try_into().expect("cej into bson should work");

        let native_to_canonical_extended_json_bson_to_native_cej =
            json_to_native_cej.clone().into_canonical_extjson();

        assert_eq!(
            native_to_canonical_extended_json_bson_to_native_cej, cej_updated_float,
            "{}",
            description,
        );

        // native_to_bson( json_to_native(cEJ) ) = cB (unless lossy)

        if valid.lossy != Some(true) {
            let mut native_to_bson_json_to_native_cej = Vec::new();
            json_to_native_cej
                .as_document()
                .unwrap()
                .to_writer(&mut native_to_bson_json_to_native_cej)
                .unwrap();

            // TODO RUST-36: Enable decimal128 tests.
            if test.bson_type != "0x13" {
                assert_eq!(
                    hex::encode(native_to_bson_json_to_native_cej).to_lowercase(),
                    valid.canonical_bson.to_lowercase(),
                    "{}",
                    description,
                );
            }
        }

        // native_to_bson( bson_to_native(dB) ) = cB

        if let Some(db) = valid.degenerate_bson {
            let bson_to_native_db =
                Document::from_reader(&mut hex::decode(&db).expect(&description).as_slice())
                    .expect(&description);

            let mut native_to_bson_bson_to_native_db = Vec::new();
            bson_to_native_db
                .to_writer(&mut native_to_bson_bson_to_native_db)
                .unwrap();

            assert_eq!(
                hex::encode(native_to_bson_bson_to_native_db).to_lowercase(),
                valid.canonical_bson.to_lowercase(),
                "{}",
                description,
            );
        }

        if let Some(ref degenerate_extjson) = valid.degenerate_extjson {
            let dej: serde_json::Value =
                serde_json::from_str(degenerate_extjson).expect(&description);

            let json_to_native_dej: Bson = dej.clone().try_into().unwrap();

            // native_to_canonical_extended_json( json_to_native(dEJ) ) = cEJ

            let native_to_canonical_extended_json_json_to_native_dej =
                json_to_native_dej.clone().into_canonical_extjson();

            // TODO RUST-36: Enable decimal128 tests.
            if test.bson_type != "0x13" {
                assert_eq!(
                    native_to_canonical_extended_json_json_to_native_dej, cej,
                    "{}",
                    description,
                );
            }

            // native_to_bson( json_to_native(dEJ) ) = cB (unless lossy)

            if valid.lossy != Some(true) {
                let mut native_to_bson_json_to_native_dej = Vec::new();
                json_to_native_dej
                    .as_document()
                    .unwrap()
                    .to_writer(&mut native_to_bson_json_to_native_dej)
                    .unwrap();

                // TODO RUST-36: Enable decimal128 tests.
                if test.bson_type != "0x13" {
                    assert_eq!(
                        hex::encode(native_to_bson_json_to_native_dej).to_lowercase(),
                        valid.canonical_bson.to_lowercase(),
                        "{}",
                        description,
                    );
                }
            }
        }

        // native_to_relaxed_extended_json( json_to_native(rEJ) ) = rEJ

        if let Some(ref rej) = valid.relaxed_extjson {
            let rej: serde_json::Value = serde_json::from_str(rej).unwrap();

            let json_to_native_rej: Bson = rej.clone().try_into().unwrap();

            let native_to_relaxed_extended_json_bson_to_native_rej =
                json_to_native_rej.clone().into_relaxed_extjson();

            assert_eq!(
                native_to_relaxed_extended_json_bson_to_native_rej, rej,
                "{}",
                description,
            );
        }
    }

    for decode_error in test.decode_errors {
        // No meaningful definition of "byte count" for an arbitrary reader.
        if decode_error.description
            == "Stated length less than byte count, with garbage after envelope"
        {
            continue;
        }

        let bson = hex::decode(decode_error.bson).expect("should decode from hex");
        Document::from_reader(&mut bson.as_slice()).expect_err(decode_error.description.as_str());
    }

    for parse_error in test.parse_errors {
        // TODO RUST-36: Enable decimal128 tests.
        if test.bson_type == "0x13" {
            continue;
        }

        // no special support for dbref convention
        if parse_error.description.contains("DBRef") {
            continue;
        }

        // TODO RUST-36: Enable decimal128 tests.
        if !cfg!(feature = "decimal128") && parse_error.description.contains("$numberDecimal") {
            continue;
        }

        let json: serde_json::Value =
            serde_json::from_str(parse_error.string.as_str()).expect(&parse_error.description);

        Bson::try_from(json).expect_err(&parse_error.description);
    }
}

#[test]
fn run() {
    run_spec_test(&["bson-corpus"], run_test);
}
