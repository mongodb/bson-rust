#![allow(clippy::cognitive_complexity)]
#![allow(clippy::vec_init_then_push)]

mod options;

use pretty_assertions::assert_eq;
use serde::{
    self,
    de::{DeserializeOwned, Unexpected},
    Deserialize,
    Serialize,
};

use std::{
    borrow::Cow,
    collections::{BTreeMap, HashSet},
};

use bson::{
    bson,
    doc,
    oid::ObjectId,
    spec::BinarySubtype,
    Binary,
    Bson,
    DateTime,
    Decimal128,
    Deserializer,
    DeserializerOptions,
    Document,
    JavaScriptCodeWithScope,
    RawArray,
    RawBinary,
    RawBson,
    RawDbPointer,
    RawDocument,
    RawDocumentBuf,
    RawJavaScriptCodeWithScope,
    RawRegex,
    Regex,
    SerializerOptions,
    Timestamp,
    Uuid,
};

/// Verifies the following:
///   - round trip `expected_value` through `Document`:
///     - serializing the `expected_value` to a `Document` matches the `expected_doc`
///     - deserializing from the serialized document produces `expected_value`
///   - round trip through raw BSON:
///     - deserializing a `T` from the raw BSON version of `expected_doc` produces `expected_value`
///     - deserializing a `Document` from the raw BSON version of `expected_doc` produces
///       `expected_doc`
///   - `bson::to_writer` and `Document::to_writer` produce the same result given the same input
fn run_test<T>(expected_value: &T, expected_doc: &Document, description: &str)
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let mut expected_bytes = Vec::new();
    expected_doc
        .to_writer(&mut expected_bytes)
        .expect(description);

    let expected_bytes_serde = bson::to_vec(&expected_value).expect(description);

    assert_eq!(expected_bytes_serde, expected_bytes, "{}", description);

    let expected_bytes_from_doc_serde = bson::to_vec(&expected_doc).expect(description);
    assert_eq!(
        expected_bytes_from_doc_serde, expected_bytes,
        "{}",
        description
    );

    let serialized_doc = bson::to_document(&expected_value).expect(description);
    assert_eq!(&serialized_doc, expected_doc, "{}", description);
    assert_eq!(
        expected_value,
        &bson::from_document::<T>(serialized_doc).expect(description),
        "{}",
        description
    );

    let non_human_readable_doc = bson::to_document_with_options(
        &expected_value,
        SerializerOptions::builder().human_readable(false).build(),
    )
    .expect(description);
    assert_eq!(&non_human_readable_doc, expected_doc, "{}", description);
    assert_eq!(
        expected_value,
        &bson::from_document_with_options::<T>(
            non_human_readable_doc,
            DeserializerOptions::builder().human_readable(false).build()
        )
        .expect(description),
        "{}",
        description
    );

    assert_eq!(
        &bson::from_reader::<_, T>(expected_bytes.as_slice()).expect(description),
        expected_value,
        "{}",
        description
    );
    assert_eq!(
        &bson::from_reader::<_, Document>(expected_bytes.as_slice()).expect(description),
        expected_doc,
        "{}",
        description
    );
}

/// Verifies the following:
/// - deserializing a `T` from `expected_doc` produces `expected_value`
/// - deserializing a `T` from the raw BSON version of `expected_doc` produces `expected_value`
/// - deserializing a `Document` from the raw BSON version of `expected_doc` produces `expected_doc`
fn run_deserialize_test<T>(expected_value: &T, expected_doc: &Document, description: &str)
where
    T: DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let mut expected_bytes = Vec::new();
    expected_doc
        .to_writer(&mut expected_bytes)
        .expect(description);

    assert_eq!(
        &bson::from_document::<T>(expected_doc.clone()).expect(description),
        expected_value,
        "{}",
        description
    );
    assert_eq!(
        &bson::from_reader::<_, T>(expected_bytes.as_slice()).expect(description),
        expected_value,
        "{}",
        description
    );
    assert_eq!(
        &bson::from_reader::<_, Document>(expected_bytes.as_slice()).expect(description),
        expected_doc,
        "{}",
        description
    );
}

