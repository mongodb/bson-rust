use core::str;

use serde::{de::Visitor, Deserialize, Serialize};

use crate::{
    deserialize_from_bson,
    deserialize_from_document,
    deserialize_from_slice,
    doc,
    oid::ObjectId,
    serde_helpers::{self, datetime, object_id, timestamp, u32, u64, HumanReadable},
    serialize_to_bson,
    serialize_to_document,
    spec::BinarySubtype,
    tests::LOCK,
    Binary,
    Bson,
    DateTime,
    Timestamp,
    Utf8Lossy,
};

use serde_with::serde_as;

#[test]
fn human_readable_wrapper() {
    #[derive(PartialEq, Eq, Debug)]
    struct Detector {
        serialized_as: bool,
        deserialized_as: bool,
    }
    impl Detector {
        fn new() -> Self {
            Detector {
                serialized_as: false,
                deserialized_as: false,
            }
        }
    }
    impl Serialize for Detector {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let s = if serializer.is_human_readable() {
                "human readable"
            } else {
                "not human readable"
            };
            serializer.serialize_str(s)
        }
    }
    impl<'de> Deserialize<'de> for Detector {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            struct V;
            impl Visitor<'_> for V {
                type Value = bool;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    formatter.write_str("Detector")
                }

                fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    match v {
                        "human readable" => Ok(true),
                        "not human readable" => Ok(false),
                        _ => Err(E::custom(format!("invalid detector string {:?}", v))),
                    }
                }
            }
            let deserialized_as = deserializer.is_human_readable();
            let serialized_as = deserializer.deserialize_str(V)?;
            Ok(Detector {
                serialized_as,
                deserialized_as,
            })
        }
    }
    #[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
    struct Data {
        first: HumanReadable<Detector>,
        outer: Detector,
        wrapped: HumanReadable<Detector>,
        inner: HumanReadable<SubData>,
    }
    #[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
    struct SubData {
        value: Detector,
    }

    let data = Data {
        first: HumanReadable(Detector::new()),
        outer: Detector::new(),
        wrapped: HumanReadable(Detector::new()),
        inner: HumanReadable(SubData {
            value: Detector::new(),
        }),
    };
    // use the raw serializer, which is non-human-readable
    let data_doc = crate::serialize_to_raw_document_buf(&data).unwrap();
    let expected_data_doc = rawdoc! {
        "first": "human readable",
        "outer": "not human readable",
        "wrapped": "human readable",
        "inner": {
            "value": "human readable",
        }
    };
    assert_eq!(data_doc, expected_data_doc);

    let tripped: Data = crate::deserialize_from_slice(expected_data_doc.as_bytes()).unwrap();
    let expected = Data {
        first: HumanReadable(Detector {
            serialized_as: true,
            deserialized_as: true,
        }),
        outer: Detector {
            serialized_as: false,
            deserialized_as: false,
        },
        wrapped: HumanReadable(Detector {
            serialized_as: true,
            deserialized_as: true,
        }),
        inner: HumanReadable(SubData {
            value: Detector {
                serialized_as: true,
                deserialized_as: true,
            },
        }),
    };
    assert_eq!(&tripped, &expected);
}

#[test]
#[allow(dead_code)] // suppress warning for unread fields
fn utf8_lossy_wrapper() {
    let invalid_bytes = b"\x80\xae".to_vec();
    let invalid_string = unsafe { String::from_utf8_unchecked(invalid_bytes) };

    let both_strings_invalid_bytes =
        rawdoc! { "s1": invalid_string.clone(), "s2": invalid_string.clone() }.into_bytes();
    let first_string_invalid_bytes =
        rawdoc! { "s1": invalid_string.clone(), "s2": ":)" }.into_bytes();

    let expected_replacement = "��".to_string();

    #[derive(Debug, Deserialize)]
    struct NoUtf8Lossy {
        s1: String,
        s2: String,
    }

    deserialize_from_slice::<NoUtf8Lossy>(&both_strings_invalid_bytes).unwrap_err();

    let s = deserialize_from_slice::<Utf8Lossy<NoUtf8Lossy>>(&both_strings_invalid_bytes)
        .unwrap()
        .0;
    assert_eq!(s.s1, expected_replacement);
    assert_eq!(s.s2, expected_replacement);

    #[derive(Debug, Deserialize)]
    struct FirstStringUtf8Lossy {
        s1: Utf8Lossy<String>,
        s2: String,
    }

    let s = deserialize_from_slice::<FirstStringUtf8Lossy>(&first_string_invalid_bytes).unwrap();
    assert_eq!(s.s1.0, expected_replacement);
    assert_eq!(&s.s2, ":)");

    deserialize_from_slice::<FirstStringUtf8Lossy>(&both_strings_invalid_bytes).unwrap_err();

    let s = deserialize_from_slice::<Utf8Lossy<FirstStringUtf8Lossy>>(&both_strings_invalid_bytes)
        .unwrap()
        .0;
    assert_eq!(s.s1.0, expected_replacement);
    assert_eq!(s.s2, expected_replacement);
}
#[test]
#[cfg(feature = "serde_with-3")]
fn test_u64_i32_helper() {
    let _guard = LOCK.run_concurrently();

    #[serde_as]
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "u64::AsI32")]
        value: u64,

        #[serde_as(as = "Option<u64::AsI32>")]
        value_optional_none: Option<u64>,

        #[serde_as(as = "Option<u64::AsI32>")]
        value_optional_some: Option<u64>,

        #[serde_as(as = "Vec<u64::AsI32>")]
        value_vector: Vec<u64>,
    }

    let value = 1;
    let a = A {
        value,
        value_optional_none: None,
        value_optional_some: Some(value),
        value_vector: vec![value],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get_i32("value").unwrap(),
        value as i32,
        "Expected serialized value to match original."
    );

    assert_eq!(
        doc.get("value_optional_none"),
        Some(&Bson::Null),
        "Expected serialized value_optional_none to be None."
    );

    assert_eq!(
        doc.get("value_optional_some"),
        Some(&Bson::Int32(value as i32)),
        "Expected serialized value_optional_some to match original."
    );

    let value_vector = doc
        .get_array("value_vector")
        .expect("Expected serialized value_vector to be a BSON array.");
    let expected_value_vector: Vec<Bson> = vec![Bson::Int32(value as i32)];
    assert_eq!(
        value_vector, &expected_value_vector,
        "Expected each serialized element in value_vector to match the original."
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Round-trip failed: deserialized struct did not match original."
    );

    // Validate serialization fails because i32::MAX + 1 is too large to fit in i32
    let invalid_value_for_serializing = i32::MAX as u64 + 1;
    let bad_a: A = A {
        value: invalid_value_for_serializing,
        value_optional_none: None,
        value_optional_some: Some(invalid_value_for_serializing),
        value_vector: vec![invalid_value_for_serializing],
    };
    let result = serialize_to_document(&bad_a);
    assert!(
        result.is_err(),
        "Serialization should fail for u64::MAX since it can't be exactly represented as i32"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("Cannot convert u64"),
        "Expected error message to mention failed u64 to i32 conversion, got: {}",
        err_string
    );

    // Validate deserialization fails for i32::MIN because negative values can't be converted to
    // u64
    let invalid_value_for_deserializing = i32::MIN;
    let bad_a = doc! {
        "value": invalid_value_for_deserializing,
        "value_optional_none": Bson::Null,
        "value_optional_some": Some(invalid_value_for_deserializing),
        "value_vector": [invalid_value_for_deserializing],
    };
    let result: Result<A, _> = deserialize_from_document(bad_a);
    assert!(
        result.is_err(),
        "Deserialization should fail for i32::MIN since it can't be exactly represented as u64"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("Cannot convert i32"),
        "Expected error message to mention failed i32 to u64 conversion, got: {}",
        err_string
    );
}
#[test]
fn test_binary_helper_generic_roundtrip() {
    let _guard = LOCK.run_concurrently();
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub struct Foo {
        data: Binary,
    }

    let x = Foo {
        data: Binary {
            subtype: BinarySubtype::Generic,
            bytes: b"12345abcde".to_vec(),
        },
    };

    let b = serialize_to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::Generic, bytes: b"12345abcde".to_vec() })}
    );

    let f = deserialize_from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}

