extern crate test;

use test::Bencher;

#[cfg(feature = "facet-unstable")]
use facet::Facet;
#[cfg(feature = "serde")]
use serde::Serialize;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "facet-unstable", derive(Facet))]
struct Foo {
    bar: Bar,
    value: String,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Serialize))]
#[cfg_attr(feature = "facet-unstable", derive(Facet))]
struct Bar {
    inner: i32,
}

#[cfg(feature = "serde")]
#[bench]
fn serde_serialize(b: &mut Bencher) {
    let value = Foo {
        bar: Bar { inner: 42 },
        value: "hello".to_owned(),
    };
    b.iter(|| crate::serialize_to_vec(&value).unwrap());
}

#[cfg(feature = "facet-unstable")]
#[bench]
fn facet_serialize(b: &mut Bencher) {
    let value = Foo {
        bar: Bar { inner: 42 },
        value: "hello".to_owned(),
    };
    b.iter(|| crate::facet::format::to_vec(&value).unwrap());
}