/// Verifies the following:
/// - Deserializing a `T` from the provided bytes does not error
/// - Serializing the `T` back to bytes produces the input.
fn run_raw_round_trip_test<'de, T>(bytes: &'de [u8], description: &str)
where
    T: Deserialize<'de> + Serialize + std::fmt::Debug,
{
    let t: T = bson::from_slice(bytes).expect(description);
    let vec = bson::to_vec(&t).expect(description);
    assert_eq!(vec.as_slice(), bytes);
}

#[test]
fn smoke() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: isize,
    }

    let v = Foo { a: 2 };
    let expected = doc! { "a": 2_i64 };

    run_test(&v, &expected, "smoke");
}

#[test]
fn smoke_under() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a_b: isize,
    }

    let v = Foo { a_b: 2 };
    let doc = doc! { "a_b": 2_i64 };
    run_test(&v, &doc, "smoke under");

    let mut m = BTreeMap::new();
    m.insert("a_b".to_string(), 2_i64);
    run_test(&m, &doc, "smoke under BTreeMap");
}

#[test]
fn nested() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: isize,
        b: Bar,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar {
        a: String,
    }

    let v = Foo {
        a: 2,
        b: Bar {
            a: "test".to_string(),
        },
    };
    let doc = doc! {
        "a": 2_i64,
        "b": {
            "a": "test"
        }
    };
    run_test(&v, &doc, "nested");
}

#[test]
fn application_deserialize_error() {
    #[derive(PartialEq, Debug)]
    struct Range10(usize);
    impl<'de> Deserialize<'de> for Range10 {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Range10, D::Error> {
            let x: usize = Deserialize::deserialize(d)?;
            if x > 10 {
                Err(serde::de::Error::invalid_value(
                    Unexpected::Unsigned(x as u64),
                    &"more than 10",
                ))
            } else {
                Ok(Range10(x))
            }
        }
    }
    let d_good = Deserializer::new(Bson::Int64(5));
    let d_bad1 = Deserializer::new(Bson::String("not an isize".to_string()));
    let d_bad2 = Deserializer::new(Bson::Int64(11));

    assert_eq!(
        Range10(5),
        Deserialize::deserialize(d_good).expect("deserialization should succeed")
    );

    Range10::deserialize(d_bad1).expect_err("deserialization from string should fail");
    Range10::deserialize(d_bad2).expect_err("deserialization from 11 should fail");
}

#[test]
fn array() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Vec<i32>,
    }

    let v = Foo {
        a: vec![1, 2, 3, 4],
    };
    let doc = doc! {
        "a": [1, 2, 3, 4],
    };
    run_test(&v, &doc, "array");
}

#[test]
fn tuple() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: (i32, i32, i32, i32),
    }

    let v = Foo { a: (1, 2, 3, 4) };
    let doc = doc! {
        "a": [1, 2, 3, 4],
    };
    run_test(&v, &doc, "tuple");
}

#[test]
fn inner_structs_with_options() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Option<Box<Foo>>,
        b: Bar,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar {
        a: String,
        b: f64,
    }

    let v = Foo {
        a: Some(Box::new(Foo {
            a: None,
            b: Bar {
                a: "foo".to_string(),
                b: 4.5,
            },
        })),
        b: Bar {
            a: "bar".to_string(),
            b: 1.0,
        },
    };
    let doc = doc! {
        "a": {
            "a": Bson::Null,
            "b": {
                "a": "foo",
                "b": 4.5,
            }
        },
        "b": {
            "a": "bar",
            "b": 1.0,
        }
    };
    run_test(&v, &doc, "inner_structs_with_options");
}

#[test]
fn inner_structs_with_skippable_options() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        #[serde(skip_serializing_if = "Option::is_none")]
        a: Option<Box<Foo>>,
        b: Bar,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar {
        a: String,
        b: f64,
    }

    let v = Foo {
        a: Some(Box::new(Foo {
            a: None,
            b: Bar {
                a: "foo".to_string(),
                b: 4.5,
            },
        })),
        b: Bar {
            a: "bar".to_string(),
            b: 1.0,
        },
    };
    let doc = doc! {
        "a" : {
            "b": {
                "a": "foo",
                "b": 4.5
            }
        },
        "b": {
            "a": "bar",
            "b": 1.0
        }
    };
    run_test(&v, &doc, "inner_structs_with_skippable_options");
}

