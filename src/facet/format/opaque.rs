use facet::{Facet, FacetOpaqueAdapter, OpaqueDeserialize, OpaqueSerialize, PtrConst};

use crate::{
    RawRegexRef,
    Regex,
    error::{Error, Result},
    raw::RawDocumentBuf,
};

#[derive(Facet)]
#[facet(opaque)]
struct UnSerializable;

static UN_SERIALIZABLE: UnSerializable = UnSerializable;

fn input_slice<'de>(input: &'de OpaqueDeserialize<'de>) -> &'de [u8] {
    match input {
        OpaqueDeserialize::Borrowed(slice) => slice,
        OpaqueDeserialize::Owned(vec) => vec.as_slice(),
    }
}

fn input_vec<'de>(input: OpaqueDeserialize<'de>) -> Vec<u8> {
    match input {
        OpaqueDeserialize::Borrowed(slice) => slice.to_owned(),
        OpaqueDeserialize::Owned(vec) => vec,
    }
}

pub(crate) struct RawDocumentBufAdapter;

impl FacetOpaqueAdapter for RawDocumentBufAdapter {
    type Error = Error;
    type SendValue<'a> = RawDocumentBuf;
    type RecvValue<'de> = RawDocumentBuf;

    fn serialize_map(value: &Self::SendValue<'_>) -> OpaqueSerialize {
        OpaqueSerialize {
            ptr: PtrConst::new(&value.as_bytes() as *const &[u8]),
            shape: <&[u8] as Facet>::SHAPE,
        }
    }

    fn deserialize_build<'de>(input: OpaqueDeserialize<'de>) -> Result<Self::RecvValue<'de>> {
        RawDocumentBuf::from_bytes(input_vec(input))
    }
}

pub(crate) struct RegexAdapter;

impl FacetOpaqueAdapter for RegexAdapter {
    type Error = Error;
    type SendValue<'a> = Regex;
    type RecvValue<'de> = Regex;

    fn serialize_map(_value: &Self::SendValue<'_>) -> OpaqueSerialize {
        OpaqueSerialize {
            ptr: PtrConst::new(&UN_SERIALIZABLE as *const UnSerializable),
            shape: UnSerializable::SHAPE,
        }
    }

    fn deserialize_build<'de>(
        input: OpaqueDeserialize<'de>,
    ) -> std::result::Result<Self::RecvValue<'de>, Self::Error> {
        Ok(RawRegexRef::parse(input_slice(&input))?.into())
    }
}
