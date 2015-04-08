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
use rustc_serialize::json;
use rustc_serialize::hex::ToHex;

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

impl Bson {
    pub fn to_json(&self) -> json::Json {
        match self {
            &Bson::FloatingPoint(v) => json::Json::F64(v),
            &Bson::String(ref v) => json::Json::String(v.clone()),
            &Bson::Array(ref v) =>
                json::Json::Array(v.iter().map(|x| x.to_json()).collect()),
            &Bson::Document(ref v) =>
                json::Json::Object(v.iter().map(|(k, v)| (k.clone(), v.to_json())).collect()),
            &Bson::Boolean(v) => json::Json::Boolean(v),
            &Bson::Null => json::Json::Null,
            &Bson::RegExp(ref pat, ref opt) => {
                let mut re = json::Object::new();
                re.insert("pattern".to_string(), json::Json::String(pat.clone()));
                re.insert("options".to_string(), json::Json::String(opt.clone()));

                json::Json::Object(re)
            },
            &Bson::JavaScriptCode(ref code) => json::Json::String(code.clone()),
            &Bson::JavaScriptCodeWithScope(ref code, ref scope) => {
                let mut obj = json::Object::new();
                obj.insert("code".to_string(), json::Json::String(code.clone()));

                let scope_obj =
                    scope.iter().map(|(k, v)| (k.clone(), v.to_json())).collect();

                obj.insert("scope".to_string(), json::Json::Object(scope_obj));

                json::Json::Object(obj)
            },
            &Bson::Deprecated => json::Json::String("deprecated".to_string()),
            &Bson::I32(v) => json::Json::I64(v as i64),
            &Bson::I64(v) => json::Json::I64(v),
            &Bson::TimeStamp(v) => json::Json::I64(v),
            &Bson::Binary(t, ref v) => {
                let mut obj = json::Object::new();
                let tval: u8 = From::from(t);
                obj.insert("type".to_string(), json::Json::I64(tval as i64));
                obj.insert("data".to_string(), json::Json::String(v[..].to_hex()));

                json::Json::Object(obj)
            },
            &Bson::ObjectId(v) => json::Json::String(v.to_hex()),
            &Bson::UtcDatetime(ref v) => json::Json::String(v.to_string()),
        }
    }

    pub fn from_json(j: &json::Json) -> Bson {
        match j {
            &json::Json::I64(x) => Bson::I64(x),
            &json::Json::U64(x) => Bson::I64(x as i64),
            &json::Json::F64(x) => Bson::FloatingPoint(x),
            &json::Json::String(ref x) => Bson::String(x.clone()),
            &json::Json::Boolean(x) => Bson::Boolean(x),
            &json::Json::Array(ref x) => Bson::Array(x.iter().map(|x| Bson::from_json(x)).collect()),
            &json::Json::Object(ref x) => Bson::Document(x.iter().map(|(k, v)| (k.clone(), Bson::from_json(v))).collect()),
            &json::Json::Null => Bson::Null,
        }
    }
}