#[test]
fn hashmap() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        map: BTreeMap<String, i32>,
        set: HashSet<char>,
    }

    let v = Foo {
        map: {
            let mut m = BTreeMap::new();
            m.insert("bar".to_string(), 4);
            m.insert("foo".to_string(), 10);
            m
        },
        set: {
            let mut s = HashSet::new();
            s.insert('a');
            s
        },
    };
    let doc = doc! {
        "map": {
            "bar": 4,
            "foo": 10
        },
        "set": ["a"]
    };
    run_test(&v, &doc, "hashmap");
}

#[test]
fn tuple_struct() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo(i32, String, f64);
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar {
        whee: Foo,
    }

    let v = Bar {
        whee: Foo(1, "foo".to_string(), 4.5),
    };
    let doc = doc! {
        "whee": [1, "foo", 4.5],
    };
    run_test(&v, &doc, "tuple_struct");
}

#[test]
fn table_array() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Vec<Bar>,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar {
        a: i32,
    }

    let v = Foo {
        a: vec![Bar { a: 1 }, Bar { a: 2 }],
    };
    let doc = doc! {
        "a": [{ "a": 1 }, { "a": 2 }]
    };
    run_test(&v, &doc, "table_array");
}

#[test]
fn type_conversion() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        bar: i32,
    }

    let v = Foo { bar: 1 };
    let doc = doc! {
        "bar": 1_i64
    };
    let deserialized: Foo = bson::from_document(doc.clone()).unwrap();
    assert_eq!(deserialized, v);

    let mut bytes = Vec::new();
    doc.to_writer(&mut bytes).unwrap();

    let bson_deserialized: Foo = bson::from_reader(bytes.as_slice()).unwrap();
    assert_eq!(bson_deserialized, v);
}

#[test]
fn missing_errors() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        bar: i32,
    }

    let doc = doc! {};

    bson::from_document::<Foo>(doc.clone()).unwrap_err();

    let mut bytes = Vec::new();
    doc.to_writer(&mut bytes).unwrap();

    bson::from_reader::<_, Foo>(bytes.as_slice()).unwrap_err();
}

#[test]
fn parse_enum() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: E,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    enum E {
        Empty,
        Bar(i32),
        Baz(f64),
        Pair(i32, i32),
        Last(Foo2),
        Vector(Vec<i32>),
        Named { a: i32 },
        MultiNamed { a: i32, b: i32 },
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo2 {
        test: String,
    }

    let v = Foo { a: E::Empty };
    let doc = doc! { "a": "Empty" };
    run_test(&v, &doc, "parse_enum: Empty");

    let v = Foo { a: E::Bar(10) };
    let doc = doc! { "a": { "Bar": 10 } };
    run_test(&v, &doc, "parse_enum: newtype variant int");

    let v = Foo { a: E::Baz(10.2) };
    let doc = doc! { "a": { "Baz": 10.2 } };
    run_test(&v, &doc, "parse_enum: newtype variant double");

    let v = Foo { a: E::Pair(12, 42) };
    let doc = doc! { "a": { "Pair": [12, 42] } };
    run_test(&v, &doc, "parse_enum: tuple variant");

    let v = Foo {
        a: E::Last(Foo2 {
            test: "test".to_string(),
        }),
    };
    let doc = doc! {
        "a": { "Last": { "test": "test" } }
    };
    run_test(&v, &doc, "parse_enum: newtype variant struct");

    let v = Foo {
        a: E::Vector(vec![12, 42]),
    };
    let doc = doc! {
        "a": { "Vector": [12, 42] }
    };
    run_test(&v, &doc, "parse_enum: newtype variant vector");

    let v = Foo {
        a: E::Named { a: 12 },
    };
    let doc = doc! {
        "a": { "Named": { "a": 12 } }
    };
    run_test(&v, &doc, "parse_enum: struct variant");

    let v = Foo {
        a: E::MultiNamed { a: 12, b: 42 },
    };
    let doc = doc! {
        "a": { "MultiNamed": { "a": 12, "b": 42 } }
    };
    run_test(&v, &doc, "parse_enum: struct variant multiple fields");
}

#[test]
fn unused_fields() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: i32,
    }

    let v = Foo { a: 2 };
    let doc = doc! {
        "a": 2,
        "b": 5,
    };

    run_deserialize_test(&v, &doc, "unused_fields");
}

