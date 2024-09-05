use core::str;

use serde::{de::Visitor, Deserialize, Serialize};

use crate::{
    from_slice,
    serde_helpers::{HumanReadable, Utf8LossyDeserialization},
    Document,
};

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
            impl<'de> Visitor<'de> for V {
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
    let bson = crate::to_bson_with_options(
        &data,
        #[allow(deprecated)]
        crate::SerializerOptions::builder()
            .human_readable(false)
            .build(),
    )
    .unwrap();
    assert_eq!(
        bson.as_document().unwrap(),
        &doc! {
            "first": "human readable",
            "outer": "not human readable",
            "wrapped": "human readable",
            "inner": {
                "value": "human readable",
            }
        }
    );

    let tripped: Data = crate::from_bson_with_options(
        bson,
        #[allow(deprecated)]
        crate::DeserializerOptions::builder()
            .human_readable(false)
            .build(),
    )
    .unwrap();
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

    let bytes = crate::to_vec(&data).unwrap();
    let raw_tripped: Data = crate::from_slice(&bytes).unwrap();
    assert_eq!(&raw_tripped, &expected);
}

#[test]
#[allow(dead_code)] // suppress warning for unread fields
fn utf8_lossy_wrapper() {
    // See https://doc.rust-lang.org/std/str/fn.from_utf8.html for details on how the invalid utf8
    // strings are constructed.

    let good_bytes = rawdoc! { "s1": "💖", "s2": "💖" }.into_bytes();

    let heart_index = "💖".bytes().next().unwrap();
    let first_heart = good_bytes.iter().position(|b| *b == heart_index).unwrap();
    let second_heart = good_bytes
        .iter()
        .skip(first_heart + 1)
        .position(|b| *b == heart_index)
        .unwrap()
        + first_heart
        + 1;

    let mut both_strings_invalid_bytes = good_bytes.clone();
    both_strings_invalid_bytes[first_heart] = 0;
    both_strings_invalid_bytes[second_heart] = 0;

    #[derive(Debug, Deserialize)]
    struct NoUtf8Lossy {
        s1: String,
        s2: String,
    }

    from_slice::<NoUtf8Lossy>(&both_strings_invalid_bytes).unwrap_err();
    from_slice::<Utf8LossyDeserialization<NoUtf8Lossy>>(&both_strings_invalid_bytes).unwrap();

    #[derive(Debug, Deserialize)]
    struct FirstStringUtf8Lossy {
        s1: Utf8LossyDeserialization<String>,
        s2: String,
    }

    let mut first_string_invalid_bytes = good_bytes.clone();
    first_string_invalid_bytes[first_heart] = 0;

    from_slice::<FirstStringUtf8Lossy>(&first_string_invalid_bytes).unwrap();
    from_slice::<FirstStringUtf8Lossy>(&both_strings_invalid_bytes).unwrap_err();
    from_slice::<Utf8LossyDeserialization<FirstStringUtf8Lossy>>(&both_strings_invalid_bytes)
        .unwrap();

    from_slice::<Utf8LossyDeserialization<Document>>(&both_strings_invalid_bytes).unwrap();
}