#[test]
fn test_binary_helper_non_generic_roundtrip() {
    let _guard = LOCK.run_concurrently();
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    pub struct Foo {
        data: Binary,
    }

    let x = Foo {
        data: Binary {
            subtype: BinarySubtype::BinaryOld,
            bytes: b"12345abcde".to_vec(),
        },
    };

    let b = serialize_to_bson(&x).unwrap();
    assert_eq!(
        b.as_document().unwrap(),
        &doc! {"data": Bson::Binary(Binary { subtype: BinarySubtype::BinaryOld, bytes: b"12345abcde".to_vec() })}
    );

    let f = deserialize_from_bson::<Foo>(b).unwrap();
    assert_eq!(x, f);
}

#[test]
#[cfg(feature = "serde_with-3")]
fn test_oid_helpers() {
    let _guard = LOCK.run_concurrently();

    #[serde_as]
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "object_id::AsHexString")]
        oid: ObjectId,

        #[serde_as(as = "Option<object_id::AsHexString>")]
        oid_optional_none: Option<ObjectId>,

        #[serde_as(as = "Option<object_id::AsHexString>")]
        oid_optional_some: Option<ObjectId>,

        #[serde_as(as = "Vec<object_id::AsHexString>")]
        oid_vector: Vec<ObjectId>,
    }

    let oid = ObjectId::new();
    let a = A {
        oid,
        oid_optional_none: None,
        oid_optional_some: Some(oid),
        oid_vector: vec![oid],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get_str("oid").unwrap(),
        oid.to_hex(),
        "Expected serialized oid to match original ObjectId as hex string."
    );

    assert_eq!(
        doc.get("oid_optional_none"),
        Some(&Bson::Null),
        "Expected serialized oid_optional_none to be None."
    );

    assert_eq!(
        doc.get("oid_optional_some"),
        Some(&Bson::String(oid.to_hex())),
        "Expected serialized oid_optional_some to match original."
    );

    let oid_vector = doc
        .get_array("oid_vector")
        .expect("Expected serialized oid_vector to be a BSON array.");
    let expected_oid_vector: Vec<Bson> = vec![Bson::String(oid.to_hex())];
    assert_eq!(
        oid_vector, &expected_oid_vector,
        "Expected each serialized element in oid_vector match the original."
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Deserialized struct does not match original."
    );

    // Validate deserializing error case with an invalid ObjectId string
    let invalid_doc = doc! {
        "oid": "not_a_valid_oid",
        "oid_optional_none": Bson::Null,
        "oid_optional_some": "also_invalid_oid",
        "oid_vector": ["bad1", "bad2"]
    };
    let result: Result<A, _> = deserialize_from_document(invalid_doc);
    assert!(
        result.is_err(),
        "Deserialization should fail for invalid ObjectId strings"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("BSON error"),
        "Expected error message to mention BSON error: {}",
        err_string
    );

    #[serde_as]
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct B {
        #[serde_as(as = "object_id::FromHexString")]
        oid: String,

        #[serde_as(as = "Option<object_id::FromHexString>")]
        oid_optional_none: Option<String>,

        #[serde_as(as = "Option<object_id::FromHexString>")]
        oid_optional_some: Option<String>,

        #[serde_as(as = "Vec<object_id::FromHexString>")]
        oid_vector: Vec<String>,
    }

    let oid = ObjectId::new();
    let b = B {
        oid: oid.to_string(),
        oid_optional_none: None,
        oid_optional_some: Some(oid.to_string()),
        oid_vector: vec![oid.to_string()],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&b).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get_object_id("oid").unwrap(),
        oid,
        "Expected serialized oid to match original ObjectId."
    );

    assert_eq!(
        doc.get("oid_optional_none"),
        Some(&Bson::Null),
        "Expected serialized oid_optional_none to be None."
    );

    assert_eq!(
        doc.get("oid_optional_some"),
        Some(&Bson::ObjectId(oid)),
        "Expected serialized oid_optional_some to match original."
    );

    let oid_vector = doc
        .get_array("oid_vector")
        .expect("Expected serialized oid_vector to be a BSON array.");
    let expected_oid_vector: Vec<Bson> = vec![Bson::ObjectId(oid)];
    assert_eq!(
        oid_vector, &expected_oid_vector,
        "Expected each serialized element in oid_vector match the original."
    );

    // Validate deserialized data
    let b_deserialized: B = deserialize_from_document(doc).unwrap();
    assert_eq!(
        b_deserialized, b,
        "Deserialized struct does not match original."
    );

    // Validate serializing error case with an invalid ObjectId string
    let invalid_oid = "invalid_oid";
    let bad_b = B {
        oid: invalid_oid.to_string(),
        oid_optional_none: None,
        oid_optional_some: Some(invalid_oid.to_string()),
        oid_vector: vec![invalid_oid.to_string()],
    };
    let result = serialize_to_document(&bad_b);
    assert!(
        result.is_err(),
        "Serialization should fail for invalid ObjectId strings"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("BSON error"),
        "Expected error message to mention BSON error: {}",
        err_string
    );
}