#[test]
fn unused_fields2() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Bar,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar {
        a: i32,
    }

    let v = Foo { a: Bar { a: 2 } };
    let doc = doc! {
        "a": {
            "a": 2,
            "b": 5
        }
    };

    run_deserialize_test(&v, &doc, "unused_fields2");
}

#[test]
fn unused_fields3() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Bar,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar {
        a: i32,
    }

    let v = Foo { a: Bar { a: 2 } };
    let doc = doc! {
        "a": {
            "a": 2
        }
    };
    run_deserialize_test(&v, &doc, "unused_fields3");
}

#[test]
fn unused_fields4() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: BTreeMap<String, String>,
    }

    let mut map = BTreeMap::new();
    map.insert("a".to_owned(), "foo".to_owned());
    let v = Foo { a: map };
    let doc = doc! {
        "a": {
            "a": "foo"
        }
    };
    run_deserialize_test(&v, &doc, "unused_fields4");
}

#[test]
fn unused_fields5() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Vec<String>,
    }

    let v = Foo {
        a: vec!["a".to_string()],
    };
    let doc = doc! {
        "a": ["a"]
    };
    run_deserialize_test(&v, &doc, "unusued_fields5");
}

#[test]
fn unused_fields6() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Option<Vec<String>>,
    }

    let v = Foo { a: Some(vec![]) };
    let doc = doc! {
        "a": []
    };
    run_deserialize_test(&v, &doc, "unused_fieds6");
}

#[test]
fn unused_fields7() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Vec<Bar>,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar {
        a: i32,
    }

    let v = Foo {
        a: vec![Bar { a: 1 }],
    };
    let doc = doc! {
        "a": [{"a": 1, "b": 2}]
    };
    run_deserialize_test(&v, &doc, "unused_fields7");
}

#[test]
fn unused_fields_deny() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    #[serde(deny_unknown_fields)]
    struct Foo {
        a: i32,
    }

    let doc = doc! {
        "a": 1,
        "b": 2,
    };
    bson::from_document::<Foo>(doc.clone()).expect_err("extra fields should cause failure");

    let mut bytes = Vec::new();
    doc.to_writer(&mut bytes).unwrap();
    bson::from_reader::<_, Foo>(bytes.as_slice()).expect_err("extra fields should cause failure");
}

#[test]
fn default_array() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        #[serde(default)]
        a: Vec<Bar>,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar;

    let v = Foo { a: vec![] };
    let doc = doc! {};
    run_deserialize_test(&v, &doc, "default_array");
}

#[test]
fn null_array() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Option<Vec<Bar>>,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar;

    let v = Foo { a: None };
    let doc = doc! {};
    run_deserialize_test(&v, &doc, "null_array");
}

#[test]
fn empty_array() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Option<Vec<Bar>>,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar;

    let v = Foo { a: Some(vec![]) };
    let doc = doc! {
        "a": []
    };
    run_deserialize_test(&v, &doc, "empty_array");
}

#[test]
fn raw_doc_buf() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        d: RawDocumentBuf,
    }

    let bytes = bson::to_vec(&doc! {
        "d": {
            "a": 12,
            "b": 5.5,
            "c": [1, true, "ok"],
            "d": { "a": "b" },
            "e": ObjectId::new(),
        }
    })
    .expect("raw_doc_buf");

    run_raw_round_trip_test::<Foo>(bytes.as_slice(), "raw_doc_buf");
}

#[test]
fn raw_doc() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo<'a> {
        #[serde(borrow)]
        d: &'a RawDocument,
    }

    let bytes = bson::to_vec(&doc! {
        "d": {
            "a": 12,
            "b": 5.5,
            "c": [1, true, "ok"],
            "d": { "a": "b" },
            "e": ObjectId::new(),
        }
    })
    .expect("raw doc");

    run_raw_round_trip_test::<Foo>(bytes.as_slice(), "raw_doc");
}

#[test]
fn raw_array() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo<'a> {
        #[serde(borrow)]
        d: &'a RawArray,
    }

    let bytes = bson::to_vec(&doc! {
        "d": [1, true, { "ok": 1 }, [ "sub", "array" ], Uuid::new()]
    })
    .expect("raw_array");

    run_raw_round_trip_test::<Foo>(bytes.as_slice(), "raw_array");
}

