use crate::{
    doc,
    oid::ObjectId,
    spec::BinarySubtype,
    tests::LOCK,
    Binary,
    Bson,
    RawBson,
    Regex,
    Timestamp,
};
use pretty_assertions::assert_eq;

#[test]
fn standard_format() {
    let _guard = LOCK.run_concurrently();
    let id_string = "thisismyname";
    let string_bytes: Vec<_> = id_string.bytes().collect();
    let mut bytes = [0; 12];
    bytes[..12].clone_from_slice(&string_bytes[..12]);

    let id = ObjectId::from_bytes(bytes);
    let date = time::OffsetDateTime::now_utc();

    let doc = doc! {
        "float": 2.4,
        "string": "hello",
        "array": ["testing", 1, true, [1, 2]],
        "doc": {
            "fish": "in",
            "a": "barrel",
            "!": 1,
        },
        "bool": true,
        "null": null,
        "regexp": Bson::RegularExpression(Regex { pattern: "s[ao]d".to_owned(), options: "i".to_owned() }),
        "with_wrapped_parens": (-20),
        "code": Bson::JavaScriptCode("function(x) { return x._id; }".to_owned()),
        "i32": 12,
        "i64": -55,
        "timestamp": Bson::Timestamp(Timestamp { time: 0, increment: 229_999_444 }),
        "binary": Binary { subtype: BinarySubtype::Md5, bytes: "thingies".to_owned().into_bytes() },
        "encrypted": Binary { subtype: BinarySubtype::Encrypted, bytes: "secret".to_owned().into_bytes() },
        "_id": id,
        "date": Bson::DateTime(crate::DateTime::from_time_0_3(date)),
    };

    let rawdoc = rawdoc! {
        "float": 2.4,
        "string": "hello",
        "array": ["testing", 1, true, [1, 2]],
        "doc": {
            "fish": "in",
            "a": "barrel",
            "!": 1,
        },
        "bool": true,
        "null": null,
        "regexp": Regex { pattern: "s[ao]d".to_owned(), options: "i".to_owned() },
        "with_wrapped_parens": (-20),
        "code": RawBson::JavaScriptCode("function(x) { return x._id; }".to_owned()),
        "i32": 12,
        "i64": -55,
        "timestamp": Timestamp { time: 0, increment: 229_999_444 },
        "binary": Binary { subtype: BinarySubtype::Md5, bytes: "thingies".to_owned().into_bytes() },
        "encrypted": Binary { subtype: BinarySubtype::Encrypted, bytes: "secret".to_owned().into_bytes() },
        "_id": id,
        "date": crate::DateTime::from_time_0_3(date),
    };

    let ts_nanos = date.unix_timestamp_nanos();
    let ts_millis = ts_nanos - (ts_nanos % 1_000_000);
    let date_trunc = time::OffsetDateTime::from_unix_timestamp_nanos(ts_millis).unwrap();
    let expected = format!(
        "{{ \"float\": 2.4, \"string\": \"hello\", \"array\": [\"testing\", 1, true, [1, 2]], \
         \"doc\": {{ \"fish\": \"in\", \"a\": \"barrel\", \"!\": 1 }}, \"bool\": true, \"null\": \
         null, \"regexp\": /s[ao]d/i, \"with_wrapped_parens\": -20, \"code\": function(x) {{ \
         return x._id; }}, \"i32\": 12, \"i64\": -55, \"timestamp\": Timestamp(0, 229999444), \
         \"binary\": Binary(0x5, {}), \"encrypted\": Binary(0x6, {}), \"_id\": ObjectId(\"{}\"), \
         \"date\": DateTime(\"{}\") }}",
        base64::encode("thingies"),
        base64::encode("secret"),
        hex::encode(id_string),
        date_trunc,
    );

    assert_eq!(expected, format!("{}", doc));

    assert_eq!(rawdoc.into_bytes(), crate::to_vec(&doc).unwrap());
}

#[test]
fn non_trailing_comma() {
    let _guard = LOCK.run_concurrently();
    let doc = doc! {
        "a": "foo",
        "b": { "ok": "then" }
    };

    let expected = "{ \"a\": \"foo\", \"b\": { \"ok\": \"then\" } }".to_string();
    assert_eq!(expected, format!("{}", doc));
}

