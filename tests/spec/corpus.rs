use bson::Bson;
use pretty_assertions::assert_eq;
use serde_derive::Deserialize;

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
    if test.bson_type == "0x13" && !cfg!(feature = "decimal128") {
        return;
    }

    for valid in test.valid {
        let description = format!("{}: {}", test.description, valid.description);

        let bson_to_native_cb = bson::decode_document(
            &mut hex::decode(&valid.canonical_bson)
                .expect(&description)
                .as_slice(),
        )
        .expect(&description);

        let mut native_to_bson_bson_to_native_cv = Vec::new();
        bson::encode_document(&mut native_to_bson_bson_to_native_cv, &bson_to_native_cb)
            .expect(&description);

        let cej: serde_json::Value =
            serde_json::from_str(&valid.canonical_extjson).expect(&description);

        // native_to_bson( bson_to_native(cB) ) = cB

        if !description.contains("1.2345678921232E+18") {
            assert_eq!(
                hex::encode(native_to_bson_bson_to_native_cv).to_lowercase(),
                valid.canonical_bson.to_lowercase(),
                "{}",
                description,
            );
        }

        // native_to_canonical_extended_json( bson_to_native(cB) ) = cEJ

        if !description.contains("1.2345678921232E+18") && test.bson_type != "0x13" {
            assert_eq!(
                Bson::Document(bson_to_native_cb.clone()).into_canonical_extjson(),
                cej,
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

        let json_to_native_cej: Bson = cej.clone().into();

        let native_to_canonical_extended_json_bson_to_native_cej =
            json_to_native_cej.clone().into_canonical_extjson();

        if !description.contains("1.2345678921232E+18") {
            assert_eq!(
                native_to_canonical_extended_json_bson_to_native_cej, cej,
                "{}",
                description,
            );
        }

        // native_to_bson( json_to_native(cEJ) ) = cB (unless lossy)

        if valid.lossy != Some(true) {
            let mut native_to_bson_json_to_native_cej = Vec::new();
            bson::encode_document(
                &mut native_to_bson_json_to_native_cej,
                json_to_native_cej.as_document().unwrap(),
            )
            .unwrap();

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
                bson::decode_document(&mut hex::decode(&db).expect(&description).as_slice())
                    .expect(&description);

            let mut native_to_bson_bson_to_native_db = Vec::new();
            bson::encode_document(&mut native_to_bson_bson_to_native_db, &bson_to_native_db)
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

            let json_to_native_dej: Bson = dej.clone().into();

            // native_to_canonical_extended_json( json_to_native(dEJ) ) = cEJ

            let native_to_canonical_extended_json_json_to_native_dej =
                json_to_native_dej.clone().into_canonical_extjson();

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
                bson::encode_document(
                    &mut native_to_bson_json_to_native_dej,
                    json_to_native_dej.as_document().unwrap(),
                )
                .unwrap();

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

            let json_to_native_rej: Bson = rej.clone().into();

            let native_to_relaxed_extended_json_bson_to_native_rej =
                json_to_native_rej.clone().into_relaxed_extjson();

            assert_eq!(
                native_to_relaxed_extended_json_bson_to_native_rej, rej,
                "{}",
                description,
            );
        }
    }
}

#[test]
fn run() {
    run_spec_test(&["bson-corpus"], run_test);
}
