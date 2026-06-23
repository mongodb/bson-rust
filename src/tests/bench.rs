extern crate test;

#[cfg(feature = "facet-unstable")]
use facet::Facet;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use test::Bencher;

use crate::{Document, doc};

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "facet-unstable", derive(Facet))]
struct Foo {
    bar: Bar,
    value: String,
}

impl Foo {
    fn new() -> Self {
        Self {
            bar: Bar { inner: 42 },
            value: "hello".to_owned(),
        }
    }
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "facet-unstable", derive(Facet))]
struct Bar {
    inner: i32,
}

#[cfg(feature = "serde")]
#[bench]
fn serde_serialize(b: &mut Bencher) {
    let value = Foo::new();
    b.iter(|| crate::serialize_to_vec(&value).unwrap());
}

#[cfg(feature = "serde")]
#[bench]
fn serde_deserialize(b: &mut Bencher) {
    let bytes = crate::serialize_to_vec(&Foo::new()).unwrap();
    b.iter(|| crate::deserialize_from_slice::<Foo>(&bytes).unwrap());
}

#[cfg(feature = "facet-unstable")]
#[bench]
fn facet_serialize(b: &mut Bencher) {
    let value = Foo::new();
    b.iter(|| crate::facet::serialize_to_vec(&value).unwrap());
}

#[cfg(feature = "facet-unstable")]
#[bench]
fn facet_deserialize(b: &mut Bencher) {
    let bytes = crate::facet::serialize_to_vec(&Foo::new()).unwrap();
    b.iter(|| crate::facet::deserialize_from_slice::<Foo>(&bytes).unwrap());
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "facet-unstable", derive(Facet))]
struct WithBson {
    bar: Bar,
    value: String,
    d: Document,
}

impl WithBson {
    fn new() -> Self {
        Self {
            bar: Bar { inner: 42 },
            value: "hello".to_owned(),
            d: doc! {
                "one": true,
                "two": [1.5, "thing"],
            },
        }
    }
}

#[cfg(feature = "serde")]
#[bench]
fn serde_bson_serialize(b: &mut Bencher) {
    let value = WithBson::new();
    b.iter(|| crate::serialize_to_vec(&value).unwrap());
}

#[cfg(feature = "serde")]
#[bench]
fn serde_bson_deserialize(b: &mut Bencher) {
    let bytes = crate::serialize_to_vec(&WithBson::new()).unwrap();
    b.iter(|| crate::deserialize_from_slice::<WithBson>(&bytes).unwrap());
}

#[cfg(feature = "facet-unstable")]
#[bench]
fn facet_bson_serialize(b: &mut Bencher) {
    let value = WithBson::new();
    b.iter(|| crate::facet::serialize_to_vec(&value).unwrap());
}

#[cfg(feature = "facet-unstable")]
#[bench]
fn facet_bson_deserialize(b: &mut Bencher) {
    let bytes = crate::facet::serialize_to_vec(&WithBson::new()).unwrap();
    b.iter(|| crate::facet::deserialize_from_slice::<WithBson>(&bytes).unwrap());
}