#[test]
fn raw_binary() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo<'a> {
        #[serde(borrow)]
        generic: RawBinary<'a>,

        #[serde(borrow)]
        old: RawBinary<'a>,

        #[serde(borrow)]
        uuid: RawBinary<'a>,

        #[serde(borrow)]
        other: RawBinary<'a>,
    }

    let bytes = bson::to_vec(&doc! {
        "generic": Binary {
            bytes: vec![1, 2, 3, 4, 5],
            subtype: BinarySubtype::Generic,
        },
        "old": Binary {
            bytes: vec![1, 2, 3],
            subtype: BinarySubtype::BinaryOld,
        },
        "uuid": Uuid::new(),
        "other": Binary {
            bytes: vec![1u8; 100],
            subtype: BinarySubtype::UserDefined(100),
        }
    })
    .expect("raw_binary");

    run_raw_round_trip_test::<Foo>(bytes.as_slice(), "raw_binary");
}

#[test]
fn raw_regex() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo<'a> {
        #[serde(borrow)]
        r: RawRegex<'a>,
    }

    let bytes = bson::to_vec(&doc! {
        "r": Regex {
            pattern: "a[b-c]d".to_string(),
            options: "ab".to_string(),
        },
    })
    .expect("raw_regex");

    run_raw_round_trip_test::<Foo>(bytes.as_slice(), "raw_regex");
}

#[test]
fn raw_code_w_scope() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo<'a> {
        #[serde(borrow)]
        r: RawJavaScriptCodeWithScope<'a>,
    }

    let bytes = bson::to_vec(&doc! {
        "r": JavaScriptCodeWithScope {
            code: "console.log(x)".to_string(),
            scope: doc! { "x": 1 },
        },
    })
    .expect("raw_code_w_scope");

    run_raw_round_trip_test::<Foo>(bytes.as_slice(), "raw_code_w_scope");
}

