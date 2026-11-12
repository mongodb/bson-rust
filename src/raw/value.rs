use crate::{
    error::Result,
    raw::{
        RawBsonRef,
        Utf8LossyBson,
        Utf8LossyJavaScriptCodeWithScope,
        view::{RawBsonRefView, value_view_at},
    },
    spec::ElementType,
};

#[derive(Clone)]
pub(crate) struct RawValue<'a> {
    kind: ElementType,
    bytes: &'a [u8],
    #[allow(unused)]
    source_offset: usize,
}

impl<'a> RawValue<'a> {
    #[cfg(any(feature = "serde", feature = "facet-unstable"))]
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
        value_view_at(self.kind, self.bytes)?.into_validated()
    }

    pub(crate) fn parse_utf8_lossy(&self) -> Result<Option<Utf8LossyBson<'a>>> {
        let view = value_view_at(self.kind, self.bytes)?;
        Ok(Some(match view {
            RawBsonRefView::String(bytes) => {
                Utf8LossyBson::String(String::from_utf8_lossy(bytes).into_owned())
            }
            RawBsonRefView::JavaScriptCode(bytes) => {
                Utf8LossyBson::JavaScriptCode(String::from_utf8_lossy(bytes).into_owned())
            }
            RawBsonRefView::Symbol(bytes) => {
                Utf8LossyBson::Symbol(String::from_utf8_lossy(bytes).into_owned())
            }
            RawBsonRefView::JavaScriptCodeWithScope { code, scope } => {
                Utf8LossyBson::JavaScriptCodeWithScope(Utf8LossyJavaScriptCodeWithScope {
                    code: String::from_utf8_lossy(code).into_owned(),
                    scope,
                })
            }
            RawBsonRefView::DbPointer { namespace, id } => {
                Utf8LossyBson::DbPointer(crate::DbPointer {
                    namespace: String::from_utf8_lossy(namespace).into_owned(),
                    id,
                })
            }
            RawBsonRefView::RegularExpression { pattern, options } => {
                Utf8LossyBson::RegularExpression(crate::Regex {
                    pattern: String::from_utf8_lossy(pattern).into_owned().try_into()?,
                    options: String::from_utf8_lossy(options).into_owned().try_into()?,
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

    #[cfg(feature = "facet-unstable")]
    pub(crate) fn source_offset(&self) -> usize {
        self.source_offset
    }

    #[cfg(feature = "facet-unstable")]
    pub(crate) fn span(&self) -> facet_reflect::Span {
        facet_reflect::Span::new(self.source_offset, self.bytes.len())
    }
}