#[test]
#[cfg(feature = "serde_with-3")]
fn test_datetime_rfc3339_string_helpers() {
    let _guard = LOCK.run_concurrently();

    #[serde_as]
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "datetime::AsRfc3339String")]
        pub date: DateTime,

        #[serde_as(as = "Option<datetime::AsRfc3339String>")]
        pub date_optional_none: Option<DateTime>,

        #[serde_as(as = "Option<datetime::AsRfc3339String>")]
        pub date_optional_some: Option<DateTime>,

        #[serde_as(as = "Vec<datetime::AsRfc3339String>")]
        pub date_vector: Vec<DateTime>,
    }

    let iso = "1996-12-20T00:39:57Z";
    let date = DateTime::parse_rfc3339_str(iso).unwrap();
    let a = A {
        date,
        date_optional_none: None,
        date_optional_some: Some(date),
        date_vector: vec![date],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get_str("date").unwrap(),
        iso,
        "Expected serialized date to match original date from RFC 3339 string."
    );

    assert_eq!(
        doc.get("date_optional_none"),
        Some(&Bson::Null),
        "Expected serialized date_optional_none to be None."
    );

    assert_eq!(
        doc.get("date_optional_some"),
        Some(&Bson::String(iso.to_string())),
        "Expected serialized date_optional_some to match original."
    );

    let date_vector = doc
        .get_array("date_vector")
        .expect("Expected serialized date_vector to be a BSON array.");
    let expected_date_vector: Vec<Bson> = vec![Bson::String(date.try_to_rfc3339_string().unwrap())];
    assert_eq!(
        date_vector, &expected_date_vector,
        "Expected each serialized element in date_vector to match the original."
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Deserialized struct does not match original."
    );

    // Validate deserializing error case with an invalid DateTime string
    let invalid_doc = doc! {
        "date": "not_a_valid_date",
        "date_optional_none": Bson::Null,
        "date_optional_some": "also_invalid_date",
        "date_vector": ["bad1", "bad2"]
    };
    let result: Result<A, _> = deserialize_from_document(invalid_doc);
    assert!(
        result.is_err(),
        "Deserialization should fail for invalid DateTime strings"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("BSON error"),
        "Expected error message to mention BSON error: {}",
        err_string
    );

    #[serde_as]
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct B {
        #[serde_as(as = "datetime::FromRfc3339String")]
        pub date: String,

        #[serde_as(as = "Option<datetime::FromRfc3339String>")]
        pub date_optional_none: Option<String>,

        #[serde_as(as = "Option<datetime::FromRfc3339String>")]
        pub date_optional_some: Option<String>,

        #[serde_as(as = "Vec<datetime::FromRfc3339String>")]
        pub date_vector: Vec<String>,
    }

    let date = DateTime::now();
    let b = B {
        date: date.try_to_rfc3339_string().unwrap(),
        date_optional_none: None,
        date_optional_some: Some(date.try_to_rfc3339_string().unwrap()),
        date_vector: vec![date.try_to_rfc3339_string().unwrap()],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&b).unwrap();

    // Validate serialized data
    assert_eq!(
        *doc.get_datetime("date").unwrap(),
        date,
        "Expected serialized date to be a BSON DateTime."
    );

    assert_eq!(
        doc.get("date_optional_none"),
        Some(&Bson::Null),
        "Expected serialized date_optional_none to be None."
    );

    assert_eq!(
        doc.get("date_optional_some"),
        Some(&Bson::DateTime(date)),
        "Expected serialized date_optional_some to match original."
    );

    let date_vector = doc
        .get_array("date_vector")
        .expect("Expected serialized date_vector to be a BSON array.");
    let expected_date_vector: Vec<Bson> = vec![Bson::DateTime(date)];
    assert_eq!(
        date_vector, &expected_date_vector,
        "Expected each serialized element in date_vector match the original."
    );

    // Validate deserialized data
    let b_deserialized: B = deserialize_from_document(doc).unwrap();
    assert_eq!(
        b_deserialized, b,
        "Deserialized struct does not match original."
    );

    // Validate serializing error case with an invalid DateTime string
    let invalid_date = "invalid_date";
    let bad_b = B {
        date: invalid_date.to_string(),
        date_optional_none: None,
        date_optional_some: Some(invalid_date.to_string()),
        date_vector: vec![invalid_date.to_string()],
    };
    let result = serialize_to_document(&bad_b);
    assert!(
        result.is_err(),
        "Serialization should fail for invalid DateTime strings"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("BSON error"),
        "Expected error message to mention BSON error: {}",
        err_string
    );
}

#[test]
#[cfg(feature = "serde_with-3")]
fn test_datetime_i64_helper() {
    let _guard = LOCK.run_concurrently();

    #[serde_as]
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "datetime::FromI64")]
        date: i64,

        #[serde_as(as = "Option<datetime::FromI64>")]
        date_optional_none: Option<i64>,

        #[serde_as(as = "Option<datetime::FromI64>")]
        date_optional_some: Option<i64>,

        #[serde_as(as = "Vec<datetime::FromI64>")]
        date_vector: Vec<i64>,
    }

    let date = DateTime::now();
    let a = A {
        date: date.timestamp_millis(),
        date_optional_none: None,
        date_optional_some: Some(date.timestamp_millis()),
        date_vector: vec![date.timestamp_millis()],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get_datetime("date").unwrap(),
        &date,
        "Expected serialized date to match original date."
    );

    assert_eq!(
        doc.get("date_optional_none"),
        Some(&Bson::Null),
        "Expected serialized date_optional_none to be None."
    );

    assert_eq!(
        doc.get("date_optional_some"),
        Some(&Bson::DateTime(date)),
        "Expected serialized date_optional_some to match original."
    );

    let date_vector = doc
        .get_array("date_vector")
        .expect("Expected serialized date_vector to be a BSON array.");
    let expected_date_vector: Vec<Bson> = vec![Bson::DateTime(date)];
    assert_eq!(
        date_vector, &expected_date_vector,
        "Expected each serialized element in date_vector match the original."
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Deserialized struct does not match original."
    );
}