#[test]
fn raw_db_pointer() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo<'a> {
        #[serde(borrow)]
        a: RawDbPointer<'a>,
    }

    // From the "DBpointer" bson corpus test
    let bytes = hex::decode("1A0000000C610002000000620056E1FC72E0C917E9C471416100").unwrap();

    run_raw_round_trip_test::<Foo>(bytes.as_slice(), "raw_db_pointer");
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct SubDoc {
    a: i32,
    b: i32,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct AllTypes {
    x: i32,
    y: i64,
    s: String,
    array: Vec<Bson>,
    bson: Bson,
    oid: ObjectId,
    null: Option<()>,
    subdoc: Document,
    b: bool,
    d: f64,
    binary: Binary,
    binary_old: Binary,
    binary_other: Binary,
    date: DateTime,
    regex: Regex,
    ts: Timestamp,
    i: SubDoc,
    undefined: Bson,
    code: Bson,
    code_w_scope: JavaScriptCodeWithScope,
    decimal: Decimal128,
    symbol: Bson,
    min_key: Bson,
    max_key: Bson,
}

impl AllTypes {
    fn fixtures() -> (Self, Document) {
        let binary = Binary {
            bytes: vec![36, 36, 36],
            subtype: BinarySubtype::Generic,
        };
        let binary_old = Binary {
            bytes: vec![36, 36, 36],
            subtype: BinarySubtype::BinaryOld,
        };
        let binary_other = Binary {
            bytes: vec![36, 36, 36],
            subtype: BinarySubtype::UserDefined(0x81),
        };
        let date = DateTime::now();
        let regex = Regex {
            pattern: "hello".to_string(),
            options: "x".to_string(),
        };
        let timestamp = Timestamp {
            time: 123,
            increment: 456,
        };
        let code = Bson::JavaScriptCode("console.log(1)".to_string());
        let code_w_scope = JavaScriptCodeWithScope {
            code: "console.log(a)".to_string(),
            scope: doc! { "a": 1 },
        };
        let oid = ObjectId::new();
        let subdoc = doc! { "k": true, "b": { "hello": "world" } };

        let decimal = {
            let bytes = hex::decode("18000000136400D0070000000000000000000000003A3000").unwrap();
            let d = Document::from_reader(bytes.as_slice()).unwrap();
            match d.get("d") {
                Some(Bson::Decimal128(d)) => *d,
                c => panic!("expected decimal128, got {:?}", c),
            }
        };

        let doc = doc! {
            "x": 1,
            "y": 2_i64,
            "s": "oke",
            "array": [ true, "oke", { "12": 24 } ],
            "bson": 1234.5,
            "oid": oid,
            "null": Bson::Null,
            "subdoc": subdoc.clone(),
            "b": true,
            "d": 12.5,
            "binary": binary.clone(),
            "binary_old": binary_old.clone(),
            "binary_other": binary_other.clone(),
            "date": date,
            "regex": regex.clone(),
            "ts": timestamp,
            "i": { "a": 300, "b": 12345 },
            "undefined": Bson::Undefined,
            "code": code.clone(),
            "code_w_scope": code_w_scope.clone(),
            "decimal": Bson::Decimal128(decimal),
            "symbol": Bson::Symbol("ok".to_string()),
            "min_key": Bson::MinKey,
            "max_key": Bson::MaxKey,
        };

        let v = AllTypes {
            x: 1,
            y: 2,
            s: "oke".to_string(),
            array: vec![
                Bson::Boolean(true),
                Bson::String("oke".to_string()),
                Bson::Document(doc! { "12": 24 }),
            ],
            bson: Bson::Double(1234.5),
            oid,
            null: None,
            subdoc,
            b: true,
            d: 12.5,
            binary,
            binary_old,
            binary_other,
            date,
            regex,
            ts: timestamp,
            i: SubDoc { a: 300, b: 12345 },
            undefined: Bson::Undefined,
            code,
            code_w_scope,
            decimal,
            symbol: Bson::Symbol("ok".to_string()),
            min_key: Bson::MinKey,
            max_key: Bson::MaxKey,
        };

        (v, doc)
    }
}

#[test]
fn all_types() {
    let (v, doc) = AllTypes::fixtures();

    run_test(&v, &doc, "all types");
}

#[test]
fn all_types_json() {
    let (mut v, _) = AllTypes::fixtures();

    let code = match v.code {
        Bson::JavaScriptCode(ref c) => c.clone(),
        c => panic!("expected code, found {:?}", c),
    };

    let code_w_scope = JavaScriptCodeWithScope {
        code: "hello world".to_string(),
        scope: doc! { "x": 1 },
    };
    let scope_json = serde_json::json!({ "x": 1 });
    v.code_w_scope = code_w_scope.clone();

    let json = serde_json::json!({
        "x": 1,
        "y": 2,
        "s": "oke",
        "array": vec![
            serde_json::json!(true),
            serde_json::json!("oke".to_string()),
            serde_json::json!({ "12": 24 }),
        ],
        "bson": 1234.5,
        "oid": { "$oid": v.oid.to_hex() },
        "null": serde_json::Value::Null,
        "subdoc": { "k": true, "b": { "hello": "world" } },
        "b": true,
        "d": 12.5,
        "binary": v.binary.bytes,
        "binary_old": { "$binary": { "base64": base64::encode(&v.binary_old.bytes), "subType": "02" } },
        "binary_other": { "$binary": { "base64": base64::encode(&v.binary_old.bytes), "subType": "81" } },
        "date": { "$date": { "$numberLong": v.date.timestamp_millis().to_string() } },
        "regex": { "$regularExpression": { "pattern": v.regex.pattern, "options": v.regex.options } },
        "ts": { "$timestamp": { "t": 123, "i": 456 } },
        "i": { "a": v.i.a, "b": v.i.b },
        "undefined": { "$undefined": true },
        "code": { "$code": code },
        "code_w_scope": { "$code": code_w_scope.code, "$scope": scope_json },
        "decimal": { "$numberDecimalBytes": v.decimal.bytes() },
        "symbol": { "$symbol": "ok" },
        "min_key": { "$minKey": 1 },
        "max_key": { "$maxKey": 1 },
    });

    assert_eq!(serde_json::to_value(&v).unwrap(), json);
}

#[test]
fn all_types_rmp() {
    let (v, _) = AllTypes::fixtures();
    let serialized = rmp_serde::to_vec_named(&v).unwrap();
    let back: AllTypes = rmp_serde::from_slice(&serialized).unwrap();

    assert_eq!(back, v);
}

#[test]
fn all_raw_types_rmp() {
    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct AllRawTypes<'a> {
        #[serde(borrow)]
        bson: RawBson<'a>,
        #[serde(borrow)]
        document: &'a RawDocument,
        #[serde(borrow)]
        array: &'a RawArray,
        buf: RawDocumentBuf,
        #[serde(borrow)]
        binary: RawBinary<'a>,
        #[serde(borrow)]
        code_w_scope: RawJavaScriptCodeWithScope<'a>,
        #[serde(borrow)]
        regex: RawRegex<'a>,
    }

    let doc_bytes = bson::to_vec(&doc! {
        "bson": "some string",
        "array": [1, 2, 3],
        "binary": Binary { bytes: vec![1, 2, 3], subtype: BinarySubtype::Generic },
        "binary_old": Binary { bytes: vec![1, 2, 3], subtype: BinarySubtype::BinaryOld },
        "code_w_scope": JavaScriptCodeWithScope {
            code: "ok".to_string(),
            scope: doc! { "x": 1 },
        },
        "regex": Regex {
            pattern: "pattern".to_string(),
            options: "opt".to_string()
        }
    })
    .unwrap();
    let doc_buf = RawDocumentBuf::new(doc_bytes).unwrap();
    let document = &doc_buf;
    let array = document.get_array("array").unwrap();

    let v = AllRawTypes {
        bson: document.get("bson").unwrap().unwrap(),
        array,
        document,
        buf: doc_buf.clone(),
        binary: document.get_binary("binary").unwrap(),
        code_w_scope: document
            .get("code_w_scope")
            .unwrap()
            .unwrap()
            .as_javascript_with_scope()
            .unwrap(),
        regex: document.get_regex("regex").unwrap(),
    };
    let serialized = rmp_serde::to_vec_named(&v).unwrap();
    let back: AllRawTypes = rmp_serde::from_slice(&serialized).unwrap();

    assert_eq!(back, v);
}

