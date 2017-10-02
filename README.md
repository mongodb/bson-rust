# bson-rs

[![Build Status](https://img.shields.io/travis/zonyitoo/bson-rs.svg)](https://travis-ci.org/zonyitoo/bson-rs)
[![crates.io](https://img.shields.io/crates/v/bson.svg)](https://crates.io/crates/bson)
[![crates.io](https://img.shields.io/crates/l/bson.svg)](https://crates.io/crates/bson)

Encoding and decoding support for BSON in Rust

## Useful links
- [API Documentation](https://docs.rs/bson/)
- [Serde](https://serde.rs/)

## Installation
This crate works with Cargo and can be found on
[crates.io](https://crates.io/crates/bson) with a `Cargo.toml` like:

```toml
[dependencies]
bson = "0.10"
```

## Usage
Link the library in _main.rs_:

```rust
#[macro_use(bson, doc)]
extern crate bson;
```

Prepare your struct for Serde serialization:

```rust
#[derive(Serialize, Deserialize, Debug)]
pub struct Person {
    #[serde(rename = "_id")]  // Use MongoDB's special primary key field name when serializing 
    pub id: String,
    pub name: String,
    pub age: i32
}
```

Serialize the struct:

```rust
use bson;

let person = Person {
    id: "12345",
    name: "Emma",
    age: 3
};

let serialized_person = bson::to_bson(&person)?;  // Serialize

if let bson::Bson::Document(document) = serialized_person {
    mongoCollection.insert_one(document, None)?;  // Insert into a MongoDB collection
} else {
    println!("Error converting the BSON object into a MongoDB document");
}
```

Deserialize the struct:

```rust
// Read the document from a MongoDB collection
let person_document = mongoCollection.find_one(Some(doc! { "_id": "12345" }), None)?
    .expect("Document not found");

// Deserialize the document into a Person instance
let person = bson::from_bson(bson::Bson::Document(person_document))?
```

## Breaking Changes

In the BSON specification, _unsigned integer types_ are unsupported; for example, `u32`. In the older version of this crate (< `v0.8.0`), if you uses `serde` to serialize _unsigned integer types_ into BSON, it will store them with `Bson::FloatingPoint` type. From `v0.8.0`, we removed this behavior and simply returned an error when you want to serialize _unsigned integer types_ to BSON. [#72](https://github.com/zonyitoo/bson-rs/pull/72)

For backward compatibility, we've provided a mod `bson::compat::u2f` to explicitly serialize _unsigned integer types_ into BSON's floating point value as follows:

```rust
#[test]
fn test_compat_u2f() {
    #[derive(Serialize, Deserialize, Eq, PartialEq, Debug)]
    struct Foo {
        #[serde(with = "bson::compat::u2f")]
        x: u32
    }

    let foo = Foo { x: 20 };
    let b = bson::to_bson(&foo).unwrap();
    assert_eq!(b, Bson::Document(doc! { "x": Bson::FloatingPoint(20.0) }));

    let de_foo = bson::from_bson::<Foo>(b).unwrap();
    assert_eq!(de_foo, foo);
}
```

In this example, we added an attribute `#[serde(with = "bson::compat::u2f")]` on field `x`, which will tell `serde` to use the `bson::compat::u2f::serialize` and `bson::compat::u2f::deserialize` methods to process this field.