#[test]
#[cfg(all(feature = "chrono-0_4", feature = "serde_with-3"))]
fn test_datetime_chrono04_datetime_helper() {
    let _guard = LOCK.run_concurrently();

    use std::str::FromStr;

    #[serde_as]
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "datetime::FromChrono04DateTime")]
        pub date: chrono::DateTime<chrono::Utc>,

        #[serde_as(as = "Option<datetime::FromChrono04DateTime>")]
        pub date_optional_none: Option<chrono::DateTime<chrono::Utc>>,

        #[serde_as(as = "Option<datetime::FromChrono04DateTime>")]
        pub date_optional_some: Option<chrono::DateTime<chrono::Utc>>,

        #[serde_as(as = "Vec<datetime::FromChrono04DateTime>")]
        pub date_vector: Vec<chrono::DateTime<chrono::Utc>>,
    }

    let iso = "1996-12-20T00:39:57Z";
    let date: chrono::DateTime<chrono::Utc> = chrono::DateTime::from_str(iso).unwrap();
    let a: A = A {
        date,
        date_optional_none: None,
        date_optional_some: Some(date),
        date_vector: vec![date],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get_datetime("date").unwrap().to_chrono(),
        date,
        "Expected serialized date to match original date."
    );

    assert_eq!(
        doc.get("date_optional_none"),
        Some(&Bson::Null),
        "Expected serialized date_optional_none to be None."
    );

    assert_eq!(
        doc.get("date_optional_some"),
        Some(&Bson::DateTime(DateTime::from_chrono(date))),
        "Expected serialized date_optional_some to match original."
    );

    let date_vector = doc
        .get_array("date_vector")
        .expect("Expected serialized date_vector to be a BSON array.");
    let expected_date_vector: Vec<Bson> = vec![Bson::DateTime(date.into())];
    assert_eq!(
        date_vector, &expected_date_vector,
        "Expected each serialized element in date_vector to be a BSON DateTime matching the \
         original."
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Deserialized struct does not match original."
    );
}

#[test]
#[cfg(all(feature = "jiff-0_2", feature = "serde_with-3"))]
fn test_datetime_jiff02_timestamp_helper() {
    let _guard = LOCK.run_concurrently();

    use std::str::FromStr;

    #[serde_as]
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "datetime::FromJiff02Timestamp")]
        pub date: jiff::Timestamp,

        #[serde_as(as = "Option<datetime::FromJiff02Timestamp>")]
        pub date_optional_none: Option<jiff::Timestamp>,

        #[serde_as(as = "Option<datetime::FromJiff02Timestamp>")]
        pub date_optional_some: Option<jiff::Timestamp>,

        #[serde_as(as = "Vec<datetime::FromJiff02Timestamp>")]
        pub date_vector: Vec<jiff::Timestamp>,
    }

    let iso = "1996-12-20T00:39:57Z";
    let date: jiff::Timestamp = jiff::Timestamp::from_str(iso).unwrap();
    let a: A = A {
        date,
        date_optional_none: None,
        date_optional_some: Some(date),
        date_vector: vec![date],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get_datetime("date").unwrap().to_jiff(),
        date,
        "Expected serialized date to match original date."
    );

    assert_eq!(
        doc.get("date_optional_none"),
        Some(&Bson::Null),
        "Expected serialized date_optional_none to be None."
    );

    assert_eq!(
        doc.get("date_optional_some"),
        Some(&Bson::DateTime(DateTime::from_jiff(date))),
        "Expected serialized date_optional_some to match original."
    );

    let date_vector = doc
        .get_array("date_vector")
        .expect("Expected serialized date_vector to be a BSON array.");
    let expected_date_vector: Vec<Bson> = vec![Bson::DateTime(date.into())];
    assert_eq!(
        date_vector, &expected_date_vector,
        "Expected each serialized element in date_vector to be a BSON DateTime matching the \
         original."
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Deserialized struct does not match original."
    );
}

#[test]
#[cfg(all(feature = "time-0_3", feature = "serde_with-3"))]
fn test_datetime_time03_offset_datetime_helper() {
    use time::OffsetDateTime;
    let _guard = LOCK.run_concurrently();

    #[serde_as]
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "datetime::FromTime03OffsetDateTime")]
        pub date: OffsetDateTime,

        #[serde_as(as = "Option<datetime::FromTime03OffsetDateTime>")]
        pub date_optional_none: Option<OffsetDateTime>,

        #[serde_as(as = "Option<datetime::FromTime03OffsetDateTime>")]
        pub date_optional_some: Option<OffsetDateTime>,

        #[serde_as(as = "Vec<datetime::FromTime03OffsetDateTime>")]
        pub date_vector: Vec<OffsetDateTime>,
    }

    let date = DateTime::now();
    let a: A = A {
        date: date.to_time_0_3(),
        date_optional_none: None,
        date_optional_some: Some(date.to_time_0_3()),
        date_vector: vec![date.to_time_0_3()],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get_datetime("date").unwrap(),
        &date,
        "Expected serialized date to match original date."
    );

    assert_eq!(
        doc.get("date_optional_none"),
        Some(&Bson::Null),
        "Expected serialized date_optional_none to be None."
    );

    assert_eq!(
        doc.get("date_optional_some"),
        Some(&Bson::DateTime(date)),
        "Expected serialized date_optional_some to match original."
    );

    let date_vector = doc
        .get_array("date_vector")
        .expect("Expected serialized date_vector to be a BSON array.");
    let expected_date_vector: Vec<Bson> = vec![Bson::DateTime(date)];
    assert_eq!(
        date_vector, &expected_date_vector,
        "Expected each serialized element in date_vector to be a BSON DateTime matching the \
         original."
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Deserialized struct does not match original."
    );
}

