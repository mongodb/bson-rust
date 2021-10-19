use crate::{spec::BinarySubtype, uuid::Uuid, Binary, Bson, Document};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct U {
    uuid: Uuid,
}

#[test]
fn into_bson() {
    let uuid = Uuid::new();

    let bson: Bson = uuid.into();
    let binary = Binary {
        bytes: uuid.as_bytes().to_vec(),
        subtype: BinarySubtype::Uuid,
    };

    assert_eq!(bson, Bson::Binary(binary.clone()));
}

#[test]
fn raw_serialization() {
    let u = U { uuid: Uuid::new() };
    let bytes = crate::to_vec(&u).unwrap();

    let doc: Document = crate::from_slice(bytes.as_slice()).unwrap();
    assert_eq!(doc, doc! { "uuid": u.uuid });

    let u_roundtrip: U = crate::from_slice(bytes.as_slice()).unwrap();
    assert_eq!(u_roundtrip, u);
}

#[test]
fn bson_serialization() {
    let u = U { uuid: Uuid::new() };
    let doc = crate::to_document(&u).unwrap();

    assert_eq!(doc, doc! { "uuid": u.uuid });

    let u_roundtrip: U = crate::from_document(doc).unwrap();
    assert_eq!(u_roundtrip, u);
}

#[test]
fn json() {
    let u = U { uuid: Uuid::new() };

    let json = serde_json::to_value(&u).unwrap();
    assert_eq!(json, json!({ "uuid": u.uuid.to_string() }));

    let u_roundtrip_json: U = serde_json::from_value(json).unwrap();
    assert_eq!(u_roundtrip_json, u);
}
