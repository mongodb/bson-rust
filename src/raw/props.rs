use crate::{spec::BinarySubtype, Binary, Bson, Document, JavaScriptCodeWithScope, Regex};

use proptest::prelude::*;

fn arbitrary_binary_subtype() -> impl Strategy<Value = BinarySubtype> {
    prop_oneof![
        Just(BinarySubtype::Generic),
        Just(BinarySubtype::Function),
        Just(BinarySubtype::BinaryOld),
        Just(BinarySubtype::UuidOld),
        Just(BinarySubtype::Uuid),
        Just(BinarySubtype::Md5),
    ]
}

pub(crate) fn arbitrary_bson() -> impl Strategy<Value = Bson> {
    let leaf = prop_oneof![
        Just(Bson::Null),
        any::<String>().prop_map(Bson::String),
        any::<bool>().prop_map(Bson::Boolean),
        any::<f64>().prop_map(Bson::Double),
        any::<i32>().prop_map(Bson::Int32),
        any::<i64>().prop_map(Bson::Int64),
        any::<(String, String)>()
            .prop_map(|(pattern, options)| Bson::RegularExpression(Regex { pattern, options })),
        any::<[u8; 12]>().prop_map(|bytes| Bson::ObjectId(crate::oid::ObjectId::with_bytes(bytes))),
        (arbitrary_binary_subtype(), any::<Vec<u8>>()).prop_map(|(subtype, bytes)| {
            let bytes = if let BinarySubtype::BinaryOld = subtype {
                // BinarySubtype::BinaryOld expects a four byte prefix, which the bson::Bson type
                // leaves up to the caller.

                let mut newbytes = Vec::with_capacity(bytes.len() + 4);
                newbytes.extend_from_slice(&(bytes.len() as i32).to_le_bytes());
                newbytes.extend_from_slice(&bytes);
                newbytes 
            } else {
                bytes
            };
            Bson::Binary(Binary { subtype, bytes })
        }),
        any::<String>().prop_map(Bson::JavaScriptCode),
    ];

    leaf.prop_recursive(4, 256, 10, |inner| {
        prop_oneof![
            prop::collection::hash_map("[^\0]*", inner.clone(), 0..12)
                .prop_map(|map| Bson::Document(map.into_iter().collect())),
            prop::collection::vec(inner.clone(), 0..12).prop_map(Bson::Array),
            (
                prop::collection::hash_map("[^\0]*", inner, 0..12)
                    .prop_map(|map| map.into_iter().collect::<Document>()),
                any::<String>()
            )
                .prop_map(|(scope, code)| Bson::JavaScriptCodeWithScope(
                    JavaScriptCodeWithScope { code, scope }
                )),
        ]
    })
}
