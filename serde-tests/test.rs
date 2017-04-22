extern crate bson;
extern crate serde;
#[macro_use] extern crate serde_derive;

use std::collections::{BTreeMap, HashSet};
use serde::{Deserialize, Serialize, Deserializer};
use serde::de::Unexpected;

use bson::{Bson, Encoder, Decoder, DecoderError};

macro_rules! bson {
    ([]) => {{ bson::Bson::Array(Vec::new()) }};

    ([$($val:tt),*]) => {{
        let mut array = Vec::new();

        $(
            array.push(bson!($val));
        )*

        bson::Bson::Array(array)
    }};

    ([$val:expr]) => {{
        bson::Bson::Array(vec!(::std::convert::From::from($val)))
    }};

    ({ $($k:expr => $v:tt),* }) => {{
        bdoc! {
            $(
                $k => $v
            ),*
        }
    }};

    ($val:expr) => {{
        ::std::convert::From::from($val)
    }};
}

macro_rules! bdoc {
    () => {{ Bson::Document(bson::Document::new()) }};

    ( $($key:expr => $val:tt),* ) => {{
        let mut document = bson::Document::new();

        $(
            document.insert_bson($key.to_owned(), bson!($val));
        )*

        Bson::Document(document)
    }};
}

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(t) => t,
        Err(e) => panic!("Failed with {:?}", e),
    })
}

macro_rules! encode( ($t:expr) => ({
    let e = Encoder::new();
    match $t.serialize(e) {
        Ok(b) => b,
        Err(e) => panic!("Failed to serialize: {}", e),
    }
}) );

macro_rules! decode( ($t:expr) => ({
    let d = Decoder::new($t);
    t!(Deserialize::deserialize(d))
}) );

#[test]
fn smoke() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: isize,
    }

    let v = Foo { a: 2 };
    assert_eq!(encode!(v), bdoc! {"a" => (2 as i64)});
    assert_eq!(v, decode!(encode!(v)));
}

#[test]
fn smoke_under() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a_b: isize,
    }

    let v = Foo { a_b: 2 };
    assert_eq!(encode!(v), bdoc! { "a_b" => (2 as i64) });
    assert_eq!(v, decode!(encode!(v)));

    let mut m = BTreeMap::new();
    m.insert("a_b".to_string(), 2 as i64);
    assert_eq!(v, decode!(encode!(m)));
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
        b: Bar { a: "test".to_string() },
    };
    assert_eq!(encode!(v),
               bdoc! {
                   "a" => (2 as i64),
                   "b" => {
                       "a" => "test"
                   }
               });
    assert_eq!(v, decode!(encode!(v)));
}

#[test]
fn application_decode_error() {
    #[derive(PartialEq, Debug)]
    struct Range10(usize);
    impl<'de> Deserialize<'de> for Range10 {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Range10, D::Error> {
            let x: usize = try!(Deserialize::deserialize(d));
            if x > 10 {
                Err(serde::de::Error::invalid_value(Unexpected::Unsigned(x as u64),
                                                    &"more than 10"))
            } else {
                Ok(Range10(x))
            }
        }
    }
    let d_good = Decoder::new(Bson::I64(5));
    let d_bad1 = Decoder::new(Bson::String("not an isize".to_string()));
    let d_bad2 = Decoder::new(Bson::I64(11));

    assert_eq!(Range10(5), t!(Deserialize::deserialize(d_good)));

    let err1: Result<Range10, _> = Deserialize::deserialize(d_bad1);
    assert!(err1.is_err());
    let err2: Result<Range10, _> = Deserialize::deserialize(d_bad2);
    assert!(err2.is_err());
}

#[test]
fn array() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Vec<i32>,
    }

    let v = Foo { a: vec![1, 2, 3, 4] };
    assert_eq!(encode!(v),
               bdoc! {
                   "a" => [1, 2, 3, 4]
               });
    assert_eq!(v, decode!(encode!(v)));
}

#[test]
fn tuple() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: (i32, i32, i32, i32),
    }

    let v = Foo { a: (1, 2, 3, 4) };
    assert_eq!(encode!(v),
               bdoc! {
                   "a" => [1, 2, 3, 4]
               });
    assert_eq!(v, decode!(encode!(v)));
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
    assert_eq!(encode!(v),
               bdoc! {
                   "a" => {
                       "a" => (Bson::Null),
                       "b" => {
                           "a" => "foo",
                           "b" => (4.5)
                       }
                   },
                   "b" => {
                       "a" => "bar",
                       "b" => (1.0)
                   }
               });
    assert_eq!(v, decode!(encode!(v)));
}

#[test]
fn inner_structs_with_skippable_options() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        #[serde(skip_serializing_if="Option::is_none")]
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
    assert_eq!(encode!(v),
               bdoc! {
                   "a" => {
                   "b" => {
                           "a" => "foo",
                           "b" => (4.5)
                       }
                   },
                   "b" => {
                       "a" => "bar",
                       "b" => (1.0)
                   }
               });
    assert_eq!(v, decode!(encode!(v)));
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
    assert_eq!(encode!(v),
               bdoc! {
            "map" => {
                "bar" => 4,
                "foo" => 10
            },
            "set" => ["a"]
        });
    assert_eq!(v, decode!(encode!(v)));
}

