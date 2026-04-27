use facet::{Facet, FacetOpaqueAdapter, OpaqueDeserialize, OpaqueSerialize, PtrConst};

use crate::{
    Binary,
    Bson,
    DateTime,
    DbPointer,
    Decimal128,
    Document,
    JavaScriptCodeWithScope,
    RawArrayBuf,
    RawBinaryRef,
    RawBson,
    RawDbPointerRef,
    RawJavaScriptCodeWithScope,
    RawJavaScriptCodeWithScopeRef,
    RawRegexRef,
    Regex,
    Timestamp,
    error::{Error, Result},
    oid::ObjectId,
    raw::{CString, RawDocument, RawDocumentBuf, value::RawValue},
    spec::ElementType,
};

// Generate Facet opaque adapters for these types: each line is (type, serialize_fn,
// deserialize_fn;).  serialize_fn is only available for types that are thin wrappers around byte
// arrays, and will only be used when serializing these types to non-bson formats; all other types
// will produce an error in that context.
adapters! {
    RawDocumentBuf,             ser_rawdoc, de_rawdoc;
    Regex,                      _,          de_regex;
    Binary,                     _,          de_binary;
    Timestamp,                  _,          de_timestamp;
    RawJavaScriptCodeWithScope, _,          de_raw_jscws;
    ObjectId,                   ser_oid,    de_oid;
    Decimal128,                 ser_dec,    de_dec;
    RawArrayBuf,                ser_rawarr, de_rawarr;
    DateTime,                   _,          de_datetime;
    DbPointer,                  _,          de_dbptr;
    JavaScriptCodeWithScope,    _,          de_jscws;
    Document,                   _,          de_doc;
    CString,                    _,          de_cstring;
    RawBson,                    _,          de_rawbson;
    Bson,                       _,          de_bson;
}

fn ser_rawdoc(value: &RawDocumentBuf) -> OpaqueSerialize {
    OpaqueSerialize {
        ptr: PtrConst::new(&value.as_bytes() as *const &[u8]),
        shape: <&[u8] as Facet>::SHAPE,
    }
}

fn de_rawdoc(input: OpaqueDeserialize) -> Result<RawDocumentBuf> {
    RawDocumentBuf::from_bytes(input_vec(input, ElementType::EmbeddedDocument)?)
}

fn de_regex(input: OpaqueDeserialize) -> Result<Regex> {
    RawRegexRef::parse(input_slice(&input, ElementType::RegularExpression)?).map(Regex::from)
}

fn de_binary(input: OpaqueDeserialize) -> Result<Binary> {
    Ok(RawBinaryRef::parse(input_slice(&input, ElementType::Binary)?)?.to_binary())
}

fn de_timestamp(input: OpaqueDeserialize) -> Result<Timestamp> {
    Timestamp::parse(input_slice(&input, ElementType::Timestamp)?)
}

fn de_raw_jscws(input: OpaqueDeserialize) -> Result<RawJavaScriptCodeWithScope> {
    Ok(RawJavaScriptCodeWithScopeRef::parse(input_slice(
        &input,
        ElementType::JavaScriptCodeWithScope,
    )?)?
    .into())
}

fn ser_oid(value: &ObjectId) -> OpaqueSerialize {
    OpaqueSerialize {
        ptr: PtrConst::new(&value.as_bytes_slice() as *const &[u8]),
        shape: <&[u8] as Facet>::SHAPE,
    }
}

fn de_oid(input: OpaqueDeserialize) -> Result<ObjectId> {
    ObjectId::parse(input_slice(&input, ElementType::ObjectId)?)
}

fn ser_dec(value: &Decimal128) -> OpaqueSerialize {
    OpaqueSerialize {
        ptr: PtrConst::new(&value.as_bytes_slice() as *const &[u8]),
        shape: <&[u8] as Facet>::SHAPE,
    }
}

fn de_dec(input: OpaqueDeserialize) -> Result<Decimal128> {
    Decimal128::parse(input_slice(&input, ElementType::Decimal128)?)
}

