# bson-rs

[![crates.io](https://img.shields.io/crates/v/bson.svg)](https://crates.io/crates/bson)
[![crates.io](https://img.shields.io/crates/l/bson.svg)](https://crates.io/crates/bson)

Encoding and decoding support for BSON in Rust

## Index
- [Overview of BSON Format](#overview-of-bson-format)
- [Usage](#usage)
    - [BSON Values](#bson-values)
    - [BSON Documents](#bson-documents)
    - [Modeling BSON with strongly typed data structures](#modeling-bson-with-strongly-typed-data-structures)
- [Breaking Changes](#breaking-changes)
- [Contributing](#contributing)
- [Running the Tests](#running-the-tests)
- [Continuous Integration](#continuous-integration)

## Useful links
- [API Documentation](https://docs.rs/bson/)
- [Serde Documentation](https://serde.rs/)

## Installation
This crate works with Cargo and can be found on
[crates.io](https://crates.io/crates/bson) with a `Cargo.toml` like:

```toml
[dependencies]
bson = "1.2.0"
```

## Overview of BSON Format

BSON, short for Binary JSON, is a binary-encoded serialization of JSON-like documents.
Like JSON, BSON supports the embedding of documents and arrays within other documents
and arrays. BSON also contains extensions that allow representation of data types that
are not part of the JSON spec. For example, BSON has a datetime type and a binary data type.

```text
// JSON equivalent
{"hello": "world"}

// BSON encoding
\x16\x00\x00\x00                   // total document size
\x02                               // 0x02 = type String
hello\x00                          // field name
\x06\x00\x00\x00world\x00          // field value
\x00                               // 0x00 = type EOO ('end of object')
```

BSON is the primary data representation for [MongoDB](https://www.mongodb.com/), and this crate is used in the
[`mongodb`](https://docs.rs/mongodb/0.10.0/mongodb/) driver crate in its API and implementation.

For more information about BSON itself, see [bsonspec.org](http://bsonspec.org).

## Usage

### BSON values

Many different types can be represented as a BSON value, including 32-bit and 64-bit signed
integers, 64 bit floating point numbers, strings, datetimes, embedded documents, and more. To
see a full list of possible BSON values, see the [BSON specification](http://bsonspec.org/spec.html). The various
possible BSON values are modeled in this crate by the [`Bson`](https://docs.rs/bson/latest/bson/enum.Bson.html) enum.

#### Creating [`Bson`](https://docs.rs/bson/latest/bson/enum.Bson.html) instances

[`Bson`](https://docs.rs/bson/latest/bson/enum.Bson.html) values can be instantiated directly or via the
[`bson!`](https://docs.rs/bson/latest/bson/macro.bson.html) macro:

```rust
let string = Bson::String("hello world".to_string());
let int = Bson::Int32(5);
let array = Bson::Array(vec![Bson::Int32(5), Bson::Boolean(false)]);

let string: Bson = "hello world".into();
let int: Bson = 5i32.into();

let string = bson!("hello world");
let int = bson!(5);
let array = bson!([5, false]);
```
[`bson!`](https://docs.rs/bson/latest/bson/macro.bson.html) has supports both array and object literals, and it automatically converts any values specified to [`Bson`](https://docs.rs/bson/latest/bson/enum.Bson.html), provided they are `Into<Bson>`.

#### [`Bson`](https://docs.rs/bson/latest/bson/enum.Bson.html) value unwrapping

[`Bson`](https://docs.rs/bson/latest/bson/enum.Bson.html) has a number of helper methods for accessing the underlying native Rust types. These helpers can be useful in circumstances in which the specific type of a BSON value
is known ahead of time.

e.g.:
```rust
let value = Bson::Int32(5);
let int = value.as_i32(); // Some(5)
let bool = value.as_bool(); // None

let value = bson!([true]);
let array = value.as_array(); // Some(&Vec<Bson>)
```

### BSON documents

BSON documents are ordered maps of UTF-8 encoded strings to BSON values. They are logically similar to JSON objects in that they can contain subdocuments, arrays, and values of several different types. This crate models BSON documents via the
[`Document`](https://docs.rs/bson/latest/bson/document/struct.Document.html) struct.

#### Creating [`Document`](https://docs.rs/bson/latest/bson/document/struct.Document.html)s

[`Document`](https://docs.rs/bson/latest/bson/document/struct.Document.html)s can be created directly either from a byte
reader containing BSON data or via the `doc!` macro:
```rust
let mut bytes = hex::decode("0C0000001069000100000000").unwrap();
let doc = Document::from_reader(&mut bytes.as_slice()).unwrap(); // { "i": 1 }

let doc = doc! {
   "hello": "world",
   "int": 5,
   "subdoc": { "cat": true },
};
```
[`doc!`](https://docs.rs/bson/latest/bson/macro.doc.html) works similarly to [`bson!`](https://docs.rs/bson/latest/bson/macro.bson.html), except that it always
returns a [`Document`](https://docs.rs/bson/latest/bson/document/struct.Document.html) rather than a [`Bson`](https://docs.rs/bson/latest/bson/enum.Bson.html).

#### [`Document`](https://docs.rs/bson/latest/bson/document/struct.Document.html) member access

[`Document`](https://docs.rs/bson/latest/bson/document/struct.Document.html) has a number of methods on it to facilitate member
access:

```rust
let doc = doc! {
   "string": "string",
   "bool": true,
   "i32": 5,
   "doc": { "x": true },
};

// attempt get values as untyped Bson
let none = doc.get("asdfadsf"); // None
let value = doc.get("string"); // Some(&Bson::String("string"))

// attempt to get values with explicit typing
let string = doc.get_str("string"); // Ok("string")
let subdoc = doc.get_document("doc"); // Some(Document({ "x": true }))
let error = doc.get_i64("i32"); // Err(...)
```

### Modeling BSON with strongly typed data structures

While it is possible to work with documents and BSON values directly, it will often introduce a
lot of boilerplate for verifying the necessary keys are present and their values are the correct
types. [`serde`](https://serde.rs/) provides a powerful way of mapping BSON data into Rust data structures largely
automatically, removing the need for all that boilerplate.

e.g.:
```rust
#[derive(Serialize, Deserialize)]
struct Person {
    name: String,
    age: i32,
    phones: Vec<String>,
}

// Some BSON input data as a `Bson`.
let bson_data: Bson = bson!({
    "name": "John Doe",
    "age": 43,
    "phones": [
        "+44 1234567",
        "+44 2345678"
    ]
});

// Deserialize the Person struct from the BSON data, automatically
// verifying that the necessary keys are present and that they are of
// the correct types.
let mut person: Person = bson::from_bson(bson_data).unwrap();

// Do things just like with any other Rust data structure.
println!("Redacting {}'s record.", person.name);
person.name = "REDACTED".to_string();

// Get a serialized version of the input data as a `Bson`.
let redacted_bson = bson::to_bson(&person).unwrap();
```

Any types that implement `Serialize` and `Deserialize` can be used in this way. Doing so helps
separate the "business logic" that operates over the data from the (de)serialization logic that
translates the data to/from its serialized form. This can lead to more clear and concise code
that is also less error prone.

## Breaking Changes

In the BSON specification, _unsigned integer types_ are unsupported; for example, `u32`. In the older version of this crate (< `v0.8.0`), if you uses `serde` to serialize _unsigned integer types_ into BSON, it will store them with `Bson::Double` type. From `v0.8.0`, we removed this behavior and simply returned an error when you want to serialize _unsigned integer types_ to BSON. [#72](https://github.com/zonyitoo/bson-rs/pull/72)

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
    assert_eq!(b, Bson::Document(doc! { "x": Bson::Double(20.0) }));

    let de_foo = bson::from_bson::<Foo>(b).unwrap();
    assert_eq!(de_foo, foo);
}
```

In this example, we added an attribute `#[serde(with = "bson::compat::u2f")]` on field `x`, which will tell `serde` to use the `bson::compat::u2f::serialize` and `bson::compat::u2f::deserialize` methods to process this field.

## Contributing

We encourage and would happily accept contributions in the form of GitHub pull requests. Before opening one, be sure to run the tests locally; check out the [testing section](#running-the-tests) for information on how to do that. Once you open a pull request, your branch will be run against the same testing matrix that we use for our [continuous integration](#continuous-integration) system, so it is usually sufficient to only run the integration tests locally against a standalone. Remember to always run the linter tests before opening a pull request.

## Running the tests

### Integration and unit tests

To actually run the tests, you can use `cargo` like you would in any other crate:
```bash
cargo test --verbose # runs against localhost:27017
```

### Linter Tests
Our linter tests use the nightly version of `rustfmt` to verify that the source is formatted properly and the stable version of `clippy` to statically detect any common mistakes.
You can use `rustup` to install them both:
```bash
rustup component add clippy --toolchain stable
rustup component add rustfmt --toolchain nightly
```
To run the linter tests, run the `check-clippy.sh` and `check-rustfmt.sh` scripts in the `.evergreen` directory:
```bash
bash .evergreen/check-clippy.sh && bash .evergreen/check-rustfmt.sh
```

## Continuous Integration
Commits to master are run automatically on [evergreen](https://evergreen.mongodb.com/waterfall/rust-bson).
