use serde::{self, Deserialize, Serialize};

use crate::{
    cstr,
    doc,
    oid::ObjectId,
    spec::BinarySubtype,
    Binary,
    Bson,
    DateTime,
    Decimal128,
    Document,
    JavaScriptCodeWithScope,
    Regex,
    Timestamp,
};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub(super) struct AllTypes {
    pub(super) x: i32,
    pub(super) y: i64,
    pub(super) s: String,
    pub(super) array: Vec<Bson>,
    pub(super) bson: Bson,
    pub(super) oid: ObjectId,
    pub(super) null: Option<()>,
    pub(super) subdoc: Document,
    pub(super) b: bool,
    pub(super) d: f64,
    pub(super) binary: Binary,
    pub(super) binary_old: Binary,
    pub(super) binary_other: Binary,
    pub(super) date: DateTime,
    pub(super) regex: Regex,
    pub(super) ts: Timestamp,
    pub(super) i: SubDoc,
    pub(super) undefined: Bson,
    pub(super) code: Bson,
    pub(super) code_w_scope: JavaScriptCodeWithScope,
    pub(super) decimal: Decimal128,
    pub(super) symbol: Bson,
    pub(super) min_key: Bson,
    pub(super) max_key: Bson,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub(super) struct SubDoc {
    pub(super) a: i32,
    pub(super) b: i32,
}

impl AllTypes {
    pub(super) fn fixtures() -> (Self, Document) {
        let binary = Binary {
            bytes: vec![36, 36, 36],
            subtype: BinarySubtype::Generic,
        };
        let binary_old = Binary {
            bytes: vec![36, 36, 36],
            subtype: BinarySubtype::BinaryOld,
        };
        let binary_other = Binary {
            bytes: vec![36, 36, 36],
            subtype: BinarySubtype::UserDefined(0x81),
        };
        let date = DateTime::now();
        let regex = Regex {
            pattern: cstr!("hello").into(),
            options: cstr!("x").into(),
        };
        let timestamp = Timestamp {
            time: 123,
            increment: 456,
        };
        let code = Bson::JavaScriptCode("console.log(1)".to_string());
        let code_w_scope = JavaScriptCodeWithScope {
            code: "console.log(a)".to_string(),
            scope: doc! { "a": 1 },
        };
        let oid = ObjectId::new();
        let subdoc = doc! { "k": true, "b": { "hello": "world" } };

        let decimal = {
            let bytes = hex::decode("18000000136400D0070000000000000000000000003A3000").unwrap();
            let d = Document::from_reader(bytes.as_slice()).unwrap();
            match d.get("d") {
                Some(Bson::Decimal128(d)) => *d,
                c => panic!("expected decimal128, got {:?}", c),
            }
        };

        let doc = doc! {
            "x": 1,
            "y": 2_i64,
            "s": "oke",
            "array": [ true, "oke", { "12": 24 } ],
            "bson": 1234.5,
            "oid": oid,
            "null": Bson::Null,
            "subdoc": subdoc.clone(),
            "b": true,
            "d": 12.5,
            "binary": binary.clone(),
            "binary_old": binary_old.clone(),
            "binary_other": binary_other.clone(),
            "date": date,
            "regex": regex.clone(),
            "ts": timestamp,
            "i": { "a": 300, "b": 12345 },
            "undefined": Bson::Undefined,
            "code": code.clone(),
            "code_w_scope": code_w_scope.clone(),
            "decimal": Bson::Decimal128(decimal),
            "symbol": Bson::Symbol("ok".to_string()),
            "min_key": Bson::MinKey,
            "max_key": Bson::MaxKey,
        };

        let v = AllTypes {
            x: 1,
            y: 2,
            s: "oke".to_string(),
            array: vec![
                Bson::Boolean(true),
                Bson::String("oke".to_string()),
                Bson::Document(doc! { "12": 24 }),
            ],
            bson: Bson::Double(1234.5),
            oid,
            null: None,
            subdoc,
            b: true,
            d: 12.5,
            binary,
            binary_old,
            binary_other,
            date,
            regex,
            ts: timestamp,
            i: SubDoc { a: 300, b: 12345 },
            undefined: Bson::Undefined,
            code,
            code_w_scope,
            decimal,
            symbol: Bson::Symbol("ok".to_string()),
            min_key: Bson::MinKey,
            max_key: Bson::MaxKey,
        };

        (v, doc)
    }
}
