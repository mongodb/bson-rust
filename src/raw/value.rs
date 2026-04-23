use std::convert::TryInto;

use crate::{
    DateTime,
    Decimal128,
    Timestamp,
    error::{Error, Result},
    oid::ObjectId,
    raw::{
        MIN_CODE_WITH_SCOPE_SIZE,
        Utf8LossyBson,
        Utf8LossyJavaScriptCodeWithScope,
        array::RawArray,
        bool_from_slice,
        bson_ref::{
            RawBinaryRef,
            RawBsonRef,
            RawDbPointerRef,
            RawJavaScriptCodeWithScopeRef,
            RawRegexRef,
        },
        cstring_bytes,
        document::RawDocument,
        f64_from_slice,
        i32_from_slice,
        i64_from_slice,
        read_lenencode_bytes,
        try_to_str,
    },
    spec::ElementType,
};

#[derive(Clone)]
pub(crate) struct RawValue<'a> {
    kind: ElementType,
    bytes: &'a [u8],
    source_offset: usize,
}

impl<'a> RawValue<'a> {
    pub(crate) fn new(kind: ElementType, bytes: &'a [u8]) -> Self {
        Self {
            kind,
            bytes,
            source_offset: 0,
        }
    }

    pub(crate) fn new_at(kind: ElementType, bytes: &'a [u8], source_offset: usize) -> Self {
        Self {
            kind,
            bytes,
            source_offset,
        }
    }

    pub(crate) fn parse(&self) -> Result<RawBsonRef<'a>> {
        Ok(match self.kind {
            ElementType::Null => RawBsonRef::Null,
            ElementType::Undefined => RawBsonRef::Undefined,
            ElementType::MinKey => RawBsonRef::MinKey,
            ElementType::MaxKey => RawBsonRef::MaxKey,
            ElementType::ObjectId => RawBsonRef::ObjectId(ObjectId::parse(self.bytes)?),
            ElementType::Int32 => RawBsonRef::Int32(i32_from_slice(self.bytes)?),
            ElementType::Int64 => RawBsonRef::Int64(i64_from_slice(self.bytes)?),
            ElementType::Double => RawBsonRef::Double(f64_from_slice(self.bytes)?),
            ElementType::String => RawBsonRef::String(self.read_str()?),
            ElementType::EmbeddedDocument => {
                RawBsonRef::Document(RawDocument::from_bytes(self.bytes)?)
            }
            ElementType::Array => {
                RawBsonRef::Array(RawArray::from_doc(RawDocument::from_bytes(self.bytes)?))
            }
            ElementType::Boolean => RawBsonRef::Boolean(
                bool_from_slice(self.bytes).map_err(|e| Error::malformed_bytes(e))?,
            ),
            ElementType::DateTime => RawBsonRef::DateTime(DateTime::parse(self.bytes)?),
            ElementType::Decimal128 => RawBsonRef::Decimal128(Decimal128::parse(self.bytes)?),
            ElementType::JavaScriptCode => RawBsonRef::JavaScriptCode(self.read_str()?),
            ElementType::Symbol => RawBsonRef::Symbol(self.read_str()?),
            ElementType::DbPointer => RawBsonRef::DbPointer(RawDbPointerRef::parse(self.bytes)?),
            ElementType::RegularExpression => {
                RawBsonRef::RegularExpression(RawRegexRef::parse(self.bytes)?)
            }
            ElementType::Timestamp => RawBsonRef::Timestamp(Timestamp::parse(self.bytes)?),
            ElementType::Binary => RawBsonRef::Binary(RawBinaryRef::parse(self.bytes)?),
            ElementType::JavaScriptCodeWithScope => RawBsonRef::JavaScriptCodeWithScope(
                RawJavaScriptCodeWithScopeRef::parse(self.bytes)?,
            ),
        })
    }

    pub(crate) fn parse_utf8_lossy(&self) -> Result<Option<Utf8LossyBson<'a>>> {
        Ok(Some(match self.kind {
            ElementType::String => Utf8LossyBson::String(self.read_utf8_lossy()),
            ElementType::JavaScriptCode => Utf8LossyBson::JavaScriptCode(self.read_utf8_lossy()),
            ElementType::JavaScriptCodeWithScope => {
                if self.bytes.len() < MIN_CODE_WITH_SCOPE_SIZE as usize {
                    return Err(Error::malformed_bytes("code with scope length too small"));
                }

                let slice = self.bytes;
                let code = String::from_utf8_lossy(read_lenencode_bytes(&slice[4..])?).into_owned();
                let scope_start = 4 + 4 + code.len() + 1;
                if scope_start >= slice.len() {
                    return Err(Error::malformed_bytes("code with scope length overrun"));
                }
                let scope = RawDocument::from_bytes(&slice[scope_start..])?;

                Utf8LossyBson::JavaScriptCodeWithScope(Utf8LossyJavaScriptCodeWithScope {
                    code,
                    scope,
                })
            }
            ElementType::Symbol => Utf8LossyBson::Symbol(self.read_utf8_lossy()),
            ElementType::DbPointer => Utf8LossyBson::DbPointer(crate::DbPointer {
                namespace: String::from_utf8_lossy(read_lenencode_bytes(self.bytes)?).into_owned(),
                id: self.get_oid_at(self.bytes.len() - 12)?,
            }),
            ElementType::RegularExpression => {
                let pattern = String::from_utf8_lossy(cstring_bytes(self.bytes)?).into_owned();
                let pattern_len = pattern.len();
                Utf8LossyBson::RegularExpression(crate::Regex {
                    pattern: pattern.try_into()?,
                    options: String::from_utf8_lossy(cstring_bytes(
                        &self.bytes[pattern_len + 1..],
                    )?)
                    .into_owned()
                    .try_into()?,
                })
            }
            _ => return Ok(None),
        }))
    }

    pub(crate) fn bytes(&self) -> &'a [u8] {
        self.bytes
    }

    pub(crate) fn kind(&self) -> ElementType {
        self.kind
    }

    pub(crate) fn source_offset(&self) -> usize {
        self.source_offset
    }

    #[cfg(feature = "facet-unstable")]
    pub(crate) fn span(&self) -> facet_reflect::Span {
        facet_reflect::Span::new(self.source_offset, self.bytes.len())
    }

    fn slice_bounds(&self, start_at: usize, size: usize) -> &'a [u8] {
        &self.bytes[start_at..(start_at + size)]
    }

    fn read_str(&self) -> Result<&'a str> {
        try_to_str(self.str_bytes())
    }

    fn str_bytes(&self) -> &'a [u8] {
        self.slice_bounds(4, self.bytes.len() - 4 - 1)
    }

    fn read_utf8_lossy(&self) -> String {
        String::from_utf8_lossy(self.str_bytes()).into_owned()
    }

    fn get_oid_at(&self, start_at: usize) -> Result<ObjectId> {
        Ok(ObjectId::from_bytes(
            self.bytes[start_at..(start_at + 12)]
                .try_into()
                .map_err(|e| Error::malformed_bytes(e))?,
        ))
    }
}