#[test]
#[cfg(feature = "serde_with-3")]
fn test_timestamp_u32_helpers() {
    let _guard = LOCK.run_concurrently();

    #[serde_as]
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "timestamp::AsU32")]
        pub timestamp: Timestamp,

        #[serde_as(as = "Option<timestamp::AsU32>")]
        pub timestamp_optional_none: Option<Timestamp>,

        #[serde_as(as = "Option<timestamp::AsU32>")]
        pub timestamp_optional_some: Option<Timestamp>,

        #[serde_as(as = "Vec<timestamp::AsU32>")]
        pub timestamp_vector: Vec<Timestamp>,
    }

    let time = 12345;
    let timestamp = Timestamp { time, increment: 0 };
    let a = A {
        timestamp,
        timestamp_optional_none: None,
        timestamp_optional_some: Some(timestamp),
        timestamp_vector: vec![timestamp],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get("timestamp").unwrap(),
        &Bson::Int64(time as i64),
        "Expected serialized time to match the original."
    );

    assert_eq!(
        doc.get("timestamp_optional_none"),
        Some(&Bson::Null),
        "Expected serialized timestamp_optional_none to be None."
    );

    assert_eq!(
        doc.get("timestamp_optional_some"),
        Some(&Bson::Int64(time as i64)),
        "Expected serialized timestamp_optional_some to match original time."
    );

    let timestamp_vector = doc
        .get_array("timestamp_vector")
        .expect("Expected serialized timestamp_vector to be a BSON array.");
    let expected_timestamp_vector: Vec<Bson> = vec![Bson::Int64(time as i64)];
    assert_eq!(
        timestamp_vector, &expected_timestamp_vector,
        "Expected each serialized element in timestamp_vector to match the original."
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Deserialized struct does not match original."
    );

    // Validate serializing error case with an invalid Timestamp
    let invalid_timestamp_for_serializing = Timestamp {
        time: 0,
        increment: 2,
    };
    let bad_a: A = A {
        timestamp: invalid_timestamp_for_serializing,
        timestamp_optional_none: None,
        timestamp_optional_some: Some(invalid_timestamp_for_serializing),
        timestamp_vector: vec![invalid_timestamp_for_serializing],
    };
    let result = serialize_to_document(&bad_a);
    assert!(
        result.is_err(),
        "Serialization should fail for Timestamp with increment != 0"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("Cannot convert Timestamp with a non-zero increment to u32"),
        "Expected error message to mention non-zero increment: {}",
        err_string
    );

    #[serde_as]
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct B {
        #[serde_as(as = "timestamp::FromU32")]
        pub time: u32,

        #[serde_as(as = "Option<timestamp::FromU32>")]
        pub time_optional_none: Option<u32>,

        #[serde_as(as = "Option<timestamp::FromU32>")]
        pub time_optional_some: Option<u32>,

        #[serde_as(as = "Vec<timestamp::FromU32>")]
        pub time_vector: Vec<u32>,
    }

    let time = 12345;
    let b = B {
        time,
        time_optional_none: None,
        time_optional_some: Some(time),
        time_vector: vec![time],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&b).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get_timestamp("time").unwrap(),
        Timestamp { time, increment: 0 },
        "Expected serialized time to match the original."
    );

    assert_eq!(
        doc.get("time_optional_none"),
        Some(&Bson::Null),
        "Expected serialized time_optional_none to be None."
    );

    assert_eq!(
        doc.get("time_optional_some"),
        Some(&Bson::Timestamp(Timestamp { time, increment: 0 })),
        "Expected serialized time_optional_some to match original."
    );

    let time_vector = doc
        .get_array("time_vector")
        .expect("Expected serialized time_vector to be a BSON array.");
    let expected_time_vector: Vec<Bson> = vec![Bson::Timestamp(Timestamp { time, increment: 0 })];
    assert_eq!(
        time_vector, &expected_time_vector,
        "Expected each serialized element in time_vector to match the original."
    );

    // Validate deserialized data
    let b_deserialized: B = deserialize_from_document(doc).unwrap();
    assert_eq!(
        b_deserialized, b,
        "Deserialized struct does not match original."
    );

    // Validate deserializing error case with an invalid Timestamp
    let invalid_timestamp_for_deserializing = Timestamp {
        time: 0,
        increment: 2,
    };
    let invalid_doc = doc! {
        "time": invalid_timestamp_for_deserializing,
        "time_optional_none": Bson::Null,
        "time_optional_some": Some(invalid_timestamp_for_deserializing),
        "time_vector": [invalid_timestamp_for_deserializing]
    };
    let result: Result<B, _> = deserialize_from_document(invalid_doc);
    assert!(
        result.is_err(),
        "Deserialization should fail for Timestamp with increment != 0"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("Cannot convert Timestamp with a non-zero increment to u32"),
        "Expected error message to mention non-zero increment: {}",
        err_string
    );
}

#[test]
#[cfg(feature = "serde_with-3")]
fn test_u32_f64_helper() {
    let _guard = LOCK.run_concurrently();

    #[serde_as]
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "u32::AsF64")]
        pub value: u32,

        #[serde_as(as = "Option<u32::AsF64>")]
        pub value_optional_none: Option<u32>,

        #[serde_as(as = "Option<u32::AsF64>")]
        pub value_optional_some: Option<u32>,

        #[serde_as(as = "Vec<u32::AsF64>")]
        pub value_vector: Vec<u32>,
    }

    let value = 12345;
    let a = A {
        value,
        value_optional_none: None,
        value_optional_some: Some(value),
        value_vector: vec![value],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get("value"),
        Some(&Bson::Double(value as f64)),
        "Expected serialized value to match the original."
    );

    assert_eq!(
        doc.get("value_optional_none"),
        Some(&Bson::Null),
        "Expected serialized value_optional_none to be None."
    );

    assert_eq!(
        doc.get("value_optional_some"),
        Some(&Bson::Double(value as f64)),
        "Expected serialized value_optional_some to match original."
    );

    let value_vector = doc
        .get_array("value_vector")
        .expect("Expected serialized value_vector to be a BSON array.");
    let expected_value_vector: Vec<Bson> = vec![Bson::Double(value as f64)];
    assert_eq!(
        value_vector, &expected_value_vector,
        "Expected each serialized element in value_vector to match the original."
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Deserialized struct does not match original."
    );
}

