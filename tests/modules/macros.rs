use bson::{doc, oid::ObjectId, spec::BinarySubtype, Binary, Bson, Regex, TimeStamp};
use chrono::offset::Utc;
use pretty_assertions::assert_eq;

#[test]
fn standard_format() {
    let id_string = "thisismyname";
    let string_bytes: Vec<_> = id_string.bytes().collect();
    let mut bytes = [0; 12];
    bytes[..12].clone_from_slice(&string_bytes[..12]);

    let id = ObjectId::with_bytes(bytes);
    let date = Utc::now();

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
        "regexp": Bson::Regex(Regex { pattern: "s[ao]d".to_owned(), options: "i".to_owned() }),
        "with_wrapped_parens": (-20),
        "code": Bson::JavaScriptCode("function(x) { return x._id; }".to_owned()),
        "i32": 12,
        "i64": -55,
        "timestamp": Bson::TimeStamp(TimeStamp { time: 0, increment: 229_999_444 }),
        "binary": Binary { subtype: BinarySubtype::Md5, bytes: "thingies".to_owned().into_bytes() },
        "encrypted": Binary { subtype: BinarySubtype::Encrypted, bytes: "secret".to_owned().into_bytes() },
        "_id": id,
        "date": Bson::UtcDatetime(date),
    };

    let expected = format!(
        "{{ float: 2.4, string: \"hello\", array: [\"testing\", 1, true, [1, 2]], doc: {{ fish: \
         \"in\", a: \"barrel\", !: 1 }}, bool: true, null: null, regexp: /s[ao]d/i, \
         with_wrapped_parens: -20, code: function(x) {{ return x._id; }}, i32: 12, i64: -55, \
         timestamp: Timestamp(0, 229999444), binary: BinData(5, 0x{}), encrypted: BinData(6, \
         0x{}), _id: ObjectId(\"{}\"), date: Date(\"{}\") }}",
        base64::encode("thingies"),
        base64::encode("secret"),
        hex::encode(id_string),
        date
    );

    assert_eq!(expected, format!("{}", doc));
}

#[test]
fn non_trailing_comma() {
    let doc = doc! {
        "a": "foo",
        "b": { "ok": "then" }
    };

    let expected = "{ a: \"foo\", b: { ok: \"then\" } }".to_string();
    assert_eq!(expected, format!("{}", doc));
}

#[test]
#[allow(clippy::float_cmp)]
fn recursive_macro() {
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

    match doc.get("a") {
        Some(&Bson::String(ref s)) => assert_eq!("foo", s),
        _ => panic!("String 'foo' was not inserted correctly."),
    }

    // Inner Doc 1
    match doc.get("b") {
        Some(&Bson::Document(ref doc)) => {
            // Inner doc 2
            match doc.get("bar") {
                Some(&Bson::Document(ref inner_doc)) => {
                    // Inner array
                    match inner_doc.get("harbor") {
                        Some(&Bson::Array(ref arr)) => {
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
                                    "Boolean 'false' was not inserted into inner array correctly."
                                ),
                            }
                        }
                        _ => panic!("Inner array was not inserted correctly."),
                    }

                    // Inner floating point
                    match inner_doc.get("jelly") {
                        Some(&Bson::FloatingPoint(ref fp)) => assert_eq!(42.0, *fp),
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
        Some(&Bson::Array(ref arr)) => {
            assert_eq!(1, arr.len());

            // Integer type
            match arr.get(0) {
                Some(Bson::I32(ref i)) => assert_eq!(-7, *i),
                _ => panic!("I32 '-7' was not inserted correctly."),
            }
        }
        _ => panic!("Single-item array was not inserted correctly."),
    }

    // Document nested in array
    match doc.get("d") {
        Some(&Bson::Array(ref arr)) => {
            assert_eq!(1, arr.len());

            // Nested document
            match arr.get(0) {
                Some(Bson::Document(ref doc)) => {
                    // String
                    match doc.get("apple") {
                        Some(&Bson::String(ref s)) => assert_eq!("ripe", s),
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
        Some(&Bson::Document(ref bdoc)) => {
            // String
            match bdoc.get("single") {
                Some(&Bson::String(ref s)) => assert_eq!("test", s),
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
}
