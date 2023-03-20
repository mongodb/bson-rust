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
//! ## Installation
//! ### Requirements
//! - Rust 1.56+
//!
//! ### Importing
//! This crate is available on [crates.io](https://crates.io/crates/bson). To use it in your application,
//! simply add it to your project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! bson = "2.6.0"
//! ```
//!
//! Note that if you are using `bson` through the `mongodb` crate, you do not need to specify it in
//! your `Cargo.toml`, since the `mongodb` crate already re-exports it.
//!
//! #### Feature Flags
//!
//! | Feature      | Description                                                                                          | Default |
//! |:-------------|:-----------------------------------------------------------------------------------------------------|:--------|
//! | `chrono-0_4` | Enable support for v0.4 of the [`chrono`](https://docs.rs/chrono/0.4) crate in the public API.       | no      |
//! | `uuid-0_8`   | Enable support for v0.8 of the [`uuid`](https://docs.rs/uuid/0.8) crate in the public API.           | no      |
//! | `uuid-1`     | Enable support for v1.x of the [`uuid`](https://docs.rs/uuid/1.x) crate in the public API.           | no      |
//! | `time-0_3`   | Enable support for v0.3 of the [`time`](https://docs.rs/time/0.3) crate in the public API.           | no      |
//! | `serde_with` | Enable [`serde_with`](https://docs.rs/serde_with/latest) integrations for [`DateTime`] and [`Uuid`]. | no      |
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
//! use bson::{bson, Bson};
//!
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
//! use bson::{bson, Bson};
//!
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
//! use bson::{doc, Document};
//! use std::io::Read;
//!
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
//! use bson::doc;
//!
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
//! use serde::{Deserialize, Serialize};
//! use bson::{bson, Bson};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Person {
//!     name: String,
//!     age: i32,
//!     phones: Vec<String>,
//! }
//!
//! // Some BSON input data as a [`Bson`].
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
//! // Get a serialized version of the input data as a [`Bson`].
//! let redacted_bson = bson::to_bson(&person).unwrap();
//! ```
//!
//! Any types that implement [`Serialize`](serde::Serialize) and [`Deserialize`](serde::Deserialize) can be used in this way. Doing so helps
//! separate the "business logic" that operates over the data from the (de)serialization logic that
//! translates the data to/from its serialized form. This can lead to more clear and concise code
//! that is also less error prone.
//!
//! ## Working with datetimes
//!
//! The BSON format includes a datetime type, which is modeled in this crate by the
//! [`DateTime`] struct, and the
//! [`Serialize`](serde::Serialize) and [`Deserialize`](serde::Deserialize) implementations for this struct produce and parse BSON datetimes
//! when serializing to or deserializing from BSON. The popular crate [`chrono`](docs.rs/chrono)
//! also provides a [`DateTime`] type, but its [`Serialize`](serde::Serialize) and [`Deserialize`](serde::Deserialize) implementations operate
//! on strings instead, so when using it with BSON, the BSON datetime type is not used. To work
//! around this, the `chrono-0_4` feature flag can be enabled. This flag exposes a number of
//! convenient conversions between [`bson::DateTime`](crate::DateTime) and [`chrono::DateTime`], including the
//! [`serde_helpers::chrono_datetime_as_bson_datetime`]
//! serde helper, which can be used to (de)serialize [`chrono::DateTime`]s to/from BSON datetimes, and
//! the `From<chrono::DateTime>` implementation for [`Bson`], which allows [`chrono::DateTime`] values
//! to be used in the `doc!` and `bson!` macros.
//!
//! e.g.
//! ``` rust
//! # #[cfg(feature = "chrono-0_4")]
//! # {
//! use serde::{Serialize, Deserialize};
//! use bson::doc;
//!
//! #[derive(Serialize, Deserialize)]
//! struct Foo {
//!     // serializes as a BSON datetime.
//!     date_time: bson::DateTime,
//!
//!     // serializes as an RFC 3339 / ISO-8601 string.
//!     chrono_datetime: chrono::DateTime<chrono::Utc>,
//!
//!     // serializes as a BSON datetime.
//!     // this requires the "chrono-0_4" feature flag
//!     #[serde(with = "bson::serde_helpers::chrono_datetime_as_bson_datetime")]
//!     chrono_as_bson: chrono::DateTime<chrono::Utc>,
//! }
//!
//! // this automatic conversion also requires the "chrono-0_4" feature flag
//! let query = doc! {
//!     "created_at": chrono::Utc::now(),
//! };
//! # }
//! ```
//!
//! ## Working with UUIDs
//!
//! See the module level documentation for the [`uuid`] module.
//!
//! ## Minimum supported Rust version (MSRV)
//!
//! The MSRV for this crate is currently 1.56.0. This will be rarely be increased, and if it ever is,
//! it will only happen in a minor or major version release.

#![allow(clippy::cognitive_complexity, clippy::derive_partial_eq_without_eq)]
#![doc(html_root_url = "https://docs.rs/bson/2.6.1")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[doc(inline)]
pub use self::{
    bson::{Array, Bson, DbPointer, Document, JavaScriptCodeWithScope, Regex, Timestamp},
    datetime::DateTime,
    binary::Binary,
    de::{
        from_bson, from_bson_with_options, from_document, from_document_with_options, from_reader,
        from_reader_utf8_lossy, from_slice, from_slice_utf8_lossy, Deserializer,
        DeserializerOptions,
    },
    decimal128::Decimal128,
    raw::{
        RawArray, RawArrayBuf, RawBinaryRef, RawBson, RawBsonRef, RawDbPointerRef, RawDocument,
        RawDocumentBuf, RawJavaScriptCodeWithScope, RawJavaScriptCodeWithScopeRef, RawRegexRef,
    },
    ser::{
        to_bson, to_bson_with_options, to_document, to_document_with_options, to_raw_document_buf,
        to_vec, Serializer, SerializerOptions,
    },
    uuid::{Uuid, UuidRepresentation},
};

#[macro_use]
mod macros;
mod bson;
pub mod binary;
pub mod datetime;
pub mod de;
pub mod decimal128;
pub mod document;
pub mod extjson;
pub mod oid;
pub mod raw;
pub mod ser;
pub mod serde_helpers;
pub mod spec;
pub mod uuid;

#[cfg(test)]
mod tests;
