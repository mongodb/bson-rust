use facet::{Facet, FacetOpaqueAdapter, OpaqueDeserialize, OpaqueSerialize, PtrConst};

use crate::{
    Binary,
    DateTime,
    DbPointer,
    Decimal128,
    Document,
    JavaScriptCodeWithScope,
    RawArrayBuf,
    RawBinaryRef,
    RawDbPointerRef,
    RawJavaScriptCodeWithScope,
    RawJavaScriptCodeWithScopeRef,
    RawRegexRef,
    Regex,
    Timestamp,
    error::{Error, Result},
    oid::ObjectId,
    raw::{CString, RawDocument, RawDocumentBuf},
    spec::ElementType,
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

fn check_tag(expected: ElementType, actual: Option<&u8>) -> Result<()> {
    let expected = expected as u8;
    match actual {
        Some(t) if *t == expected => Ok(()),
        None => Err(Error::malformed_bytes("empty input")),
        Some(t) => Err(Error::malformed_bytes(format!(
            "invalid type tag: expected {expected}, got {t}"
        ))),
    }
}

fn input_slice<'de>(input: &'de OpaqueDeserialize<'de>, kind: ElementType) -> Result<&'de [u8]> {
    let slice = match input {
        OpaqueDeserialize::Borrowed(slice) => slice,
        OpaqueDeserialize::Owned(vec) => vec.as_slice(),
    };
    // omit type tag
    check_tag(kind, slice.last())?;
    Ok(&slice[0..slice.len() - 1])
}

fn input_vec<'de>(input: OpaqueDeserialize<'de>, kind: ElementType) -> Result<Vec<u8>> {
    let mut vec = match input {
        OpaqueDeserialize::Borrowed(slice) => slice.to_owned(),
        OpaqueDeserialize::Owned(vec) => vec,
    };
    // omit type tag
    check_tag(kind, vec.pop().as_ref())?;
    Ok(vec)
}

macro_rules! adapter {
    (
        struct $an:ident;

        fn serialize($val:ident: &$send:ty) $ser:block

        fn deserialize($input:ident: OpaqueDeserialize) -> Result<$recv:ty> $deser:block
    ) => {
        pub(crate) struct $an;

        impl FacetOpaqueAdapter for $an {
            type Error = Error;
            type SendValue<'a> = $send;
            type RecvValue<'de> = $recv;

            fn serialize_map($val: &Self::SendValue<'_>) -> OpaqueSerialize $ser

            fn deserialize_build<'de>(
                $input: OpaqueDeserialize<'de>,
            ) -> std::result::Result<Self::RecvValue<'de>, Self::Error> $deser
        }
    };
    (
        struct $an:ident;

        fn deserialize($input:ident: OpaqueDeserialize) -> Result<$recv:ty> $deser:block
    ) => {
        adapter! {
            struct $an;

            fn serialize(_value: &$recv) {
                UnSerializable::OPAQUE
            }

            fn deserialize($input: OpaqueDeserialize) -> Result<$recv> $deser
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

    fn deserialize(input: OpaqueDeserialize) -> Result<RawDocumentBuf> {
        RawDocumentBuf::from_bytes(input_vec(input, ElementType::EmbeddedDocument)?)
    }
}

adapter! {
    struct RegexAdapter;

    fn deserialize(input: OpaqueDeserialize) -> Result<Regex> {
        RawRegexRef::parse(input_slice(&input, ElementType::RegularExpression)?).map(Regex::from)
    }
}

adapter! {
    struct BinaryAdapter;

    fn deserialize(input: OpaqueDeserialize) -> Result<Binary> {
        Ok(RawBinaryRef::parse(input_slice(&input, ElementType::Binary)?)?.to_binary())
    }
}

adapter! {
    struct TimestampAdapter;

    fn deserialize(input: OpaqueDeserialize) -> Result<Timestamp> {
        Timestamp::parse(input_slice(&input, ElementType::Timestamp)?)
    }
}

adapter! {
    struct RawJavaScriptCodeWithScopeAdapter;

    fn deserialize(input: OpaqueDeserialize) -> Result<RawJavaScriptCodeWithScope> {
        Ok(RawJavaScriptCodeWithScopeRef::parse(
            input_slice(&input, ElementType::JavaScriptCodeWithScope)?
        )?.into())
    }
}

adapter! {
    struct ObjectIdAdapter;

    fn serialize(value: &ObjectId) {
        OpaqueSerialize {
            ptr: PtrConst::new(&value.as_bytes_slice() as *const &[u8]),
            shape: <&[u8] as Facet>::SHAPE,
        }
    }

    fn deserialize(input: OpaqueDeserialize) -> Result<ObjectId> {
        ObjectId::parse(input_slice(&input, ElementType::ObjectId)?)
    }
}

adapter! {
    struct Decimal128Adapter;

    fn serialize(value: &Decimal128) {
        OpaqueSerialize {
            ptr: PtrConst::new(&value.as_bytes_slice() as *const &[u8]),
            shape: <&[u8] as Facet>::SHAPE,
        }
    }

    fn deserialize(input: OpaqueDeserialize) -> Result<Decimal128> {
        Decimal128::parse(input_slice(&input, ElementType::Decimal128)?)
    }
}

adapter! {
    struct RawArrayBufAdapter;

    fn serialize(value: &RawArrayBuf) {
        OpaqueSerialize {
            ptr: PtrConst::new(&value.as_bytes() as *const &[u8]),
            shape: <&[u8] as Facet>::SHAPE,
        }
    }

    fn deserialize(input: OpaqueDeserialize) -> Result<RawArrayBuf> {
        Ok(RawArrayBuf::from_raw_document_buf(
            RawDocumentBuf::from_bytes(input_vec(input, ElementType::Array)?)?
        ))
    }
}

adapter! {
    struct DateTimeAdapter;

    fn deserialize(input: OpaqueDeserialize) -> Result<DateTime> {
        DateTime::parse(input_slice(&input, ElementType::DateTime)?)
    }
}

adapter! {
    struct DbPointerAdapter;

    fn deserialize(input: OpaqueDeserialize) -> Result<DbPointer> {
        Ok(RawDbPointerRef::parse(input_slice(&input, ElementType::DbPointer)?)?.into())
    }
}

adapter! {
    struct JavaScriptCodeWithScopeAdapter;

    fn deserialize(input: OpaqueDeserialize) -> Result<JavaScriptCodeWithScope> {
        Ok(RawJavaScriptCodeWithScopeRef::parse(
            input_slice(&input, ElementType::JavaScriptCodeWithScope)?
        )?.try_into()?)
    }
}

adapter! {
    struct DocumentAdapter;

    fn deserialize(input: OpaqueDeserialize) -> Result<Document> {
        RawDocument::from_bytes(
            input_slice(&input, ElementType::EmbeddedDocument)?
        )?.try_into()
    }
}

adapter! {
    struct CStringAdapter;

    fn deserialize(input: OpaqueDeserialize) -> Result<CString> {
        crate::raw::read_lenencode(input_slice(&input, ElementType::String)?)?
            .to_owned()
            .try_into()
    }
}