#[test]
#[cfg(feature = "serde_with-3")]
fn test_u32_i32_helper() {
    let _guard = LOCK.run_concurrently();

    #[serde_as]
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "u32::AsI32")]
        value: u32,

        #[serde_as(as = "Option<u32::AsI32>")]
        value_optional_none: Option<u32>,

        #[serde_as(as = "Option<u32::AsI32>")]
        value_optional_some: Option<u32>,

        #[serde_as(as = "Vec<u32::AsI32>")]
        value_vector: Vec<u32>,
    }

    let value = 1;
    let a = A {
        value,
        value_optional_none: None,
        value_optional_some: Some(value),
        value_vector: vec![value],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get_i32("value").unwrap(),
        value as i32,
        "Expected serialized value to match original."
    );

    assert_eq!(
        doc.get("value_optional_none"),
        Some(&Bson::Null),
        "Expected serialized value_optional_none to be None."
    );

    assert_eq!(
        doc.get("value_optional_some"),
        Some(&Bson::Int32(value as i32)),
        "Expected serialized value_optional_some to match original."
    );

    let value_vector = doc
        .get_array("value_vector")
        .expect("Expected serialized value_vector to be a BSON array.");
    let expected_value_vector: Vec<Bson> = vec![Bson::Int32(value as i32)];
    assert_eq!(
        value_vector, &expected_value_vector,
        "Expected each serialized element in value_vector to match the original."
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Deserialized struct does not match original."
    );

    // Validate serialization fails because u32::MAX is too large to fit in i32
    let invalid_value_for_serializing = u32::MAX;
    let bad_a: A = A {
        value: invalid_value_for_serializing,
        value_optional_none: None,
        value_optional_some: Some(invalid_value_for_serializing),
        value_vector: vec![invalid_value_for_serializing],
    };
    let result = serialize_to_document(&bad_a);
    assert!(
        result.is_err(),
        "Serialization should fail for u32::MAX since it can't be exactly represented as i32"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("Cannot convert u32"),
        "Expected error message to mention failed u32 to i32 conversion, got: {}",
        err_string
    );

    // Validate deserialization fails for i32::MIN because negative values can't be converted to
    // u32
    let invalid_value_for_deserializing = i32::MIN;
    let bad_a = doc! {
        "value": invalid_value_for_deserializing,
        "value_optional_none": Bson::Null,
        "value_optional_some": Some(invalid_value_for_deserializing),
        "value_vector": [invalid_value_for_deserializing],
    };
    let result: Result<A, _> = deserialize_from_document(bad_a);
    assert!(
        result.is_err(),
        "Deserialization should fail for i32::MIN since it can't be exactly represented as u32"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("Cannot convert i32"),
        "Expected error message to mention failed i32 to u32 conversion, got: {}",
        err_string
    );
}

#[test]
#[cfg(feature = "serde_with-3")]
fn test_u32_i64_helper() {
    let _guard = LOCK.run_concurrently();

    #[serde_as]
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "u32::AsI64")]
        value: u32,

        #[serde_as(as = "Option<u32::AsI64>")]
        value_optional_none: Option<u32>,

        #[serde_as(as = "Option<u32::AsI64>")]
        value_optional_some: Option<u32>,

        #[serde_as(as = "Vec<u32::AsI64>")]
        value_vector: Vec<u32>,
    }

    let value = u32::MAX;
    let a = A {
        value,
        value_optional_none: None,
        value_optional_some: Some(value),
        value_vector: vec![value],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get_i64("value").unwrap(),
        value as i64,
        "Expected serialized value to match original."
    );

    assert_eq!(
        doc.get("value_optional_none"),
        Some(&Bson::Null),
        "Expected serialized value_optional_none to be None."
    );

    assert_eq!(
        doc.get("value_optional_some"),
        Some(&Bson::Int64(value as i64)),
        "Expected serialized value_optional_some to match original."
    );

    let value_vector = doc
        .get_array("value_vector")
        .expect("Expected serialized value_vector to be a BSON array.");
    let expected_value_vector: Vec<Bson> = vec![Bson::Int64(value as i64)];
    assert_eq!(
        value_vector, &expected_value_vector,
        "Expected each serialized element in value_vector to match the original."
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Round-trip failed: deserialized struct did not match original."
    );

    // Validate deserialization fails for i64::MIN because negative values can't be converted to
    // u32
    let invalid_value_for_deserializing = i64::MIN;
    let bad_a = doc! {
        "value": invalid_value_for_deserializing,
        "value_optional_none": Bson::Null,
        "value_optional_some": Some(invalid_value_for_deserializing),
        "value_vector": [invalid_value_for_deserializing],
    };
    let result: Result<A, _> = deserialize_from_document(bad_a);
    assert!(
        result.is_err(),
        "Deserialization should fail for i64::MIN since it can't be exactly represented as u32"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("Cannot convert i64"),
        "Expected error message to mention failed i64 to u32 conversion, got: {}",
        err_string
    );
}

#[test]
#[cfg(feature = "serde_with-3")]
fn test_u64_f64_helper() {
    let _guard = LOCK.run_concurrently();

    #[serde_as]
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "u64::AsF64")]
        pub value: u64,

        #[serde_as(as = "Option<u64::AsF64>")]
        pub value_optional_none: Option<u64>,

        #[serde_as(as = "Option<u64::AsF64>")]
        pub value_optional_some: Option<u64>,

        #[serde_as(as = "Vec<u64::AsF64>")]
        pub value_vector: Vec<u64>,
    }

    let value = 12345;
    let a = A {
        value,
        value_optional_none: None,
        value_optional_some: Some(value),
        value_vector: vec![value],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get("value"),
        Some(&Bson::Double(value as f64)),
        "Expected serialized value to match the original."
    );

    assert_eq!(
        doc.get("value_optional_none"),
        Some(&Bson::Null),
        "Expected serialized value_optional_none to be None."
    );

    assert_eq!(
        doc.get("value_optional_some"),
        Some(&Bson::Double(value as f64)),
        "Expected serialized value_optional_some to match original."
    );

    let value_vector = doc
        .get_array("value_vector")
        .expect("Expected serialized value_vector to be a BSON array.");
    let expected_value_vector: Vec<Bson> = vec![Bson::Double(value as f64)];
    assert_eq!(
        value_vector, &expected_value_vector,
        "Expected each serialized element in value_vector to match the original."
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Deserialized struct does not match original."
    );

    // Validate serializing error case with u64 over size limit
    let invalid_value_for_serializing = u64::MAX;
    let bad_a: A = A {
        value: invalid_value_for_serializing,
        value_optional_none: None,
        value_optional_some: Some(invalid_value_for_serializing),
        value_vector: vec![invalid_value_for_serializing],
    };
    let result = serialize_to_document(&bad_a);
    assert!(
        result.is_err(),
        "Serialization should fail for u64::MAX since it can't be exactly represented as f64"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("Cannot convert u64"),
        "Expected error message to mention failed u64 to f64 conversion, got: {}",
        err_string
    );
}

