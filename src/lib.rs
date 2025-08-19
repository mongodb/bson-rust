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
//! - Rust 1.64+
//!
//! ### Importing
//! This crate is available on [crates.io](https://crates.io/crates/bson). To use it in your application,
//! simply add it to your project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! bson = "3.0.0"
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
//! | `jiff-0_2` | Enable support for v0.2 of the [`jiff`](https://docs.rs/jiff/0.2) crate in the public API.             | no      |
//! | `uuid-1`     | Enable support for v1.x of the [`uuid`](https://docs.rs/uuid/1.x) crate in the public API.           | no      |
//! | `time-0_3`   | Enable support for v0.3 of the [`time`](https://docs.rs/time/0.3) crate in the public API.           | no      |
//! | `serde`      | Enable integration with the [`serde`](https://docs.rs/serde/) serialization/deserialization framework.  | no      |
//! | `serde_with-3` | Enable [`serde_with`](https://docs.rs/serde_with/3.x) type conversion utilities in the public API. | no      |
//! | `serde_path_to_error` | Enable support for error paths via integration with [`serde_path_to_error`](https://docs.rs/serde_path_to_err/latest).  This is an unstable feature and any breaking changes to `serde_path_to_error` may affect usage of it via this feature. | no |
//! | `compat-3-0-0` | Required for future compatibility if default features are disabled. | yes |
//! | `large_dates` | Increase the supported year range for some `bson::DateTime` utilities from +/-9,999 (inclusive) to +/-999,999 (inclusive). Note that enabling this feature can impact performance and introduce parsing ambiguities. | no |
//! | `serde_json-1` | Enable support for v1.x of the [`serde_json`](https://docs.rs/serde_json/1.x) crate in the public API. | no |
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
//! ## Integration with `serde`
//!
//! While it is possible to work with documents and BSON values directly, it will often introduce a
//! lot of boilerplate for verifying the necessary keys are present and their values are the correct
//! types. Enabling the `serde` feature provides integration with the [`serde`](https://serde.rs/)
//! crate that maps BSON data into Rust data structs largely automatically, removing the need for
//! all that boilerplate.
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
//! let mut person: Person = bson::deserialize_from_bson(bson_data).unwrap();
//!
//! // Do things just like with any other Rust data structure.
//! println!("Redacting {}'s record.", person.name);
//! person.name = "REDACTED".to_string();
//!
//! // Get a serialized version of the input data as a [`Bson`].
//! let redacted_bson = bson::serialize_to_bson(&person).unwrap();
//! ```
//!
//! Any types that implement [`Serialize`](serde::Serialize) and [`Deserialize`](serde::Deserialize)
//! can be used in this way. Doing so helps separate the "business logic" that operates over the
//! data from the (de)serialization logic that translates the data to/from its serialized form. This
//! can lead to more clear and concise code that is also less error prone.
//!
//! When serializing values that cannot be represented in BSON, or deserialzing from BSON that does
//! not match the format expected by the type, the default error will only report the specific field
//! that failed. To aid debugging, enabling the `serde_path_to_error` feature will
//! [augment errors](crate::error::Error::path) with the full field path from root object to
//! failing field.  This feature does incur a small CPU and memory overhead during (de)serialization
//! and should be enabled with care in performance-sensitive environments.
//!
//! ### Embedding BSON Value Types
//!
//! The `serde` feature also enables implementations of [`Serialize`](serde::Serialize) and
//! [`Deserialize`](serde::Deserialize) for the Rust types provided by this crate that represent
//! BSON values, allowing them to be embedded in domain-specific structs as appropriate:
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use bson::{bson, Bson, oid::ObjectId};
//!
//! #[derive(Serialize, Deserialize)]
//! struct Person {
//!     id: ObjectId,
//!     name: String,
//!     age: i32,
//!     phones: Vec<String>,
//! }
//!
//! let bson_data: Bson = bson!({
//!     "id": ObjectId::new(),
//!     "name": "John Doe",
//!     "age": 43,
//!     "phones": [
//!         "+44 1234567",
//!         "+44 2345678"
//!     ]
//! });
//!
//! let person: Person = bson::deserialize_from_bson(bson_data).unwrap();
//! ```
//!
//! ### Encoding vs. Serialization
//!
//! With the `serde` feature enabled, a BSON document can be converted to its wire-format byte
//! representation in multiple ways:
//! ```rust
//! # fn wrapper() -> bson::error::Result<()> {
//! use bson::{doc, serialize_to_vec};
//! let my_document = doc! { "hello": "bson" };
//! let encoded = my_document.to_vec()?;
//! let serialized = serialize_to_vec(&my_document)?;
//! # Ok(())
//! # }
//! # wrapper().unwrap();
//! ```
//!
//! We recommend that, where possible, documents be converted to byte form using the encoding
//! methods ([`Document::to_vec`]/[`Document::to_writer`]); this is more efficient as it avoids
//! the intermediate `serde` data model representation.  This also applies to decoding; prefer
//! [`Document::from_reader`] over [`deserialize_from_reader`] / [`deserialize_from_slice`].
//!
//! ### Serializer Compatibility
//!
//! The implementations of [`Serialize`](serde::Serialize) and [`Deserialize`](serde::Deserialize)
//! for BSON value types are tested with the `serde` \[de\]serializers provided by this crate and by
//! the `serde_json` crate.  Compatibility with formats provided by other crates is not guaranteed
//! and the data produced by serializing BSON values to other formats may change when this crate is
//! updated.
//!
//! ## Working with Extended JSON
//!
//! MongoDB Extended JSON (extJSON) is a format of JSON that allows for the encoding
//! of BSON type information. Normal JSON cannot unambiguously represent all BSON types losslessly,
//! so an extension was designed to include conventions for representing those types.
//!
//! For example, a BSON binary is represented by the following format:
//! ```text
//! {
//!    "$binary": {
//!        "base64": <base64 encoded payload as a string>,
//!        "subType": <subtype as a one or two character hex string>,
//!    }
//! }
//! ```
//! For more information on extJSON and the complete list of translations, see the [official MongoDB documentation](https://www.mongodb.com/docs/manual/reference/mongodb-extended-json/).
//!
//! All MongoDB drivers and BSON libraries interpret and produce extJSON, so it can serve as a
//! useful tool for communicating between applications where raw BSON bytes cannot be used (e.g. via
//! JSON REST APIs). It's also useful for representing BSON data as a string.
//!
//! ### Canonical and Relaxed Modes
//!
//! There are two modes of extJSON: "Canonical" and "Relaxed". They are the same except for the
//! following differences:
//!   - In relaxed mode, all BSON numbers are represented by the JSON number type, rather than the
//!     object notation.
//!   - In relaxed mode, the string in the datetime object notation is RFC 3339 (ISO-8601) formatted
//!     (if the date is after 1970).
//!
//! e.g.
//! ```rust
//! # use bson::bson;
//! let doc = bson!({ "x": 5, "d": bson::DateTime::now() });
//!
//! println!("relaxed: {}", doc.clone().into_relaxed_extjson());
//! // relaxed: "{"x":5,"d":{"$date":"2020-06-01T22:19:13.075Z"}}"
//!
//! println!("canonical: {}", doc.into_canonical_extjson());
//! // canonical: {"x":{"$numberInt":"5"},"d":{"$date":{"$numberLong":"1591050020711"}}}
//! ```
//!
//! Canonical mode is useful when BSON values need to be round tripped without losing any type
//! information. Relaxed mode is more useful when debugging or logging BSON data.
//!
//! ### Deserializing from Extended JSON
//!
//! Extended JSON can be deserialized into a [`Bson`] value using the
//! [`TryFrom`](https://docs.rs/bson/latest/bson/enum.Bson.html#impl-TryFrom%3CValue%3E-for-Bson)
//! implementation for [`serde_json::Value`]. This implementation accepts both canonical and relaxed
//! extJSON, and the two modes can be mixed within a single representation.
//!
//! e.g.
//! ```rust
//! # use bson::Bson;
//! # use serde_json::json;
//! # use std::convert::{TryFrom, TryInto};
//! let json_doc = json!({ "x": 5i32, "y": { "$numberInt": "5" }, "z": { "subdoc": "hello" } });
//! let bson: Bson = json_doc.try_into().unwrap(); // Bson::Document(...)
//!
//! let json_date = json!({ "$date": { "$numberLong": "1590972160292" } });
//! let bson_date: Bson = json_date.try_into().unwrap(); // Bson::DateTime(...)
//!
//! let invalid_ext_json = json!({ "$numberLong": 5 });
//! Bson::try_from(invalid_ext_json).expect_err("5 should be a string");
//! ```
//!
//! ### Serializing to Extended JSON
//!
//! A [`Bson`] value can be serialized into extJSON using the [`Bson::into_relaxed_extjson`] and
//! [`Bson::into_canonical_extjson`] methods. The `Into<serde_json::Value>` implementation for
//! [`Bson`] produces relaxed extJSON.
//!
//! e.g.
//! ```rust
//! # use bson::{bson, oid};
//! let doc = bson!({ "x": 5i32, "_id": oid::ObjectId::new() });
//!
//! let relaxed_extjson: serde_json::Value = doc.clone().into();
//! println!("{}", relaxed_extjson); // { "x": 5, "_id": { "$oid": <hexstring> } }
//!
//! let relaxed_extjson = doc.clone().into_relaxed_extjson();
//! println!("{}", relaxed_extjson); // { "x": 5, "_id": { "$oid": <hexstring> } }
//!
//! let canonical_extjson = doc.into_canonical_extjson();
//! println!("{}", canonical_extjson); // { "x": { "$numberInt": "5" }, "_id": { "$oid": <hexstring> } }
//! ```
//!
//! ## Working with datetimes
//!
//! The BSON format includes a datetime type, which is modeled in this crate by the
//! [`DateTime`] struct, and the
//! [`Serialize`](serde::Serialize) and [`Deserialize`](serde::Deserialize) implementations for this
//! struct produce and parse BSON datetimes when serializing to or deserializing from BSON. The
//! popular crate [`chrono`](docs.rs/chrono) also provides a [`DateTime`] type, but its
//! [`Serialize`](serde::Serialize) and [`Deserialize`](serde::Deserialize) implementations operate
//! on strings instead, so when using it with BSON, the BSON datetime type is not used. To work
//! around this, the `chrono-0_4` feature flag can be enabled. This flag exposes a number of
//! convenient conversions between [`bson::DateTime`](crate::DateTime) and [`chrono::DateTime`],
//! including the [`serde_helpers::datetime::FromChrono04DateTime`]
//! serde helper, which can be used to (de)serialize [`chrono::DateTime`]s to/from BSON datetimes,
//! and the `From<chrono::DateTime>` implementation for [`Bson`], which allows [`chrono::DateTime`]
//! values to be used in the `doc!` and `bson!` macros.
//!
//! e.g.
//! ``` rust
//! # #[cfg(feature = "chrono-0_4")]
//! # {
//! use serde::{Serialize, Deserialize};
//! use serde_with::serde_as;
//! use bson::doc;
//! use bson::serde_helpers::datetime;
//!
//! #[serde_as]
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
//!     #[serde_as(as = "datetime::FromChrono04DateTime")]
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
//! ## WASM support
//!
//! This crate compiles to the `wasm32-unknown-unknown` target; when doing so, the `js-sys` crate is
//! used for the current timestamp component of `ObjectId` generation.
//!
//! ## Minimum supported Rust version (MSRV)
//!
//! The MSRV for this crate is currently 1.81. This will be rarely be increased, and if it ever
//! is, it will only happen in a minor or major version release.