#[test]
fn tuple_struct() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo(i32, String, f64);
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar {
        whee: Foo,
    }

    let v = Bar { whee: Foo(1, "foo".to_string(), 4.5) };
    assert_eq!(encode!(v),
               bdoc! {
            "whee" => [1, "foo", (4.5)]
        });
    assert_eq!(v, decode!(encode!(v)));
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

    let v = Foo { a: vec![Bar { a: 1 }, Bar { a: 2 }] };
    assert_eq!(encode!(v),
               bdoc! {
            "a" => [{"a" => 1}, {"a" => 2}]
        });
    assert_eq!(v, decode!(encode!(v)));
}


#[test]
fn type_conversion() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        bar: i32,
    }

    let d = Decoder::new(bdoc!{
        "bar" => 1
    });
    let a: Result<Foo, DecoderError> = Deserialize::deserialize(d);
    assert_eq!(a.unwrap(), Foo { bar: 1 });
}

#[test]
fn missing_errors() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        bar: i32,
    }

    let d = Decoder::new(bdoc!{});
    let a: Result<Foo, DecoderError> = Deserialize::deserialize(d);

    assert!(a.is_err());
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
    assert_eq!(encode!(v), bdoc! { "a" => "Empty" });
    assert_eq!(v, decode!(encode!(v)));

    let v = Foo { a: E::Bar(10) };
    assert_eq!(encode!(v), bdoc! { "a" => { "Bar" => 10 } });
    assert_eq!(v, decode!(encode!(v)));

    let v = Foo { a: E::Baz(10.2) };
    assert_eq!(encode!(v), bdoc! { "a" => { "Baz" => 10.2 } });
    assert_eq!(v, decode!(encode!(v)));

    let v = Foo { a: E::Pair(12, 42) };
    assert_eq!(encode!(v), bdoc! { "a" => { "Pair" => [ 12, 42] } });
    assert_eq!(v, decode!(encode!(v)));

    let v = Foo { a: E::Last(Foo2 { test: "test".to_string() }) };
    assert_eq!(encode!(v),
               bdoc! { "a" => { "Last" => { "test" => "test" } } });
    assert_eq!(v, decode!(encode!(v)));

    let v = Foo { a: E::Vector(vec![12, 42]) };
    assert_eq!(encode!(v), bdoc! { "a" => { "Vector" => [ 12, 42 ] } });
    assert_eq!(v, decode!(encode!(v)));

    let v = Foo { a: E::Named { a: 12 } };
    assert_eq!(encode!(v), bdoc! { "a" => { "Named" => { "a" => 12 } } });
    assert_eq!(v, decode!(encode!(v)));
    let v = Foo { a: E::MultiNamed { a: 12, b: 42 } };
    assert_eq!(encode!(v),
               bdoc! { "a" => { "MultiNamed" => { "a" => 12, "b" => 42 } } });
    assert_eq!(v, decode!(encode!(v)));
}

#[test]
fn unused_fields() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: i32,
    }

    let v = Foo { a: 2 };
    let d = Decoder::new(bdoc! {
        "a" => 2,
        "b" => 5
    });

    assert_eq!(v, t!(Deserialize::deserialize(d)));
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
    let d = Decoder::new(bdoc! {
        "a" => {
            "a" => 2,
            "b" => 5
        }
    });

    assert_eq!(v, t!(Deserialize::deserialize(d)));
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
    let d = Decoder::new(bdoc! {
        "a" => {
            "a" => 2
        }
    });
    assert_eq!(v, t!(Deserialize::deserialize(d)));
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
    let d = Decoder::new(bdoc! {
        "a" => {
            "a" => "foo"
        }
    });
    assert_eq!(v, t!(Deserialize::deserialize(d)));
}

#[test]
fn unused_fields5() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Vec<String>,
    }

    let v = Foo { a: vec!["a".to_string()] };
    let d = Decoder::new(bdoc! {
        "a" => ["a"]
    });
    assert_eq!(v, t!(Deserialize::deserialize(d)));
}

#[test]
fn unused_fields6() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Option<Vec<String>>,
    }

    let v = Foo { a: Some(vec![]) };
    let d = Decoder::new(bdoc! {
        "a" => []
    });
    assert_eq!(v, t!(Deserialize::deserialize(d)));
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

    let v = Foo { a: vec![Bar { a: 1 }] };
    let d = Decoder::new(bdoc! {
        "a" => [{"a" => 1, "b" => 2}]
    });
    assert_eq!(v, t!(Deserialize::deserialize(d)));
}

#[test]
fn empty_arrays() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        #[serde(default)]
        a: Vec<Bar>,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar;

    let v = Foo { a: vec![] };
    let d = Decoder::new(bdoc!{});
    assert_eq!(v, t!(Deserialize::deserialize(d)));
}

#[test]
fn empty_arrays2() {
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Foo {
        a: Option<Vec<Bar>>,
    }
    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Bar;

    let v = Foo { a: None };
    let d = Decoder::new(bdoc!{});
    assert_eq!(v, t!(Deserialize::deserialize(d)));

    let v = Foo { a: Some(vec![]) };
    let d = Decoder::new(bdoc! {
        "a" => []
    });
    assert_eq!(v, t!(Deserialize::deserialize(d)));
}
