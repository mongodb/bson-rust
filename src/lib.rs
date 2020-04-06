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
//! use bson::{decode_document, encode_document, Bson, Document};
//! use std::io::Cursor;
//!
//! let mut doc = Document::new();
//! doc.insert("foo".to_owned(), Bson::String("bar".to_owned()));
//!
//! let mut buf = Vec::new();
//! encode_document(&mut buf, &doc).unwrap();
//!
//! let doc = decode_document(&mut Cursor::new(&buf[..])).unwrap();
//! ```

#[cfg(feature = "decimal128")]
pub use self::decimal128::Decimal128;
pub use self::{
    bson::{
        Array,
        Binary,
        Bson,
        Document,
        JavaScriptCodeWithScope,
        Regex,
        TimeStamp,
        UtcDateTime,
    },
    decoder::{
        decode_document,
        decode_document_utf8_lossy,
        from_bson,
        Decoder,
        DecoderError,
        DecoderResult,
    },
    encoder::{encode_document, to_bson, Encoder, EncoderError, EncoderResult},
    ordered::{ValueAccessError, ValueAccessResult},
};

#[macro_use]
pub mod macros;
mod bson;
pub mod compat;
#[cfg(feature = "decimal128")]
pub mod decimal128;
mod decoder;
mod encoder;
pub mod oid;
pub mod ordered;
pub mod spec;
