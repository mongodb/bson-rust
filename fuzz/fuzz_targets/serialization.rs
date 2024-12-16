#![no_main]
use arbitrary::Arbitrary;
use bson::{
    raw::{RawBson, RawBsonRef, RawDocument},
    spec::BinarySubtype,
    Decimal128,
};
use libfuzzer_sys::fuzz_target;
use std::str::FromStr;

fn convert_bson_ref(bson_ref: RawBsonRef) -> Option<RawBson> {
    match bson_ref {
        RawBsonRef::Double(d) => {
            if d.is_nan() {
                Some(RawBsonRef::Double(f64::NAN).to_raw_bson())
            } else {
                Some(RawBsonRef::Double(d).to_raw_bson())
            }
        }
        RawBsonRef::String(s) => {
            if !s.is_empty() && !s.contains('\0') && s.len() <= (i32::MAX as usize) {
                Some(RawBsonRef::String(s).to_raw_bson())
            } else {
                None
            }
        }
        RawBsonRef::Document(d) => {
            let mut valid = true;
            for result in d.iter() {
                match result {
                    Ok((key, _)) if key.is_empty() || key.contains('\0') => {
                        valid = false;
                        break;
                    }
                    Err(_) => {
                        valid = false;
                        break;
                    }
                    _ => {}
                }
            }
            if valid {
                Some(RawBsonRef::Document(d).to_raw_bson())
            } else {
                None
            }
        }
        RawBsonRef::Array(a) => {
            let mut valid = true;
            for result in a.into_iter() {
                if result.is_err() {
                    valid = false;
                    break;
                }
            }
            if valid {
                Some(RawBsonRef::Array(a).to_raw_bson())
            } else {
                None
            }
        }
        RawBsonRef::Binary(b) => {
            if b.bytes.len() <= i32::MAX as usize
                && match b.subtype {
                    BinarySubtype::Generic
                    | BinarySubtype::Function
                    | BinarySubtype::BinaryOld
                    | BinarySubtype::UuidOld
                    | BinarySubtype::Uuid
                    | BinarySubtype::Md5
                    | BinarySubtype::UserDefined(_) => true,
                    _ => false,
                }
            {
                Some(RawBsonRef::Binary(b).to_raw_bson())
            } else {
                None
            }
        }
        RawBsonRef::RegularExpression(regex) => {
            let valid_options = "ilmsux";
            let mut options_sorted = regex.options.chars().collect::<Vec<_>>();
            options_sorted.sort_unstable();
            options_sorted.dedup();
            let sorted_str: String = options_sorted.into_iter().collect();

            if sorted_str.chars().all(|c| valid_options.contains(c))
                && !regex.pattern.contains('\0')
                && regex.pattern.len() <= (i32::MAX as usize)
            {
                Some(RawBsonRef::RegularExpression(regex).to_raw_bson())
            } else {
                None
            }
        }
        RawBsonRef::JavaScriptCode(code) => {
            if !code.is_empty() && !code.contains('\0') && code.len() <= (i32::MAX as usize) {
                Some(RawBsonRef::JavaScriptCode(code).to_raw_bson())
            } else {
                None
            }
        }
        RawBsonRef::JavaScriptCodeWithScope(code_w_scope) => {
            if !code_w_scope.code.is_empty()
                && !code_w_scope.code.contains('\0')
                && code_w_scope.code.len() <= (i32::MAX as usize)
            {
                Some(RawBsonRef::JavaScriptCodeWithScope(code_w_scope).to_raw_bson())
            } else {
                None
            }
        }
        RawBsonRef::DbPointer(ptr) => {
            let raw_bson = RawBsonRef::DbPointer(ptr).to_raw_bson();
            Some(raw_bson)
        }
        RawBsonRef::Symbol(s) => {
            if !s.is_empty() && !s.contains('\0') && s.len() <= i32::MAX as usize {
                Some(RawBsonRef::Symbol(s).to_raw_bson())
            } else {
                None
            }
        }
        RawBsonRef::Decimal128(d) => {
            let d_str = d.to_string();
            if d_str.contains("NaN") {
                if let Ok(nan) = Decimal128::from_str("NaN") {
                    Some(RawBsonRef::Decimal128(nan).to_raw_bson())
                } else {
                    None
                }
            } else if d_str == "Infinity" || d_str == "-Infinity" {
                if let Ok(val) = Decimal128::from_str(&d_str) {
                    Some(RawBsonRef::Decimal128(val).to_raw_bson())
                } else {
                    None
                }
            } else {
                Some(RawBsonRef::Decimal128(d).to_raw_bson())
            }
        }
        other => Some(other.to_raw_bson()),
    }
}

#[derive(Debug, Arbitrary)]
struct Input {
    bytes: Vec<u8>,
}

fuzz_target!(|input: Input| {
    if let Ok(doc) = RawDocument::from_bytes(&input.bytes) {
        for result in doc.iter() {
            if let Ok((key, value)) = result {
                if let Some(converted) = convert_bson_ref(value) {
                    let original_bytes = value.to_raw_bson();
                    match value {
                        RawBsonRef::Double(d) if d.is_nan() => {
                            if let Some(converted_ref) = converted.as_raw_bson_ref().as_f64() {
                                assert!(
                                    converted_ref.is_nan(),
                                    "NaN comparison failed for key: {}",
                                    key
                                );
                            }
                        }
                        RawBsonRef::Double(d) if d.is_infinite() => {
                            if let Some(converted_ref) = converted.as_raw_bson_ref().as_f64() {
                                assert_eq!(
                                    d.is_sign_positive(),
                                    converted_ref.is_sign_positive(),
                                    "Infinity sign mismatch for key: {}",
                                    key
                                );
                                assert!(
                                    converted_ref.is_infinite(),
                                    "Infinity comparison failed for key: {}",
                                    key
                                );
                            }
                        }
                        RawBsonRef::Decimal128(d) if d.to_string().contains("NaN") => {
                            match converted.as_raw_bson_ref() {
                                RawBsonRef::Decimal128(cd) => {
                                    assert!(
                                        cd.to_string().contains("NaN"),
                                        "Decimal128 NaN comparison failed for key: {}",
                                        key
                                    );
                                }
                                _ => panic!(
                                    "Type mismatch: expected Decimal128, got different type for \
                                     key: {}",
                                    key
                                ),
                            }
                        }
                        RawBsonRef::Decimal128(d) if d.to_string().contains("Infinity") => {
                            match converted.as_raw_bson_ref() {
                                RawBsonRef::Decimal128(cd) => {
                                    let d_str = d.to_string();
                                    let cd_str = cd.to_string();
                                    assert_eq!(
                                        d_str, cd_str,
                                        "Decimal128 Infinity comparison failed for key: {}",
                                        key
                                    );
                                }
                                _ => panic!(
                                    "Type mismatch: expected Decimal128, got different type for \
                                     key: {}",
                                    key
                                ),
                            }
                        }
                        _ => {
                            assert_eq!(
                                converted, original_bytes,
                                "Serialization mismatch for key: {}",
                                key
                            );
                        }
                    }
                }
            }
        }
    }
});
