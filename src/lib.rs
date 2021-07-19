// The MIT License (MIT)

// Copyright (c) 2015 Y. T. Chung <zonyitoo@gmail.com>

// Permission is hereby granted, free of charge, to any person obtaining a copy of
// this software and associated documentation files (the "Software"), to deal in
// the Software without restriction, including without limitation the rights to
// use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of
// the Software, and to permit persons to whom the Software is furnished to do so,
// subject to the following conditions:

// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.

// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS
// FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR
// COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER
// IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN
// CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

//! BSON, short for Binary JSON, is a binary-encoded serialization of JSON-like documents.
//! Like JSON, BSON supports the embedding of documents and arrays within other documents
//! and arrays. BSON also contains extensions that allow representation of data types that
//! are not part of the JSON spec. For example, BSON has a datetime type and a binary data type.
//!
//! ```text
//! // JSON equivalent
//! {"hello": "world"}
//!
//! // BSON encoding
//! \x16\x00\x00\x00                   // total document size
//! \x02                               // 0x02 = type String
//! hello\x00                          // field name
//! \x06\x00\x00\x00world\x00          // field value
//! \x00                               // 0x00 = type EOO ('end of object')
//! ```
//!
//! BSON is the primary data representation for [MongoDB](https://www.mongodb.com/), and this crate is used in the
//! [`mongodb`](https://docs.rs/mongodb/latest/mongodb/) driver crate in its API and implementation.
//!
//! For more information about BSON itself, see [bsonspec.org](http://bsonspec.org).
//!
//! ## BSON values
//!
//! Many different types can be represented as a BSON value, including 32-bit and 64-bit signed
//! integers, 64 bit floating point numbers, strings, datetimes, embedded documents, and more. To
//! see a full list of possible BSON values, see the [BSON specification](http://bsonspec.org/spec.html). The various
//! possible BSON values are modeled in this crate by the [`Bson`](enum.Bson.html) enum.
//!
//! ### Creating [`Bson`](enum.Bson.html) instances
//!
//! [`Bson`](enum.Bson.html) values can be instantiated directly or via the
//! [`bson!`](macro.bson.html) macro:
//!
//! ```rust
//! # use bson::{bson, Bson};
//! let string = Bson::String("hello world".to_string());
//! let int = Bson::Int32(5);
//! let array = Bson::Array(vec![Bson::Int32(5), Bson::Boolean(false)]);
//!
//! let string: Bson = "hello world".into();
//! let int: Bson = 5i32.into();
//!
//! let string = bson!("hello world");
//! let int = bson!(5);
//! let array = bson!([5, false]);
//! ```
//! [`bson!`](macro.bson.html) has supports both array and object literals, and it automatically
//! converts any values specified to [`Bson`](enum.Bson.html), provided they are `Into<Bson>`.
//!
//! ### [`Bson`](enum.Bson.html) value unwrapping
//!
//! [`Bson`](enum.Bson.html) has a number of helper methods for accessing the underlying native Rust
//! types. These helpers can be useful in circumstances in which the specific type of a BSON value
//! is known ahead of time.
//!
//! e.g.:
//! ```rust
//! # use bson::{bson, Bson};
//! let value = Bson::Int32(5);
//! let int = value.as_i32(); // Some(5)
//! let bool = value.as_bool(); // None
//!
//! let value = bson!([true]);
//! let array = value.as_array(); // Some(&Vec<Bson>)
//! ```
//!
//! ## BSON documents
//!
//! BSON documents are ordered maps of UTF-8 encoded strings to BSON values. They are logically
//! similar to JSON objects in that they can contain subdocuments, arrays, and values of several
//! different types. This crate models BSON documents via the
//! [`Document`](document/struct.Document.html) struct.
//!
//! ### Creating [`Document`](document/struct.Document.html)s
//!
//! [`Document`](document/struct.Document.html)s can be created directly either from a byte
//! reader containing BSON data or via the `doc!` macro:
//! ```rust
//! # use bson::{doc, Document};
//! # use std::io::Read;
//! let mut bytes = hex::decode("0C0000001069000100000000").unwrap();
//! let doc = Document::from_reader(&mut bytes.as_slice()).unwrap(); // { "i": 1 }
//!
//! let doc = doc! {
//!    "hello": "world",
//!    "int": 5,
//!    "subdoc": { "cat": true },
//! };
//! ```
//! [`doc!`](macro.doc.html) works similarly to [`bson!`](macro.bson.html), except that it always
//! returns a [`Document`](document/struct.Document.html) rather than a [`Bson`](enum.Bson.html).
//!
//! ### [`Document`](document/struct.Document.html) member access
//!
//! [`Document`](document/struct.Document.html) has a number of methods on it to facilitate member
//! access:
//!
//! ```rust
//! # use bson::doc;
//! let doc = doc! {
//!    "string": "string",
//!    "bool": true,
//!    "i32": 5,
//!    "doc": { "x": true },
//! };
//!
//! // attempt get values as untyped Bson
//! let none = doc.get("asdfadsf"); // None
//! let value = doc.get("string"); // Some(&Bson::String("string"))
//!
//! // attempt to get values with explicit typing
//! let string = doc.get_str("string"); // Ok("string")
//! let subdoc = doc.get_document("doc"); // Some(Document({ "x": true }))
//! let error = doc.get_i64("i32"); // Err(...)
//! ```
//!
//! ## Modeling BSON with strongly typed data structures
//!
//! While it is possible to work with documents and BSON values directly, it will often introduce a
//! lot of boilerplate for verifying the necessary keys are present and their values are the correct
//! types. [`serde`](https://serde.rs/) provides a powerful way of mapping BSON data into Rust data structures largely
//! automatically, removing the need for all that boilerplate.
//!
//! e.g.:
//! ```rust
//! # use serde::{Deserialize, Serialize};
//! # use bson::{bson, Bson};
//! #[derive(Serialize, Deserialize)]
//! struct Person {
//!     name: String,
//!     age: i32,
//!     phones: Vec<String>,
//! }
//!
//! // Some BSON input data as a `Bson`.
//! let bson_data: Bson = bson!({
//!     "name": "John Doe",
//!     "age": 43,
//!     "phones": [
//!         "+44 1234567",
//!         "+44 2345678"
//!     ]
//! });
//!
//! // Deserialize the Person struct from the BSON data, automatically
//! // verifying that the necessary keys are present and that they are of
//! // the correct types.
//! let mut person: Person = bson::from_bson(bson_data).unwrap();
//!
//! // Do things just like with any other Rust data structure.
//! println!("Redacting {}'s record.", person.name);
//! person.name = "REDACTED".to_string();
//!
//! // Get a serialized version of the input data as a `Bson`.
//! let redacted_bson = bson::to_bson(&person).unwrap();
//! ```
//!
//! Any types that implement `Serialize` and `Deserialize` can be used in this way. Doing so helps
//! separate the "business logic" that operates over the data from the (de)serialization logic that
//! translates the data to/from its serialized form. This can lead to more clear and concise code
//! that is also less error prone.

#![allow(clippy::cognitive_complexity)]
#![doc(html_root_url = "https://docs.rs/bson/2.0.0-beta.2")]

pub use self::{
    bson::{Array, Binary, Bson, DbPointer, Document, JavaScriptCodeWithScope, Regex, Timestamp},
    datetime::DateTime,
    de::{
        from_bson,
        from_document,
        from_reader,
        from_reader_utf8_lossy,
        from_slice,
        from_slice_utf8_lossy,
        Deserializer,
    },
    decimal128::Decimal128,
    ser::{to_bson, to_document, to_vec, to_writer, Serializer},
};

#[macro_use]
mod macros;
mod bson;
mod datetime;
pub mod de;
pub mod decimal128;
pub mod document;
pub mod extjson;
pub mod oid;
pub mod ser;
pub mod serde_helpers;
pub mod spec;

#[cfg(test)]
mod tests;
