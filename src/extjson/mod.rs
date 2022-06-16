//! Deserialization and serialization of [MongoDB Extended JSON v2](https://www.mongodb.com/docs/manual/reference/mongodb-extended-json/)
//!
//! ## Overview of Extended JSON
//!
//! MongoDB Extended JSON (abbreviated extJSON) is format of JSON that allows for the encoding of
//! BSON type information. Normal JSON cannot unambiguously represent all BSON types losslessly, so
//! an extension was designed to include conventions for representing those types.
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
//!     object
//! notation.
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
//! ## Deserializing Extended JSON
//!
//! Extended JSON can be deserialized using [`Bson`](../enum.Bson.html)'s
//! `TryFrom<serde_json::Value>` implementation. This implementation accepts both canonical and
//! relaxed extJSON, and the two modes can even be mixed within a single representation.
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
//! ## Serializing to Extended JSON
//!
//! Extended JSON can be created via [`Bson`](../enum.Bson.html)'s `Into<serde_json::Value>`
//! implementation (which will create relaxed extJSON),
//! [`Bson::into_relaxed_extjson`](../enum.Bson.html#method.into_relaxed_extjson), and
//! [`Bson::into_canonical_extjson`](../enum.Bson.html#method.into_canonical_extjson).
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

pub mod de;
pub(crate) mod models;
