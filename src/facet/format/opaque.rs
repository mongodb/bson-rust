use facet::{Facet, FacetOpaqueAdapter, OpaqueDeserialize, OpaqueSerialize, PtrConst};

use crate::{
    Binary,
    RawBinaryRef,
    RawJavaScriptCodeWithScope,
    RawJavaScriptCodeWithScopeRef,
    RawRegexRef,
    Regex,
    Timestamp,
    error::Error,
    raw::RawDocumentBuf,
};

#[derive(Facet)]
#[facet(opaque)]
struct UnSerializable;

static UN_SERIALIZABLE: UnSerializable = UnSerializable;

impl UnSerializable {
    const OPAQUE: OpaqueSerialize = OpaqueSerialize {
        ptr: PtrConst::new_sized(&UN_SERIALIZABLE as *const UnSerializable),
        shape: UnSerializable::SHAPE,
    };
}

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

macro_rules! adapter {
    (
        struct $an:ident;

        fn serialize($val:ident: &$send:ty) {
            $ser:expr
        }

        fn deserialize($input:ident) -> Result<$recv:ty> {
            $deser:expr
        }
    ) => {
        pub(crate) struct $an;

        impl FacetOpaqueAdapter for $an {
            type Error = Error;
            type SendValue<'a> = $send;
            type RecvValue<'de> = $recv;

            fn serialize_map($val: &Self::SendValue<'_>) -> OpaqueSerialize {
                $ser
            }

            fn deserialize_build<'de>(
                $input: OpaqueDeserialize<'de>,
            ) -> std::result::Result<Self::RecvValue<'de>, Self::Error> {
                $deser
            }
        }
    };
    (
        struct $an:ident;

        fn deserialize($input:ident) -> Result<$recv:ty> {
            $deser:expr
        }
    ) => {
        adapter! {
            struct $an;

            fn serialize(_value: &$recv) {
                UnSerializable::OPAQUE
            }

            fn deserialize($input) -> Result<$recv> {
                $deser
            }
        }
    };
}

adapter! {
    struct RawDocumentBufAdapter;

    fn serialize(value: &RawDocumentBuf) {
        OpaqueSerialize {
            ptr: PtrConst::new(&value.as_bytes() as *const &[u8]),
            shape: <&[u8] as Facet>::SHAPE,
        }
    }

    fn deserialize(input) -> Result<RawDocumentBuf> {
        RawDocumentBuf::from_bytes(input_vec(input))
    }
}

adapter! {
    struct RegexAdapter;

    fn deserialize(input) -> Result<Regex> {
        Ok(RawRegexRef::parse(input_slice(&input))?.into())
    }
}

adapter! {
    struct BinaryAdapter;

    fn deserialize(input) -> Result<Binary> {
        Ok(RawBinaryRef::parse(input_slice(&input))?.to_binary())
    }
}

adapter! {
    struct TimestampAdapter;

    fn deserialize(input) -> Result<Timestamp> {
        Timestamp::parse(input_slice(&input))
    }
}

adapter! {
    struct RawJavaScriptCodeWithScopeAdapter;

    fn deserialize(input) -> Result<RawJavaScriptCodeWithScope> {
        Ok(RawJavaScriptCodeWithScopeRef::parse(input_slice(&input))?.into())
    }
}
