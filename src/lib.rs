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

#![doc = include_str!("../README.md")]
#![allow(clippy::cognitive_complexity, clippy::derive_partial_eq_without_eq)]
#![doc(html_root_url = "https://docs.rs/bson/2.6.0")]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[doc(inline)]
pub use self::{
    binary::Binary,
    bson::{Array, Bson, DbPointer, Document, JavaScriptCodeWithScope, Regex, Timestamp},
    datetime::DateTime,
    de::{
        from_bson,
        from_bson_with_options,
        from_document,
        from_document_with_options,
        from_reader,
        from_reader_utf8_lossy,
        from_slice,
        from_slice_utf8_lossy,
        Deserializer,
        DeserializerOptions,
    },
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
    ser::{
        to_bson,
        to_bson_with_options,
        to_document,
        to_document_with_options,
        to_raw_document_buf,
        to_vec,
        Serializer,
        SerializerOptions,
    },
    uuid::{Uuid, UuidRepresentation},
};

#[macro_use]
mod macros;
pub mod binary;
mod bson;
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
