#[cfg(feature = "facet-unstable")]
use facet::Facet;
#[cfg(feature = "serde")]
use serde::Deserialize;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct TestFile {
    pub(crate) description: String,
    pub(crate) bson_type: String,
    pub(crate) test_key: Option<String>,

    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "facet-unstable", facet(default))]
    pub(crate) valid: Vec<Valid>,

    #[cfg_attr(feature = "serde", serde(rename = "decodeErrors"))]
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "decodeErrors"))]
    #[cfg_attr(feature = "facet-unstable", facet(default))]
    pub(crate) decode_errors: Vec<DecodeError>,

    #[cfg_attr(feature = "serde", serde(rename = "parseErrors"))]
    #[cfg_attr(feature = "serde", serde(default))]
    #[cfg_attr(feature = "facet-unstable", facet(rename = "parseErrors"))]
    #[cfg_attr(feature = "facet-unstable", facet(default))]
    pub(crate) parse_errors: Vec<ParseError>,

    #[allow(dead_code)]
    pub(crate) deprecated: Option<bool>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "facet-unstable", derive(Facet))]
#[cfg_attr(feature = "facet-unstable", facet(deny_unknown_fields))]
pub(crate) struct Valid {
    pub(crate) description: String,
    pub(crate) canonical_bson: String,
    pub(crate) canonical_extjson: String,
    pub(crate) relaxed_extjson: Option<String>,
    pub(crate) degenerate_bson: Option<String>,
    pub(crate) degenerate_extjson: Option<String>,
    #[allow(dead_code)]
    pub(crate) converted_bson: Option<String>,
    #[allow(dead_code)]
    pub(crate) converted_extjson: Option<String>,
    pub(crate) lossy: Option<bool>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "facet-unstable", derive(Facet))]
pub(crate) struct DecodeError {
    pub(crate) description: String,
    pub(crate) bson: String,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "facet-unstable", derive(Facet))]
pub(crate) struct ParseError {
    pub(crate) description: String,
    pub(crate) string: String,
}

// Prose test 1: Prohibit null bytes in null-terminated strings when encoding BSON
// This is enforced at compile time via the `CStr` type.

// Prose test 2: Test duplicate keys
#[cfg(all(feature = "serde", feature = "serde_json-1"))]
mod duplicate_keys {
    use crate::{Bson, Document, RawBsonRef, RawDocument, RawDocumentBuf, cstr, error::Result};

    // 2.1 Test encoding
    #[test]
    fn encoding_raw() {
        // Test appending duplicate BSON keys using supported BSON builder API.
        let mut doc = RawDocumentBuf::new();
        doc.append(cstr!("foo"), 1);
        doc.append(cstr!("foo"), 2);
        let got = doc.iter().collect::<Result<Vec<_>>>().unwrap();
        // Expect the following implementation-defined result: Both values preserved in-order
        assert_eq!(
            got,
            vec![
                (cstr!("foo"), RawBsonRef::Int32(1)),
                (cstr!("foo"), RawBsonRef::Int32(2)),
            ]
        );
    }

    #[test]
    fn encoding_parsed() {
        // Test appending duplicate BSON keys using supported BSON builder API.
        let mut doc = Document::new();
        doc.insert("foo", 1);
        doc.insert("foo", 2);
        let got = doc.iter().map(|(k, v)| (k.as_str(), v)).collect::<Vec<_>>();
        // Expect the following implementation-defined result: Last-one wins
        assert_eq!(got, vec![("foo", &Bson::Int32(2))]);
    }

    // 2.2 Test decoding
    #[test]
    fn decoding() {
        // Bytes represent BSON for { 'foo': 1, 'foo': 2 }
        let data = hex::decode("1700000010666f6f000100000010666f6f000200000000").unwrap();

        // Iterating elements without converting to a map
        let raw = RawDocument::from_bytes(&data).unwrap();
        let got = raw.iter().collect::<Result<Vec<_>>>().unwrap();
        // Expect the following implementation-defined result: Both elements returned in-order
        assert_eq!(
            got,
            vec![
                (cstr!("foo"), RawBsonRef::Int32(1)),
                (cstr!("foo"), RawBsonRef::Int32(2)),
            ]
        );

        // Looking up a key in a document
        let got = raw.get("foo").unwrap();
        // Expect the following implementation-defined result: First-one wins
        assert_eq!(got, Some(RawBsonRef::Int32(1)));

        // Converting to a native language map or dictionary
        let got: Document = raw.try_into().unwrap();
        // Expect the following implementation-defined result: Last-one wins
        assert_eq!(got, doc! { "foo": 2 });

        // Converting to Extended JSON
        let got = Bson::Document(raw.try_into().unwrap()).into_relaxed_extjson();
        // Expect the following implementation-defined result: Last-one wins
        assert_eq!(got, serde_json::json!({ "foo": 2 }));
    }

    // 2.3 Test round-trip
    #[test]
    fn round_trip() {
        // Test decoding BSON data containing duplicate keys to a language representation and
        // encoding it back to BSON.
        // Bytes represent BSON for { 'foo': 1, 'foo': 2 }
        let data = hex::decode("1700000010666f6f000100000010666f6f000200000000").unwrap();
        let tmp1: Document = RawDocument::from_bytes(&data).unwrap().try_into().unwrap();
        let tmp2: RawDocumentBuf = tmp1.try_into().unwrap();
        let got = hex::encode(tmp2.as_bytes());

        // Expect the following implementation-defined result:
        // Last-one wins: `got` is BSON for `{ "foo": 2 }`
        assert_eq!(got, "0e00000010666f6f000200000000");
    }

    // 2.4 Test parsing Extended JSON
    #[test]
    fn parse_extjson() {
        // Test parsing an Extended JSON document containing duplicate keys to BSON.
        let got: Document = serde_json::from_str(r#"{ "foo": 1, "foo": 2 }"#).unwrap();

        // Expect the following implementation-defined result: Last-one wins
        assert_eq!(got, doc! { "foo": 2 });
    }
}
