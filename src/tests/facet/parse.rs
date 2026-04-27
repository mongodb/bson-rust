use crate::{
    Binary,
    Bson,
    DateTime,
    DbPointer,
    Decimal128,
    JavaScriptCodeWithScope,
    RawArrayBuf,
    RawBson,
    RawJavaScriptCodeWithScope,
    Regex,
    Timestamp,
    facet::deserialize_from_slice,
    oid::ObjectId,
    raw::CString,
    spec::BinarySubtype,
};

use facet::Facet;

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
    value_deserialize(RawBson::Double(std::f64::consts::PI));
}

#[test]
fn bson_deserialize() {
    value_deserialize_cooked(Bson::Double(std::f64::consts::PI));
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
    value_deserialize(std::f64::consts::PI);
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
