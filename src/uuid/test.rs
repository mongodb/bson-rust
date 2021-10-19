use crate::{
    spec::BinarySubtype,
    uuid::{Uuid, UuidRepresentation},
    Binary,
    Bson,
    Document,
};
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
        bytes: uuid.bytes().to_vec(),
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

#[test]
fn test_binary_constructors() {
    let uuid = crate::Uuid::parse_str("00112233445566778899AABBCCDDEEFF").unwrap();
    let bin = Binary::from_uuid(uuid);
    assert_eq!(bin.bytes, uuid.bytes());
    assert_eq!(bin.subtype, BinarySubtype::Uuid);

    let bin = Binary::from_uuid_with_representation(uuid, UuidRepresentation::Standard);
    assert_eq!(bin.bytes, uuid.bytes());
    assert_eq!(bin.subtype, BinarySubtype::Uuid);

    let bin = Binary::from_uuid_with_representation(uuid, UuidRepresentation::JavaLegacy);
    assert_eq!(
        bin.bytes,
        Uuid::parse_str("7766554433221100FFEEDDCCBBAA9988")
            .unwrap()
            .bytes()
    );
    assert_eq!(bin.subtype, BinarySubtype::UuidOld);

    let bin = Binary::from_uuid_with_representation(uuid, UuidRepresentation::CSharpLegacy);
    assert_eq!(
        bin.bytes,
        Uuid::parse_str("33221100554477668899AABBCCDDEEFF")
            .unwrap()
            .bytes()
    );
    assert_eq!(bin.subtype, BinarySubtype::UuidOld);

    // Same byte ordering as standard representation
    let bin = Binary::from_uuid_with_representation(uuid, UuidRepresentation::PythonLegacy);
    assert_eq!(
        bin.bytes,
        Uuid::parse_str("00112233445566778899AABBCCDDEEFF")
            .unwrap()
            .bytes()
    );
    assert_eq!(bin.subtype, BinarySubtype::UuidOld);
}

#[test]
fn test_binary_to_uuid_standard_rep() {
    let uuid = crate::Uuid::parse_str("00112233445566778899AABBCCDDEEFF").unwrap();
    let bin = Binary::from_uuid(uuid);

    assert_eq!(bin.to_uuid().unwrap(), uuid);
    assert_eq!(
        bin.to_uuid_with_representation(UuidRepresentation::Standard)
            .unwrap(),
        uuid
    );

    assert!(bin
        .to_uuid_with_representation(UuidRepresentation::CSharpLegacy)
        .is_err());
    assert!(bin
        .to_uuid_with_representation(UuidRepresentation::PythonLegacy)
        .is_err());
    assert!(bin
        .to_uuid_with_representation(UuidRepresentation::PythonLegacy)
        .is_err());
}

#[test]
fn test_binary_to_uuid_explicitly_standard_rep() {
    let uuid = crate::Uuid::parse_str("00112233445566778899AABBCCDDEEFF").unwrap();
    let bin = Binary::from_uuid_with_representation(uuid, UuidRepresentation::Standard);

    assert_eq!(bin.to_uuid().unwrap(), uuid);
    assert_eq!(
        bin.to_uuid_with_representation(UuidRepresentation::Standard)
            .unwrap(),
        uuid
    );

    assert!(bin
        .to_uuid_with_representation(UuidRepresentation::CSharpLegacy)
        .is_err());
    assert!(bin
        .to_uuid_with_representation(UuidRepresentation::PythonLegacy)
        .is_err());
    assert!(bin
        .to_uuid_with_representation(UuidRepresentation::PythonLegacy)
        .is_err());
}

#[test]
fn test_binary_to_uuid_java_rep() {
    let uuid = crate::Uuid::parse_str("00112233445566778899AABBCCDDEEFF").unwrap();
    let bin = Binary::from_uuid_with_representation(uuid, UuidRepresentation::JavaLegacy);

    assert!(bin.to_uuid().is_err());
    assert!(bin
        .to_uuid_with_representation(UuidRepresentation::Standard)
        .is_err());

    assert_eq!(
        bin.to_uuid_with_representation(UuidRepresentation::JavaLegacy)
            .unwrap(),
        uuid
    );
}

#[test]
fn test_binary_to_uuid_csharp_legacy_rep() {
    let uuid = crate::Uuid::parse_str("00112233445566778899AABBCCDDEEFF").unwrap();
    let bin = Binary::from_uuid_with_representation(uuid, UuidRepresentation::CSharpLegacy);

    assert!(bin.to_uuid().is_err());
    assert!(bin
        .to_uuid_with_representation(UuidRepresentation::Standard)
        .is_err());

    assert_eq!(
        bin.to_uuid_with_representation(UuidRepresentation::CSharpLegacy)
            .unwrap(),
        uuid
    );
}

#[test]
fn test_binary_to_uuid_python_legacy_rep() {
    let uuid = crate::Uuid::parse_str("00112233445566778899AABBCCDDEEFF").unwrap();
    let bin = Binary::from_uuid_with_representation(uuid, UuidRepresentation::PythonLegacy);

    assert!(bin.to_uuid().is_err());
    assert!(bin
        .to_uuid_with_representation(UuidRepresentation::Standard)
        .is_err());

    assert_eq!(
        bin.to_uuid_with_representation(UuidRepresentation::PythonLegacy)
            .unwrap(),
        uuid
    );
}

#[cfg(feature = "uuid-0_8")]
#[test]
fn interop() {
    let uuid = crate::Uuid::new();
    let uuid_uuid = uuid.to_uuid_0_8();
    assert_eq!(uuid.to_string(), uuid_uuid.to_string());
    assert_eq!(&uuid.bytes(), uuid_uuid.as_bytes());

    let back: crate::Uuid = uuid_uuid.into();
    assert_eq!(back, uuid);

    let d_bson = doc! { "uuid": uuid };
    let d_uuid = doc! { "uuid": uuid_uuid };
    assert_eq!(d_bson, d_uuid);
}
