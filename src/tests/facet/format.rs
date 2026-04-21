use std::io::Cursor;

use crate::{
    Binary,
    Bson,
    DateTime,
    DbPointer,
    Decimal128,
    Document,
    JavaScriptCodeWithScope,
    Regex,
    Timestamp,
    cstr,
    facet::{deserialize_from_slice, serialize_to_vec},
    oid::ObjectId,
    spec::BinarySubtype,
};

use facet::Facet;

#[test]
fn simple_serialize() {
    #[derive(Facet, Debug)]
    struct Inner {
        value: i32,
    }

    #[derive(Facet, Debug)]
    struct Outer {
        inner: Inner,
        other: i32,
    }

    let bytes = serialize_to_vec(&Outer {
        inner: Inner { value: 42 },
        other: 13,
    })
    .unwrap();
    let doc = Document::from_reader(Cursor::new(bytes)).unwrap();
    assert_eq!(doc, doc! { "inner": { "value": 42 }, "other": 13 });
}

#[test]
fn complex_serialize() {
    #[derive(Facet, Debug)]
    struct Inner {
        value: i32,
        arr: Vec<&'static str>,
    }

    #[derive(Facet, Debug)]
    struct Outer {
        inner: Vec<Inner>,
        other: i32,
        more: bool,
    }

    let bytes = serialize_to_vec(&Outer {
        inner: vec![
            Inner {
                value: 42,
                arr: vec!["hello", "world"],
            },
            Inner {
                value: 13,
                arr: vec!["goodbye", "serde"],
            },
        ],
        other: 1066,
        more: true,
    })
    .unwrap();
    let doc = Document::from_reader(Cursor::new(bytes)).unwrap();
    assert_eq!(
        doc,
        doc! {
            "inner": [
                { "value": 42, "arr": ["hello", "world"] },
                { "value": 13, "arr": ["goodbye", "serde"] },
            ],
            "other": 1066,
            "more": true,
        }
    );
}

#[test]
fn array_serialize() {
    #[derive(Facet, Debug)]
    struct Outer {
        value: Vec<i32>,
    }

    let bytes = serialize_to_vec(&Outer {
        value: vec![42, 13],
    })
    .unwrap();
    let doc = Document::from_reader(Cursor::new(bytes)).unwrap();
    assert_eq!(doc, doc! { "value": [42, 13] });
}

fn value_serialize<T>(v: T)
where
    T: Facet<'static> + TryInto<Bson, Error: std::fmt::Debug> + Clone,
{
    #[derive(Facet)]
    struct Outer<T> {
        value: T,
    }
    let bytes = serialize_to_vec(&Outer { value: v.clone() }).unwrap();
    let doc = Document::from_reader(Cursor::new(bytes)).unwrap();
    let bson_val: Bson = v.try_into().unwrap();
    assert_eq!(doc, doc! { "value": bson_val });
}

#[test]
fn regex_serialize() {
    value_serialize(Regex {
        pattern: cstr!("foo.*bar").to_owned(),
        options: cstr!("").to_owned(),
    });
}

#[test]
fn binary_serialize() {
    value_serialize(Binary {
        subtype: BinarySubtype::Generic,
        bytes: vec![1, 2, 3, 4],
    });
}

#[test]
fn timestamp_serialize() {
    value_serialize(Timestamp {
        time: 1234,
        increment: 5,
    });
}

#[test]
fn object_id_serialize() {
    value_serialize(ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap());
}

#[test]
fn datetime_serialize() {
    value_serialize(DateTime::from_millis(1_000_000_000_000));
}

#[test]
fn decimal128_serialize() {
    value_serialize("3.14".parse::<Decimal128>().unwrap());
}

#[test]
fn javascript_code_with_scope_serialize() {
    value_serialize(JavaScriptCodeWithScope {
        code: "function(x) { return x + n; }".into(),
        scope: doc! { "n": 1 },
    });
}

#[test]
fn db_pointer_serialize() {
    let id = ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap();
    value_serialize(DbPointer {
        namespace: "test.coll".into(),
        id,
    });
}

#[test]
fn document_serialize() {
    value_serialize(doc! { "hello": "world" });
}

#[test]
fn bson_serialize() {
    value_serialize(Bson::Null);
    value_serialize(Bson::Int32(13));
}

#[test]
fn rawdoc_serialize() {
    value_serialize(rawdoc! { "hello": "world" });
}

#[test]
fn rawarr_serialize() {
    value_serialize([1, 2, 3].into_iter().collect::<crate::RawArrayBuf>());
}

#[test]
fn rawjsc_serialize() {
    value_serialize(crate::RawJavaScriptCodeWithScope {
        code: "a+b".to_owned(),
        scope: rawdoc! { "a": 1, "b": 2 },
    });
}

#[test]
fn rawbson_serialize() {
    value_serialize(crate::RawBson::Int32(9023));
}

#[test]
fn simple_deserialize() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        val: i32,
        next: i32,
    }
    let bytes = rawdoc! { "val": 42, "next": 13 }.into_bytes();
    let f: Foo = deserialize_from_slice(&bytes).unwrap();
    assert_eq!(Foo { val: 42, next: 13 }, f);
}

#[test]
fn nested_deserialize() {
    #[derive(Debug, PartialEq, Facet)]
    struct Foo {
        val: i32,
        next: i32,
        inner: Bar,
        last: i32,
    }
    #[derive(Debug, PartialEq, Facet)]
    struct Bar {
        a: i32,
        b: i32,
    }
    let bytes =
        rawdoc! { "val": 42, "next": 13, "inner": { "a": 1, "b": 2 }, "last": 1066 }.into_bytes();
    let f: Foo = deserialize_from_slice(&bytes).unwrap();
    assert_eq!(
        Foo {
            val: 42,
            next: 13,
            inner: Bar { a: 1, b: 2 },
            last: 1066
        },
        f,
    );
}