#[test]
fn borrowed() {
    #[derive(Debug, Deserialize, PartialEq)]
    struct Foo<'a> {
        s: &'a str,
        binary: &'a [u8],
        doc: Inner<'a>,
        #[serde(borrow)]
        cow: Cow<'a, str>,
        #[serde(borrow)]
        array: Vec<&'a str>,
    }

    #[derive(Debug, Deserialize, PartialEq)]
    struct Inner<'a> {
        string: &'a str,
    }

    let binary = Binary {
        bytes: vec![36, 36, 36],
        subtype: BinarySubtype::Generic,
    };

    let doc = doc! {
        "s": "borrowed string",
        "binary": binary.clone(),
        "doc": {
            "string": "another borrowed string",
        },
        "cow": "cow",
        "array": ["borrowed string"],
    };
    let mut bson = Vec::new();
    doc.to_writer(&mut bson).unwrap();

    let s = "borrowed string".to_string();
    let ss = "another borrowed string".to_string();
    let cow = "cow".to_string();
    let inner = Inner {
        string: ss.as_str(),
    };
    let v = Foo {
        s: s.as_str(),
        binary: binary.bytes.as_slice(),
        doc: inner,
        cow: Cow::Borrowed(cow.as_str()),
        array: vec![s.as_str()],
    };

    let deserialized: Foo =
        bson::from_slice(bson.as_slice()).expect("deserialization should succeed");
    assert_eq!(deserialized, v);
}

#[test]
fn u2i() {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Foo {
        u_8: u8,
        u_16: u16,
        u_32: u32,
        u_32_max: u32,
        u_64: u64,
        i_64_max: u64,
    }

    let v = Foo {
        u_8: 15,
        u_16: 123,
        u_32: 1234,
        u_32_max: u32::MAX,
        u_64: 12345,
        i_64_max: i64::MAX as u64,
    };

    let expected = doc! {
        "u_8": 15_i32,
        "u_16": 123_i32,
        "u_32": 1234_i64,
        "u_32_max": u32::MAX as i64,
        "u_64": 12345_i64,
        "i_64_max": i64::MAX,
    };

    run_test(&v, &expected, "u2i - valid");

    #[derive(Serialize, Debug)]
    struct TooBig {
        u_64: u64,
    }
    let v = TooBig {
        u_64: i64::MAX as u64 + 1,
    };
    bson::to_document(&v).unwrap_err();
    bson::to_vec(&v).unwrap_err();
}

