use std::convert::TryFrom;

use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    binary::{Binary, PackedBitVector, Vector},
    from_document,
    from_slice,
    spec::BinarySubtype,
    to_document,
    to_raw_document_buf,
    Bson,
    Document,
    RawDocumentBuf,
};

use super::run_spec_test;

const INT8: u8 = 0x03;
const FLOAT32: u8 = 0x27;
const PACKED_BIT: u8 = 0x10;

#[derive(Deserialize)]
struct TestFile {
    description: String,
    test_key: String,
    tests: Vec<Test>,
}

#[derive(Deserialize)]
struct Test {
    description: String,
    valid: bool,
    vector: Option<Vec<Number>>,
    #[serde(
        rename = "dtype_hex",
        deserialize_with = "deserialize_u8_from_hex_string"
    )]
    d_type: u8,
    padding: Option<i16>,
    canonical_bson: Option<String>,
}

fn deserialize_u8_from_hex_string<'de, D>(deserializer: D) -> Result<u8, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    u8::from_str_radix(s.trim_start_matches("0x"), 16).map_err(serde::de::Error::custom)
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Number {
    Int(i16),
    Float(f32),
}

// Some of the invalid cases (e.g. mixed number types, padding for non-packed-bit vectors) are
// impossible to construct, so we return an error from this method.
fn vector_from_numbers(
    numbers: Vec<Number>,
    d_type: u8,
    padding: Option<i16>,
) -> Result<Vector, String> {
    let padding = u8::try_from(padding.unwrap_or(0)).map_err(|e| e.to_string())?;
    if padding != 0 && d_type != PACKED_BIT {
        return Err(format!("got nonzero padding for data type {}", d_type));
    }
    match d_type {
        INT8 => {
            let vector = numbers
                .into_iter()
                .map(|n| match n {
                    Number::Int(n) => i8::try_from(n).map_err(|e| e.to_string()),
                    Number::Float(n) => Err(format!("expected i8, got float {}", n)),
                })
                .collect::<Result<Vec<i8>, String>>()?;
            Ok(Vector::Int8(vector))
        }
        FLOAT32 => {
            let vector = numbers
                .into_iter()
                .map(|n| match n {
                    Number::Int(n) => Err(format!("expected f32, got int {}", n)),
                    Number::Float(n) => Ok(n),
                })
                .collect::<Result<Vec<f32>, String>>()?;
            Ok(Vector::Float32(vector))
        }
        PACKED_BIT => {
            let vector = numbers
                .into_iter()
                .map(|n| match n {
                    Number::Int(n) => u8::try_from(n).map_err(|e| e.to_string()),
                    Number::Float(n) => Err(format!("expected u8, got float {}", n)),
                })
                .collect::<Result<Vec<u8>, String>>()?;
            Ok(Vector::PackedBit(
                PackedBitVector::new(vector, padding).map_err(|e| e.to_string())?,
            ))
        }
        other => Err(format!("invalid data type: {}", other)),
    }
}

// Only return the binary if it represents a valid vector; otherwise, return an error.
fn binary_from_bytes(bson: &str, test_key: &str, description: &str) -> Result<Binary, String> {
    let bytes = hex::decode(bson).expect(description);
    let mut test_document = Document::from_reader(bytes.as_slice()).expect(description);
    let bson = test_document.remove(test_key).expect(description);
    let binary = match bson {
        Bson::Binary(binary) => binary,
        other => panic!("{}: expected binary, got {}", description, other),
    };
    if let Err(error) = Vector::try_from(&binary) {
        Err(error.to_string())
    } else {
        Ok(binary)
    }
}

fn run_test_file(test_file: TestFile) {
    for test in test_file.tests {
        let description = format!("{} ({})", test.description, test_file.description);

        let test_vector = match test.vector {
            Some(vector) => match vector_from_numbers(vector, test.d_type, test.padding) {
                Ok(vector) => {
                    assert!(test.valid, "{}", description);
                    Some(vector)
                }
                Err(error) => {
                    assert!(!test.valid, "{}: {}", description, error);
                    None
                }
            },
            None => None,
        };

        let test_binary = match test.canonical_bson {
            Some(bson) => match binary_from_bytes(&bson, &test_file.test_key, &description) {
                Ok(vector) => {
                    assert!(test.valid, "{}", description);
                    Some(vector)
                }
                Err(error) => {
                    assert!(!test.valid, "{}: {}", description, error);
                    None
                }
            },
            None => None,
        };

        let (Some(test_vector), Some(test_binary)) = (test_vector, test_binary) else {
            return;
        };

        let test_document = doc! { "vector": &test_binary };

        // TryFrom<Binary> for Vector
        let parsed_vector = Vector::try_from(&test_binary).expect(&description);
        assert_eq!(parsed_vector, test_vector);

        // From<Vector> for Binary
        let binary = Binary::from(&test_vector);
        assert_eq!(binary.subtype, BinarySubtype::Vector);
        assert_eq!(binary, test_binary);

        // From<Vector> for Bson
        let document = doc! { "vector": &test_vector };
        assert_eq!(document, test_document);

        // From<Vector> for RawBson
        let raw_document = rawdoc! { "vector": &test_vector };
        let test_raw_document = RawDocumentBuf::from_document(&test_document).expect(&description);
        assert_eq!(raw_document, test_raw_document);

        #[derive(Debug, Deserialize, PartialEq, Serialize)]
        struct Data {
            vector: Vector,
        }
        let data = Data {
            vector: test_vector,
        };

        // Serialize for Vector (Document)
        let serialized_document = to_document(&data).expect(&description);
        assert_eq!(serialized_document, test_document);

        // Deserialize for Vector (Document)
        let deserialized_data: Data = from_document(serialized_document).expect(&description);
        assert_eq!(deserialized_data, data);

        // Serialize for Vector (RawDocumentBuf)
        let serialized_raw_document = to_raw_document_buf(&data).expect(&description);
        assert_eq!(serialized_raw_document, test_raw_document);

        // Deserialize for Vector (RawDocumentBuf)
        let deserialized_data: Data =
            from_slice(serialized_raw_document.as_bytes()).expect(&description);
        assert_eq!(deserialized_data, data);
    }
}

#[test]
fn run_vector_tests() {
    run_spec_test(&["bson-binary-vector"], run_test_file);
}
