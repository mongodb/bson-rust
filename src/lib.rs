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

//! BSON is a binary format in which zero or more key/value pairs are stored as a single entity.
//! We call this entity a document.
//!
//! This library supports version 1.0 of the [BSON standard](http://bsonspec.org/spec.html).
//!
//! ## Basic usage
//!
//! ```rust
//! extern crate bson;
//! use bson::{decode_document, encode_document, Bson, Document};
//! use std::io::Cursor;
//!
//! fn main() {
//!     let mut doc = Document::new();
//!     doc.insert("foo".to_owned(), Bson::String("bar".to_owned()));
//!
//!     let mut buf = Vec::new();
//!     encode_document(&mut buf, &doc).unwrap();
//!
//!     let doc = decode_document(&mut Cursor::new(&buf[..])).unwrap();
//! }
//! ```

extern crate byteorder;
extern crate chrono;
extern crate hex;
extern crate linked_hash_map;
extern crate rand;
extern crate serde;
#[macro_use]
extern crate serde_json;
extern crate md5;
extern crate time;
extern crate try_from;
extern crate decimal;

pub use self::bson::{Array, Bson, Document, TimeStamp, UtcDateTime};
pub use self::decoder::{decode_document, decode_document_utf8_lossy, from_bson, Decoder, DecoderError, DecoderResult};
pub use self::encoder::{encode_document, to_bson, Encoder, EncoderError, EncoderResult};
pub use self::ordered::{ValueAccessError, ValueAccessResult};
pub use self::decimal128::Decimal128;

#[macro_use]
pub mod macros;
mod bson;
pub mod compat;
mod decoder;
mod encoder;
pub mod oid;
pub mod ordered;
pub mod spec;
pub mod decimal128;