#[test]
#[allow(clippy::float_cmp)]
fn recursive_macro() {
    let _guard = LOCK.run_concurrently();
    let doc = doc! {
        "a": "foo",
        "b": {
            "bar": {
                "harbor": ["seal", false],
                "jelly": 42.0,
            },
            "grape": 27,
        },
        "c": [-7],
        "d": [
            {
                "apple": "ripe",
            }
        ],
        "e": { "single": "test" },
        "n": (Bson::Null),
    };
    let rawdoc = rawdoc! {
        "a": "foo",
        "b": {
            "bar": {
                "harbor": ["seal", false],
                "jelly": 42.0,
            },
            "grape": 27,
        },
        "c": [-7],
        "d": [
            {
                "apple": "ripe",
            }
        ],
        "e": { "single": "test" },
        "n": (RawBson::Null),
    };

    match doc.get("a") {
        Some(Bson::String(s)) => assert_eq!("foo", s),
        _ => panic!("String 'foo' was not inserted correctly."),
    }

    // Inner Doc 1
    match doc.get("b") {
        Some(Bson::Document(doc)) => {
            // Inner doc 2
            match doc.get("bar") {
                Some(Bson::Document(inner_doc)) => {
                    // Inner array
                    match inner_doc.get("harbor") {
                        Some(Bson::Array(arr)) => {
                            assert_eq!(2, arr.len());

                            // Match array items
                            match arr.get(0) {
                                Some(Bson::String(ref s)) => assert_eq!("seal", s),
                                _ => panic!(
                                    "String 'seal' was not inserted into inner array correctly."
                                ),
                            }
                            match arr.get(1) {
                                Some(Bson::Boolean(ref b)) => assert!(!b),
                                _ => panic!(
                                    "Bool 'false' was not inserted into inner array correctly."
                                ),
                            }
                        }
                        _ => panic!("Inner array was not inserted correctly."),
                    }

                    // Inner floating point
                    match inner_doc.get("jelly") {
                        Some(Bson::Double(fp)) => assert_eq!(42.0, *fp),
                        _ => panic!("Floating point 42.0 was not inserted correctly."),
                    }
                }
                _ => panic!("Second inner document was not inserted correctly."),
            }
        }
        _ => panic!("Inner document was not inserted correctly."),
    }

    // Single-item array
    match doc.get("c") {
        Some(Bson::Array(arr)) => {
            assert_eq!(1, arr.len());

            // Integer type
            match arr.get(0) {
                Some(Bson::Int32(ref i)) => assert_eq!(-7, *i),
                _ => panic!("I32 '-7' was not inserted correctly."),
            }
        }
        _ => panic!("Single-item array was not inserted correctly."),
    }

    // Document nested in array
    match doc.get("d") {
        Some(Bson::Array(arr)) => {
            assert_eq!(1, arr.len());

            // Nested document
            match arr.get(0) {
                Some(Bson::Document(ref doc)) => {
                    // String
                    match doc.get("apple") {
                        Some(Bson::String(s)) => assert_eq!("ripe", s),
                        _ => panic!("String 'ripe' was not inserted correctly."),
                    }
                }
                _ => panic!("Document was not inserted into array correctly."),
            }
        }
        _ => panic!("Array was not inserted correctly."),
    }

    // Single-item document
    match doc.get("e") {
        Some(Bson::Document(bdoc)) => {
            // String
            match bdoc.get("single") {
                Some(Bson::String(s)) => assert_eq!("test", s),
                _ => panic!("String 'test' was not inserted correctly."),
            }
        }
        _ => panic!("Single-item document was not inserted correctly."),
    }

    match doc.get("n") {
        Some(&Bson::Null) => {
            // It was null
        }
        _ => panic!("Null was not inserted correctly."),
    }

    assert_eq!(rawdoc.into_bytes(), crate::to_vec(&doc).unwrap());
}

#[test]
#[allow(clippy::from_over_into)]
fn can_use_macro_with_into_bson() {
    struct Custom;

    impl Into<Bson> for Custom {
        fn into(self) -> Bson {
            "foo".into()
        }
    }

    impl Into<RawBson> for Custom {
        fn into(self) -> RawBson {
            "foo".into()
        }
    }

    _ = bson!({
        "a": Custom,
    });
    _ = doc! {
        "a": Custom,
    };
    _ = rawbson!({
        "a": Custom,
    });
    _ = rawdoc! {
        "a": Custom,
    };
}