#[test]
fn serde_with_chrono() {
    #[serde_with::serde_as]
    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    struct Foo {
        #[serde_as(as = "Option<bson::DateTime>")]
        as_bson: Option<chrono::DateTime<chrono::Utc>>,

        #[serde_as(as = "Option<bson::DateTime>")]
        none_bson: Option<chrono::DateTime<chrono::Utc>>,
    }

    let f = Foo {
        as_bson: Some(bson::DateTime::now().into()),
        none_bson: None,
    };
    let expected = doc! {
        "as_bson": Bson::DateTime(f.as_bson.unwrap().into()),
        "none_bson": Bson::Null
    };

    run_test(&f, &expected, "serde_with - chrono");
}

#[test]
fn serde_with_uuid() {
    #[serde_with::serde_as]
    #[derive(Deserialize, Serialize, PartialEq, Debug)]
    struct Foo {
        #[serde_as(as = "Option<bson::Uuid>")]
        as_bson: Option<uuid::Uuid>,

        #[serde_as(as = "Option<bson::Uuid>")]
        none_bson: Option<uuid::Uuid>,
    }

    let f = Foo {
        as_bson: Some(uuid::Uuid::new_v4()),
        none_bson: None,
    };
    let expected = doc! {
        "as_bson": bson::Uuid::from(f.as_bson.unwrap()),
        "none_bson": Bson::Null
    };

    run_test(&f, &expected, "serde_with - uuid");
}

#[test]
fn hint_cleared() {
    #[derive(Debug, Serialize, Deserialize)]
    struct Foo<'a> {
        #[serde(borrow)]
        doc: &'a RawDocument,
        #[serde(borrow)]
        binary: RawBinary<'a>,
    }

    let binary_value = Binary {
        bytes: vec![1, 2, 3, 4],
        subtype: BinarySubtype::Generic,
    };

    let doc_value = doc! {
        "binary": binary_value.clone()
    };

    let bytes = bson::to_vec(&doc_value).unwrap();

    let doc = RawDocument::new(&bytes).unwrap();
    let binary = doc.get_binary("binary").unwrap();

    let f = Foo { doc, binary };

    let serialized_bytes = bson::to_vec(&f).unwrap();
    let round_doc: Document = bson::from_slice(&serialized_bytes).unwrap();

    assert_eq!(round_doc, doc! { "doc": doc_value, "binary": binary_value });
}

#[test]
fn non_human_readable() {
    let bytes = vec![1, 2, 3, 4];
    let binary = RawBinary {
        bytes: &bytes,
        subtype: BinarySubtype::BinaryOld,
    };

    let doc_bytes = bson::to_vec(&doc! { "a": "b", "array": [1, 2, 3] }).unwrap();
    let doc = RawDocument::new(doc_bytes.as_slice()).unwrap();
    let arr = doc.get_array("array").unwrap();
    let oid = ObjectId::new();
    let uuid = Uuid::new();

    #[derive(Debug, Deserialize, Serialize)]
    struct Foo<'a> {
        #[serde(borrow)]
        binary: RawBinary<'a>,
        #[serde(borrow)]
        doc: &'a RawDocument,
        #[serde(borrow)]
        arr: &'a RawArray,
        oid: ObjectId,
        uuid: Uuid,
    }

    let val = Foo {
        binary,
        doc,
        arr,
        oid,
        uuid,
    };

    let human_readable = bson::to_bson(&val).unwrap();
    let non_human_readable = bson::to_bson_with_options(
        &val,
        SerializerOptions::builder().human_readable(false).build(),
    )
    .unwrap();

    let expected = bson!({
        "binary": Binary { bytes: bytes.clone(), subtype: BinarySubtype::BinaryOld },
        "doc": {
            "a": "b",
            "array": [1, 2, 3],
        },
        "arr": [1, 2, 3],
        "oid": oid,
        "uuid": uuid
    });
    assert_eq!(human_readable, expected);
    assert_eq!(human_readable, non_human_readable);
}
