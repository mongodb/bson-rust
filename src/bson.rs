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

use std::collections::BTreeMap;
use std::string;

use chrono::{DateTime, UTC};

use spec::BinarySubtype;

#[derive(Debug, Clone)]
pub enum Bson {
    FloatingPoint(f64),
    String(String),
    Array(self::Array),
    Document(self::Document),
    Boolean(bool),
    Null,
    RegExp(string::String, string::String),
    JavaScriptCode(string::String),
    JavaScriptCodeWithScope(string::String, self::Document),
    Deprecated,
    I32(i32),
    I64(i64),
    TimeStamp(i64),
    Binary(BinarySubtype, Vec<u8>),
    ObjectId([u8; 12]),
    UtcDatetime(DateTime<UTC>),
}

pub type Array = Vec<Bson>;
pub type Document = BTreeMap<String, Bson>;