fn ser_rawarr(value: &RawArrayBuf) -> OpaqueSerialize {
    OpaqueSerialize {
        ptr: PtrConst::new(&value.as_bytes() as *const &[u8]),
        shape: <&[u8] as Facet>::SHAPE,
    }
}

fn de_rawarr(input: OpaqueDeserialize) -> Result<RawArrayBuf> {
    Ok(RawArrayBuf::from_raw_document_buf(
        RawDocumentBuf::from_bytes(input_vec(input, ElementType::Array)?)?,
    ))
}

fn de_datetime(input: OpaqueDeserialize) -> Result<DateTime> {
    DateTime::parse(input_slice(&input, ElementType::DateTime)?)
}

fn de_dbptr(input: OpaqueDeserialize) -> Result<DbPointer> {
    Ok(RawDbPointerRef::parse(input_slice(&input, ElementType::DbPointer)?)?.into())
}

fn de_jscws(input: OpaqueDeserialize) -> Result<JavaScriptCodeWithScope> {
    RawJavaScriptCodeWithScopeRef::parse(input_slice(
        &input,
        ElementType::JavaScriptCodeWithScope,
    )?)?
    .try_into()
}

fn de_doc(input: OpaqueDeserialize) -> Result<Document> {
    RawDocument::from_bytes(input_slice(&input, ElementType::EmbeddedDocument)?)?.try_into()
}

fn de_cstring(input: OpaqueDeserialize) -> Result<CString> {
    crate::raw::read_lenencode(input_slice(&input, ElementType::String)?)?
        .to_owned()
        .try_into()
}

fn de_rawbson(input: OpaqueDeserialize) -> Result<RawBson> {
    let bytes = match &input {
        OpaqueDeserialize::Borrowed(slice) => slice,
        OpaqueDeserialize::Owned(vec) => vec.as_slice(),
    };
    let tag = bytes[bytes.len() - 1];
    let Some(kind) = ElementType::from(tag) else {
        return Err(Error::malformed_bytes(format!("invalid type tag {tag}")));
    };
    let bytes = &bytes[0..bytes.len() - 1];
    let value = RawValue::new(kind, bytes);
    value.parse().map(RawBson::from)
}

fn de_bson(input: OpaqueDeserialize) -> Result<Bson> {
    de_rawbson(input)?.try_into()
}

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

fn input_vec(input: OpaqueDeserialize, kind: ElementType) -> Result<Vec<u8>> {
    let mut vec = match input {
        OpaqueDeserialize::Borrowed(slice) => slice.to_owned(),
        OpaqueDeserialize::Owned(vec) => vec,
    };
    // omit type tag
    check_tag(kind, vec.pop().as_ref())?;
    Ok(vec)
}

macro_rules! adapter {
    ($ty:ident, _, $de:ident $(,)?) => {
        adapter!($ty, ser_unserializable, $de);
    };
    ($ty:ident, $ser:ident, $de:ident $(,)?) => {
        ::paste::paste! {
            pub(crate) struct [<$ty Adapter>];
            impl FacetOpaqueAdapter for [<$ty Adapter>] {
                type Error = Error;
                type SendValue<'a> = $ty;
                type RecvValue<'de> = $ty;
                fn serialize_map(value: &Self::SendValue<'_>) -> OpaqueSerialize {
                    $ser(value)
                }
                fn deserialize_build<'de>(
                    input: OpaqueDeserialize<'de>,
                ) -> std::result::Result<Self::RecvValue<'de>, Self::Error> {
                    $de(input)
                }
            }
        }
    };
}
use adapter;

fn ser_unserializable<T>(_: &T) -> OpaqueSerialize {
    UnSerializable::OPAQUE
}

macro_rules! adapters {
    ($($ty:ident, $ser:tt, $de:ident);* $(;)?) => {
        $( adapter!($ty, $ser, $de); )*
    };
}
use adapters;