#[test]
#[cfg(feature = "serde_with-3")]
fn test_u64_i64_helper() {
    let _guard = LOCK.run_concurrently();
    #[serde_as]
    #[derive(Deserialize, Serialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "u64::AsI64")]
        value: u64,

        #[serde_as(as = "Option<u64::AsI64>")]
        value_optional_none: Option<u64>,

        #[serde_as(as = "Option<u64::AsI64>")]
        value_optional_some: Option<u64>,

        #[serde_as(as = "Vec<u64::AsI64>")]
        value_vector: Vec<u64>,
    }

    let value = i64::MAX as u64;
    let a = A {
        value,
        value_optional_none: None,
        value_optional_some: Some(value),
        value_vector: vec![value],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_eq!(
        doc.get_i64("value").unwrap(),
        value as i64,
        "Expected serialized value to match original."
    );

    assert_eq!(
        doc.get("value_optional_none"),
        Some(&Bson::Null),
        "Expected serialized value_optional_none to be None."
    );

    assert_eq!(
        doc.get("value_optional_some"),
        Some(&Bson::Int64(value as i64)),
        "Expected serialized value_optional_some to match original."
    );

    let value_vector = doc
        .get_array("value_vector")
        .expect("Expected serialized value_vector to be a BSON array.");
    let expected_value_vector: Vec<Bson> = vec![Bson::Int64(value as i64)];
    assert_eq!(
        value_vector, &expected_value_vector,
        "Expected each serialized element in value_vector to match the original."
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Round-trip failed: deserialized struct did not match original."
    );

    // Validate serialization fails because i64::MAX + 1 is too large to fit in i64
    let invalid_value_for_serializing = i64::MAX as u64 + 1;
    let bad_a: A = A {
        value: invalid_value_for_serializing,
        value_optional_none: None,
        value_optional_some: Some(invalid_value_for_serializing),
        value_vector: vec![invalid_value_for_serializing],
    };
    let result = serialize_to_document(&bad_a);
    assert!(
        result.is_err(),
        "Serialization should fail for (i64::MAX as u64) + 1 since it can't be exactly \
         represented as i64"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("Cannot convert u64"),
        "Expected error message to mention failed u64 to i64 conversion, got: {}",
        err_string
    );

    // Validate deserialization fails for i64::MIN because negative values can't be converted to
    // u64
    let invalid_value_for_deserializing = i64::MIN;
    let bad_a = doc! {
        "value": invalid_value_for_deserializing,
        "value_optional_none": Bson::Null,
        "value_optional_some": Some(invalid_value_for_deserializing),
        "value_vector": [invalid_value_for_deserializing],
    };
    let result: Result<A, _> = deserialize_from_document(bad_a);
    assert!(
        result.is_err(),
        "Deserialization should fail for i64::MIN since it can't be exactly represented as u64"
    );
    let err_string = format!("{:?}", result.unwrap_err());
    assert!(
        err_string.contains("Cannot convert i64"),
        "Expected error message to mention failed i64 to u64 conversion, got: {}",
        err_string
    );
}