#![allow(clippy::cognitive_complexity, clippy::derive_partial_eq_without_eq)]
#![doc(html_root_url = "https://docs.rs/bson/3.0.0")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(missing_docs)]

#[doc(inline)]
pub use self::{
    binary::Binary,
    bson::{Array, Bson, DbPointer, Document, JavaScriptCodeWithScope, Regex, Timestamp},
    datetime::DateTime,
    decimal128::Decimal128,
    raw::{
        RawArray,
        RawArrayBuf,
        RawBinaryRef,
        RawBson,
        RawBsonRef,
        RawDbPointerRef,
        RawDocument,
        RawDocumentBuf,
        RawJavaScriptCodeWithScope,
        RawJavaScriptCodeWithScopeRef,
        RawRegexRef,
    },
    utf8_lossy::Utf8Lossy,
    uuid::{Uuid, UuidRepresentation},
};

#[cfg(feature = "serde")]
#[doc(inline)]
pub use self::{
    de::{
        deserialize_from_bson,
        deserialize_from_document,
        deserialize_from_reader,
        deserialize_from_slice,
        raw::RawDeserializer,
        Deserializer,
    },
    ser::{
        serialize_to_bson,
        serialize_to_buffer,
        serialize_to_document,
        serialize_to_raw_document_buf,
        serialize_to_vec,
        Serializer,
    },
};

#[macro_use]
mod macros;
mod base64;
pub mod binary;
mod bson;
pub mod datetime;
#[cfg(feature = "serde")]
pub mod de;
pub mod decimal128;
pub mod document;
pub mod error;
#[cfg(feature = "serde")]
mod extjson;
pub mod oid;
pub mod raw;
#[cfg(feature = "serde")]
pub mod ser;
#[cfg(feature = "serde")]
pub mod serde_helpers;
pub mod spec;
mod utf8_lossy;
pub mod uuid;

#[cfg(test)]
mod tests;

#[cfg(not(feature = "compat-3-0-0"))]
compile_error!(
    "The feature 'compat-3-0-0' must be enabled to ensure forward compatibility with future \
     versions of this crate."
);
