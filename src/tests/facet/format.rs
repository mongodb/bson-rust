use std::io::Cursor;

use crate::{
    Binary,
    Bson,
    DateTime,
    DbPointer,
    Decimal128,
    Document,
    JavaScriptCodeWithScope,
    RawArrayBuf,
    RawBson,
    RawJavaScriptCodeWithScope,
    Regex,
    Timestamp,
    cstr,
    facet::{deserialize_from_slice, serialize_to_vec},
    oid::ObjectId,
    raw::CString,
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
fn regex_json_serialize() {
    let r = Regex::from_strings("foo.*bar", "n").unwrap();
    assert!(matches!(
        facet_json::to_string(&r),
        Err(facet_format::SerializeError::Unsupported(..)),
    ));
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
fn cstring_serialize() {
    value_serialize(cstr!("hello").to_owned());
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

fn value_deserialize<T>(v: T)
where
    T: Facet<'static> + Into<RawBson> + Clone + PartialEq + std::fmt::Debug,
{
    #[derive(Debug, PartialEq, Facet)]
    struct Outer<T> {
        val: i32,
        next: i32,
        inner: T,
        last: i32,
    }
    let bytes = rawdoc! { "val": 42, "next": 13, "inner": v.clone(), "last": 1066 }.into_bytes();
    let o: Outer<T> = deserialize_from_slice(&bytes).unwrap();
    assert_eq!(
        Outer {
            val: 42,
            next: 13,
            inner: v,
            last: 1066
        },
        o,
    );
}

fn value_deserialize_cooked<T>(v: T)
where
    T: Facet<'static> + Into<Bson> + Clone + PartialEq + std::fmt::Debug,
{
    #[derive(Debug, PartialEq, Facet)]
    struct Outer<T> {
        val: i32,
        next: i32,
        inner: T,
        last: i32,
    }
    let bytes = {
        let bv: Bson = v.clone().into();
        let raw_v: RawBson = bv.try_into().unwrap();
        rawdoc! { "val": 42, "next": 13, "inner": raw_v, "last": 1066 }.into_bytes()
    };
    let o: Outer<T> = deserialize_from_slice(&bytes).unwrap();
    assert_eq!(
        Outer {
            val: 42,
            next: 13,
            inner: v,
            last: 1066
        },
        o,
    );
}

#[test]
fn rawdoc_deserialize() {
    value_deserialize(rawdoc! { "a": 1, "b": 2 });
}

#[test]
fn regex_deserialize() {
    value_deserialize(Regex::from_strings("foobar", "n").unwrap());
}

#[test]
fn binary_deserialize() {
    value_deserialize(Binary {
        subtype: BinarySubtype::Generic,
        bytes: vec![0, 1, 2, 3],
    });
}

#[test]
fn timestamp_deserialize() {
    value_deserialize(Timestamp {
        time: 1000,
        increment: 2000,
    });
}

#[test]
fn rawjscws_deserialize() {
    value_deserialize(RawJavaScriptCodeWithScope {
        code: "a+b".to_owned(),
        scope: rawdoc! { "a": 1, "b": 2},
    });
}

#[test]
fn object_id_deserialize() {
    value_deserialize(ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap());
}

#[test]
fn decimal128_deserialize() {
    value_deserialize("3.14".parse::<Decimal128>().unwrap());
}

#[test]
fn datetime_deserialize() {
    value_deserialize(DateTime::from_millis(1_000_000_000_000));
}

#[test]
fn db_pointer_deserialize() {
    value_deserialize(DbPointer {
        namespace: "test.coll".into(),
        id: ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap(),
    });
}

#[test]
fn javascript_code_with_scope_deserialize() {
    value_deserialize_cooked(JavaScriptCodeWithScope {
        code: "function(x) { return x + n; }".into(),
        scope: doc! { "n": 1 },
    });
}

#[test]
fn document_deserialize() {
    value_deserialize_cooked(doc! { "hello": "world", "count": 3 });
}

#[test]
fn rawarr_deserialize() {
    value_deserialize([1, 2, 3].into_iter().collect::<RawArrayBuf>());
}

#[test]
fn cstring_deserialize() {
    value_deserialize(CString::try_from("hello world".to_owned()).unwrap());
}

#[test]
fn rawbson_deserialize() {
    value_deserialize(RawBson::Double(3.14));
}

#[test]
fn bson_deserialize() {
    value_deserialize_cooked(Bson::Double(3.14));
}

#[test]
fn untagged_enum_deserialize() {
    #[derive(Debug, PartialEq, Facet)]
    #[repr(u8)]
    #[facet(untagged)]
    enum Payload {
        A { x: i32 },
        B { y: i32 },
    }

    #[derive(Debug, PartialEq, Facet)]
    struct Wrapper {
        before: i32,
        inner: Payload,
        after: i32,
    }

    let bytes = rawdoc! { "before": 1, "inner": { "x": 99 }, "after": 7 }.into_bytes();
    let w: Wrapper = deserialize_from_slice(&bytes).unwrap();
    assert_eq!(w.inner, Payload::A { x: 99 });

    let bytes = rawdoc! { "before": 1, "inner": { "y": 42 }, "after": 7 }.into_bytes();
    let w: Wrapper = deserialize_from_slice(&bytes).unwrap();
    assert_eq!(
        w,
        Wrapper {
            before: 1,
            inner: Payload::B { y: 42 },
            after: 7,
        }
    );
}

#[test]
fn skip_unknown_field_deserialize() {
    #[derive(Debug, PartialEq, Facet)]
    struct Keep {
        keep: i32,
    }

    let bytes = rawdoc! { "throwaway": 99_i32, "keep": 7 }.into_bytes();
    let k: Keep = deserialize_from_slice(&bytes).unwrap();
    assert_eq!(k, Keep { keep: 7 });

    let bytes = rawdoc! {
        "throwaway": { "a": 1, "b": { "c": 2 } },
        "keep": 7,
    }
    .into_bytes();
    let k: Keep = deserialize_from_slice(&bytes).unwrap();
    assert_eq!(k, Keep { keep: 7 });

    let bytes = rawdoc! {
        "throwaway": [1, 2, 3, 4],
        "keep": 7,
    }
    .into_bytes();
    let k: Keep = deserialize_from_slice(&bytes).unwrap();
    assert_eq!(k, Keep { keep: 7 });

    let bytes = rawdoc! {
        "keep": 7,
        "throwaway": { "nested": 13 },
    }
    .into_bytes();
    let k: Keep = deserialize_from_slice(&bytes).unwrap();
    assert_eq!(k, Keep { keep: 7 });
}

#[test]
fn array_deserialize() {
    value_deserialize_cooked(vec![1, 2]);
}

#[test]
fn nested_array_deserialize() {
    value_deserialize_cooked(vec![vec![1, 2], vec![3, 4]]);
}

#[test]
fn double_deserialize() {
    value_deserialize(3.14_f64);
}

#[test]
fn string_deserialize() {
    value_deserialize("hello world".to_owned());
}

#[test]
fn bool_deserialize() {
    value_deserialize(true);
    value_deserialize(false);
}

#[test]
fn int64_deserialize() {
    value_deserialize(1234567890123_i64);
}

#[test]
fn null_deserialize() {
    value_deserialize_cooked::<Option<i32>>(None);
    value_deserialize_cooked(Some(42_i32));
}