#[test]
#[cfg(all(feature = "serde_with-3", feature = "uuid-1"))]
fn test_uuid_1_helpers() {
    use crate::uuid::UuidRepresentation;
    use serde_helpers::uuid_1;
    use uuid::Uuid;

    let _guard = LOCK.run_concurrently();

    fn assert_binary_match(
        actual: &Bson,
        uuid: &Uuid,
        expected_subtype: BinarySubtype,
        uuid_representation: UuidRepresentation,
    ) {
        match actual {
            Bson::Binary(Binary { subtype, bytes }) => {
                assert_eq!(
                    subtype, &expected_subtype,
                    "Expected subtype {:?}, but got {:?}",
                    expected_subtype, subtype
                );
                let expected_bytes = {
                    let uuid: crate::Uuid = crate::uuid::Uuid::from(*uuid);
                    crate::Binary::from_uuid_with_representation(uuid, uuid_representation).bytes
                };
                assert_eq!(
                    bytes, &expected_bytes,
                    "Serialized binary bytes did not match for representation {:?}",
                    uuid_representation
                );
            }
            other => panic!("Expected Bson::Binary, got {:?}", other),
        }
    }

    #[serde_as]
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct A {
        #[serde_as(as = "uuid_1::AsBinary")]
        uuid: Uuid,

        #[serde_as(as = "Option<uuid_1::AsBinary>")]
        uuid_optional_none: Option<Uuid>,

        #[serde_as(as = "Option<uuid_1::AsBinary>")]
        uuid_optional_some: Option<Uuid>,

        #[serde_as(as = "Vec<uuid_1::AsBinary>")]
        uuid_vector: Vec<Uuid>,
    }

    let uuid = Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap();
    let a: A = A {
        uuid,
        uuid_optional_none: None,
        uuid_optional_some: Some(uuid),
        uuid_vector: vec![uuid],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&a).unwrap();

    // Validate serialized data
    assert_binary_match(
        doc.get("uuid").unwrap(),
        &uuid,
        BinarySubtype::Uuid,
        UuidRepresentation::Standard,
    );

    assert_eq!(
        doc.get("uuid_optional_none"),
        Some(&Bson::Null),
        "Expected serialized uuid_optional_none to be None."
    );

    assert_binary_match(
        doc.get("uuid_optional_some").unwrap(),
        &uuid,
        BinarySubtype::Uuid,
        UuidRepresentation::Standard,
    );

    let uuid_vector = doc
        .get_array("uuid_vector")
        .expect("Expected serialized uuid_vector to be a BSON array.");
    assert_eq!(uuid_vector.len(), 1);
    assert_binary_match(
        &uuid_vector[0],
        &uuid,
        BinarySubtype::Uuid,
        UuidRepresentation::Standard,
    );

    // Validate deserialized data
    let a_deserialized: A = deserialize_from_document(doc).unwrap();
    assert_eq!(
        a_deserialized, a,
        "Deserialized struct does not match original."
    );

    #[serde_as]
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct B {
        #[serde_as(as = "uuid_1::AsCSharpLegacyBinary")]
        uuid: Uuid,

        #[serde_as(as = "Option<uuid_1::AsCSharpLegacyBinary>")]
        uuid_optional_none: Option<Uuid>,

        #[serde_as(as = "Option<uuid_1::AsCSharpLegacyBinary>")]
        uuid_optional_some: Option<Uuid>,

        #[serde_as(as = "Vec<uuid_1::AsCSharpLegacyBinary>")]
        uuid_vector: Vec<Uuid>,
    }

    let uuid = Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap();
    let b: B = B {
        uuid,
        uuid_optional_none: None,
        uuid_optional_some: Some(uuid),
        uuid_vector: vec![uuid],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&b).unwrap();

    // Validate serialized data
    assert_binary_match(
        doc.get("uuid").unwrap(),
        &uuid,
        BinarySubtype::UuidOld,
        UuidRepresentation::CSharpLegacy,
    );

    assert_eq!(
        doc.get("uuid_optional_none"),
        Some(&Bson::Null),
        "Expected serialized uuid_optional_none to be None."
    );

    assert_binary_match(
        doc.get("uuid_optional_some").unwrap(),
        &uuid,
        BinarySubtype::UuidOld,
        UuidRepresentation::CSharpLegacy,
    );

    let uuid_vector = doc
        .get_array("uuid_vector")
        .expect("Expected uuid_vector to be a BSON array");
    assert_eq!(uuid_vector.len(), 1);
    assert_binary_match(
        &uuid_vector[0],
        &uuid,
        BinarySubtype::UuidOld,
        UuidRepresentation::CSharpLegacy,
    );

    // Validate deserialized data
    let b_deserialized: B = deserialize_from_document(doc).unwrap();
    assert_eq!(
        b_deserialized, b,
        "Deserialized struct does not match original."
    );

    #[serde_as]
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct C {
        #[serde_as(as = "uuid_1::AsJavaLegacyBinary")]
        uuid: Uuid,

        #[serde_as(as = "Option<uuid_1::AsJavaLegacyBinary>")]
        uuid_optional_none: Option<Uuid>,

        #[serde_as(as = "Option<uuid_1::AsJavaLegacyBinary>")]
        uuid_optional_some: Option<Uuid>,

        #[serde_as(as = "Vec<uuid_1::AsJavaLegacyBinary>")]
        uuid_vector: Vec<Uuid>,
    }

    let uuid = Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap();
    let c: C = C {
        uuid,
        uuid_optional_none: None,
        uuid_optional_some: Some(uuid),
        uuid_vector: vec![uuid],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&c).unwrap();

    // Validate serialized data
    assert_binary_match(
        doc.get("uuid").unwrap(),
        &uuid,
        BinarySubtype::UuidOld,
        UuidRepresentation::JavaLegacy,
    );

    assert_eq!(
        doc.get("uuid_optional_none"),
        Some(&Bson::Null),
        "Expected serialized uuid_optional_none to be None."
    );

    assert_binary_match(
        doc.get("uuid_optional_some").unwrap(),
        &uuid,
        BinarySubtype::UuidOld,
        UuidRepresentation::JavaLegacy,
    );

    let uuid_vector = doc
        .get_array("uuid_vector")
        .expect("Expected uuid_vector to be a BSON array");
    assert_eq!(uuid_vector.len(), 1);
    assert_binary_match(
        &uuid_vector[0],
        &uuid,
        BinarySubtype::UuidOld,
        UuidRepresentation::JavaLegacy,
    );

    // Validate deserialized data
    let c_deserialized: C = deserialize_from_document(doc).unwrap();
    assert_eq!(
        c_deserialized, c,
        "Deserialized struct does not match original."
    );

    #[serde_as]
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct D {
        #[serde_as(as = "uuid_1::AsPythonLegacyBinary")]
        uuid: Uuid,

        #[serde_as(as = "Option<uuid_1::AsPythonLegacyBinary>")]
        uuid_optional_none: Option<Uuid>,

        #[serde_as(as = "Option<uuid_1::AsPythonLegacyBinary>")]
        uuid_optional_some: Option<Uuid>,

        #[serde_as(as = "Vec<uuid_1::AsPythonLegacyBinary>")]
        uuid_vector: Vec<Uuid>,
    }

    let uuid = Uuid::parse_str("936DA01F9ABD4d9d80C702AF85C822A8").unwrap();
    let d: D = D {
        uuid,
        uuid_optional_none: None,
        uuid_optional_some: Some(uuid),
        uuid_vector: vec![uuid],
    };

    // Serialize the struct to BSON
    let doc = serialize_to_document(&d).unwrap();

    // Validate serialized data
    assert_binary_match(
        doc.get("uuid").unwrap(),
        &uuid,
        BinarySubtype::UuidOld,
        UuidRepresentation::PythonLegacy,
    );

    assert_eq!(
        doc.get("uuid_optional_none"),
        Some(&Bson::Null),
        "Expected serialized uuid_optional_none to be None."
    );

    assert_binary_match(
        doc.get("uuid_optional_some").unwrap(),
        &uuid,
        BinarySubtype::UuidOld,
        UuidRepresentation::PythonLegacy,
    );

    let uuid_vector = doc
        .get_array("uuid_vector")
        .expect("Expected uuid_vector to be a BSON array");
    assert_eq!(uuid_vector.len(), 1);
    assert_binary_match(
        &uuid_vector[0],
        &uuid,
        BinarySubtype::UuidOld,
        UuidRepresentation::PythonLegacy,
    );

    // Validate deserialized data
    let d_deserialized: D = deserialize_from_document(doc).unwrap();
    assert_eq!(
        d_deserialized, d,
        "Deserialized struct does not match original."
    );
}
