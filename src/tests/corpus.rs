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
