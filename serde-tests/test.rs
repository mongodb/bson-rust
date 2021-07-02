#![allow(clippy::cognitive_complexity)]
#![allow(clippy::vec_init_then_push)]

use pretty_assertions::assert_eq;
use serde::{
    self,
    de::{DeserializeOwned, Unexpected},
    Deserialize,
    Serialize,
};

use std::collections::{BTreeMap, HashSet};

use bson::{
    doc,
    oid::ObjectId,
    spec::BinarySubtype,
    Binary,
    Bson,
    DateTime,
    Decimal128,
    Deserializer,
    Document,
    JavaScriptCodeWithScope,
    Regex,
    Timestamp,
};

/// Verifies the following:
///   - round trip `expected_value` through `Document`:
///     - serializing the `expected_value` to a `Document` matches the `expected_doc`
///     - deserializing from the serialized document produces `expected_value`
///   - round trip through raw BSON:
///     - deserializing a `T` from the raw BSON version of `expected_doc` produces `expected_value`
///     - desierializing a `Document` from the raw BSON version of `expected_doc` produces
///       `expected_doc`
fn run_test<T>(expected_value: &T, expected_doc: &Document, description: &str)
where
    T: Serialize + DeserializeOwned + PartialEq + std::fmt::Debug,
{
    let mut expected_bytes = Vec::new();
    expected_doc
        .to_writer(&mut expected_bytes)
        .expect(description);

    let serialized_doc = bson::to_document(&expected_value).expect(description);
    assert_eq!(&serialized_doc, expected_doc, "{}", description);
    assert_eq!(
        expected_value,
        &bson::from_document::<T>(serialized_doc).expect(description),
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
    bson::from_document::<Foo>(doc.clone()).expect_err("extra filds should cause failure");

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
fn all_types() {
    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct Bar {
        a: i32,
        b: i32,
    }

    #[derive(Debug, Deserialize, Serialize, PartialEq)]
    struct Foo {
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
        i: Bar,
        undefined: Bson,
        code: Bson,
        code_w_scope: JavaScriptCodeWithScope,
        decimal: Decimal128,
        symbol: Bson,
        min_key: Bson,
        max_key: Bson,
    }

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
        "decimal": Bson::Decimal128(Decimal128::from_i32(5)),
        "symbol": Bson::Symbol("ok".to_string()),
        "min_key": Bson::MinKey,
        "max_key": Bson::MaxKey,
    };

    let v = Foo {
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
        i: Bar { a: 300, b: 12345 },
        undefined: Bson::Undefined,
        code,
        code_w_scope,
        decimal: Decimal128::from_i32(5),
        symbol: Bson::Symbol("ok".to_string()),
        min_key: Bson::MinKey,
        max_key: Bson::MaxKey,
    };

    run_test(&v, &doc, "all types");
}
