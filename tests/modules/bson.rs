use bson::{
    doc,
    oid::ObjectId,
    spec::BinarySubtype,
    Binary,
    Bson,
    Document,
    JavaScriptCodeWithScope,
    Regex,
};
use serde_json::{json, Value};

#[test]
fn to_json() {
    let mut doc = Document::new();
    doc.insert(
        "_id",
        Bson::ObjectId(ObjectId::with_bytes(*b"abcdefghijkl")),
    );
    doc.insert("first", Bson::Int32(1));
    doc.insert("second", Bson::String("foo".to_owned()));
    doc.insert("alphanumeric", Bson::String("bar".to_owned()));
    let data: Value = Bson::Document(doc).into();

    assert!(data.is_object());
    let obj = data.as_object().unwrap();

    let id = obj.get("_id").unwrap();
    assert!(id.is_object());
    let id_val = id.get("$oid").unwrap();
    assert!(id_val.is_string());
    assert_eq!(id_val, "6162636465666768696a6b6c");

    let first = obj.get("first").unwrap();
    assert!(first.is_number());
    assert_eq!(first.as_i64().unwrap(), 1);

    let second = obj.get("second").unwrap();
    assert!(second.is_string());
    assert_eq!(second.as_str().unwrap(), "foo");

    let alphanumeric = obj.get("alphanumeric").unwrap();
    assert!(alphanumeric.is_string());
    assert_eq!(alphanumeric.as_str().unwrap(), "bar");
}

#[test]
fn bson_default() {
    let bson1 = Bson::default();
    assert_eq!(bson1, Bson::Null);
}

#[test]
fn document_default() {
    let doc1 = Document::default();
    assert_eq!(doc1.keys().count(), 0);
    assert_eq!(doc1, Document::new());
}

#[test]
fn from_impls() {
    assert_eq!(Bson::from(1.5f32), Bson::Double(1.5));
    assert_eq!(Bson::from(2.25f64), Bson::Double(2.25));
    assert_eq!(Bson::from("data"), Bson::String(String::from("data")));
    assert_eq!(
        Bson::from(String::from("data")),
        Bson::String(String::from("data"))
    );
    assert_eq!(Bson::from(doc! {}), Bson::Document(Document::new()));
    assert_eq!(Bson::from(false), Bson::Boolean(false));
    assert_eq!(
        Bson::from(Regex {
            pattern: String::from("\\s+$"),
            options: String::from("i")
        }),
        Bson::RegularExpression(Regex {
            pattern: String::from("\\s+$"),
            options: String::from("i")
        })
    );
    assert_eq!(
        Bson::from(JavaScriptCodeWithScope {
            code: String::from("alert(\"hi\");"),
            scope: doc! {}
        }),
        Bson::JavaScriptCodeWithScope(JavaScriptCodeWithScope {
            code: String::from("alert(\"hi\");"),
            scope: doc! {}
        })
    );
    //
    assert_eq!(
        Bson::from(Binary {
            subtype: BinarySubtype::Generic,
            bytes: vec![1, 2, 3]
        }),
        Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: vec![1, 2, 3]
        })
    );
    assert_eq!(Bson::from(-48i32), Bson::Int32(-48));
    assert_eq!(Bson::from(-96i64), Bson::Int64(-96));
    assert_eq!(Bson::from(152u32), Bson::Int32(152));
    assert_eq!(Bson::from(4096u64), Bson::Int64(4096));

    let oid = ObjectId::new();
    assert_eq!(
        Bson::from(b"abcdefghijkl"),
        Bson::ObjectId(ObjectId::with_bytes(*b"abcdefghijkl"))
    );
    assert_eq!(Bson::from(oid.clone()), Bson::ObjectId(oid.clone()));
    assert_eq!(
        Bson::from(vec![1, 2, 3]),
        Bson::Array(vec![Bson::Int32(1), Bson::Int32(2), Bson::Int32(3)])
    );
    assert_eq!(
        Bson::from(json!({"_id": {"$oid": oid.to_hex()}, "name": ["bson-rs"]})),
        Bson::Document(doc! {"_id": &oid, "name": ["bson-rs"]})
    );

    // References
    assert_eq!(Bson::from(&24i32), Bson::Int32(24));
    assert_eq!(
        Bson::from(&String::from("data")),
        Bson::String(String::from("data"))
    );
    assert_eq!(Bson::from(&oid), Bson::ObjectId(oid));
    assert_eq!(
        Bson::from(&doc! {"a": "b"}),
        Bson::Document(doc! {"a": "b"})
    );

    let db_pointer = Bson::from(json!({
        "$dbPointer": {
            "$ref": "db.coll",
            "$id": { "$oid": "507f1f77bcf86cd799439011" },
        }
    }));
    let db_pointer = db_pointer.as_db_pointer().unwrap();
    assert_eq!(Bson::from(db_pointer), Bson::DbPointer(db_pointer.clone()));
}
