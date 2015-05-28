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
//! This library supports Version 1.0 of BSON standard.
//!
//! ## Basic usage
//!
//! ```rust
//! extern crate bson;
//! use std::io::Cursor;
//! use bson::{Bson, Document, encode_document, decode_document};
//!
//! fn main() {
//!     let mut doc = Document::new();
//!     doc.insert("foo".to_owned(), Bson::String("bar".to_owned()));
//!
//!     let mut buf = Vec::new();
//!     encode_document(&mut buf, &doc).unwrap();
//!
//!     let mut r = Cursor::new(&buf[..]);
//!     let doc = decode_document(&mut Cursor::new(&buf[..]));
//! }
//! ```

extern crate rustc_serialize;
extern crate chrono;
extern crate byteorder;

pub use self::bson::{Bson, ToBson, Document, Array};
pub use self::encoder::{encode_document, EncoderResult, EncoderError};
pub use self::decoder::{decode_document, DecoderResult, DecoderError};

mod bson;
pub mod spec;

mod encoder;
mod decoder;

