use serde::{de::Visitor, Deserialize, Serialize};

use crate::serde_helpers::HumanReadable;

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
