use std::convert::TryInto;

use crate::{
    Bson,
    RawBson,
    raw::{CStr, Error, MIN_BSON_DOCUMENT_SIZE, RawValue, Result, read_cstring},
    spec::ElementType,
};

use super::{RawBsonRef, RawDocument, checked_add, i32_from_slice, read_len};

/// An iterator over the document's entries.
pub struct Iter<'a> {
    inner: RawIter<'a>,
}

impl<'a> Iter<'a> {
    pub(crate) fn new(doc: &'a RawDocument) -> Self {
        Iter {
            inner: RawIter::new(doc),
        }
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<(&'a CStr, RawBsonRef<'a>)>;

    fn next(&mut self) -> Option<Result<(&'a CStr, RawBsonRef<'a>)>> {
        match self.inner.next() {
            Some(Ok(elem)) => match elem.value() {
                Err(e) => Some(Err(e)),
                Ok(value) => Some(Ok((elem.key, value))),
            },
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

/// An iterator over the document's elements.
pub struct RawIter<'a> {
    bytes: &'a [u8],
    offset: usize,

    /// Whether the underlying doc is assumed to be valid or if an error has been encountered.
    /// After an error, all subsequent iterations will return None.
    valid: bool,

    /// When true, the iterator treats a null byte as the document terminator only at the last
    /// position of `bytes`; a null encountered earlier is an error. When false (used by
    /// `new_unchecked` for flat parsers that span nested documents), any null byte ends
    /// iteration.
    strict: bool,
}

impl<'a> RawIter<'a> {
    pub(crate) fn new(doc: &'a RawDocument) -> Self {
        Self {
            bytes: doc.as_bytes(),
            offset: 4,
            valid: true,
            strict: true,
        }
    }

    pub(crate) fn new_unchecked(bytes: &'a [u8], offset: usize) -> Self {
        Self {
            bytes,
            offset,
            valid: true,
            strict: false,
        }
    }

    fn verify_enough_bytes(&self, start: usize, num_bytes: usize) -> Result<()> {
        let end = checked_add(start, num_bytes)?;
        if self.bytes.get(start..end).is_none() {
            return Err(Error::malformed_bytes(format!(
                "length exceeds remaining length of buffer: {} vs {}",
                num_bytes,
                self.bytes.len() - start
            )));
        }
        Ok(())
    }

    pub(crate) fn next_document_len(&self, starting_at: usize) -> Result<usize> {
        self.verify_enough_bytes(starting_at, MIN_BSON_DOCUMENT_SIZE as usize)?;
        let size = i32_from_slice(&self.bytes[starting_at..])? as usize;

        if size < MIN_BSON_DOCUMENT_SIZE as usize {
            return Err(Error::malformed_bytes(format!(
                "document too small: {} bytes",
                size
            )));
        }

        self.verify_enough_bytes(starting_at, size)?;

        if self.bytes[starting_at + size - 1] != 0 {
            return Err(Error::malformed_bytes("not null terminated"));
        }
        Ok(size)
    }
}

/// A view into a value contained in a [`RawDocument`] or [`RawDocumentBuf`](crate::RawDocumentBuf).
/// The underlying bytes of the element are not parsed or validated; call [`RawElement::value`] or
/// one of the `TryFrom` implementations to convert the element into a BSON value.
#[derive(Clone)]
pub struct RawElement<'a> {
    key: &'a CStr,
    value: RawValue<'a>,
}

impl<'a> TryFrom<RawElement<'a>> for RawBsonRef<'a> {
    type Error = Error;

    fn try_from(element: RawElement<'a>) -> Result<Self> {
        element.value()
    }
}

impl TryFrom<RawElement<'_>> for RawBson {
    type Error = Error;

    fn try_from(element: RawElement<'_>) -> Result<Self> {
        Ok(element.value()?.into())
    }
}

impl TryFrom<RawElement<'_>> for Bson {
    type Error = Error;

    fn try_from(element: RawElement<'_>) -> Result<Self> {
        element.value()?.try_into()
    }
}

impl<'a> RawElement<'a> {
    #[cfg(feature = "serde")]
    pub(crate) fn toplevel(bytes: &'a [u8]) -> Self {
        use crate::raw::cstr;

        Self {
            key: cstr!("TOPLEVEL"),
            value: RawValue::new(ElementType::EmbeddedDocument, bytes),
        }
    }

    /// The size of the element.
    pub fn size(&self) -> usize {
        self.value.bytes.len()
    }

    /// The document key the element corresponds to.
    pub fn key(&self) -> &'a CStr {
        self.key
    }

    /// The type of the element.
    pub fn element_type(&self) -> ElementType {
        self.value.kind
    }

    /// Parses this element into a [`RawBsonRef`] and returns an error if the underlying bytes are
    /// invalid.
    pub fn value(&self) -> Result<RawBsonRef<'a>> {
        self.value
            .parse()
            .map_err(|e| e.with_key(self.key.as_str()))
    }

    pub(crate) fn value_raw(&self) -> &RawValue<'a> {
        &self.value
    }

    /// Parses this element into [`RawBson`], replacing any invalid UTF-8 strings with the Unicode
    /// replacement character. Returns an error if the underlying bytes are invalid.
    pub fn value_utf8_lossy(&self) -> Result<RawBson> {
        match self.value_utf8_lossy_inner()? {
            Some(v) => Ok(v.into()),
            None => Ok(self.value()?.into()),
        }
    }

    pub(crate) fn value_utf8_lossy_inner(&self) -> Result<Option<Utf8LossyBson<'a>>> {
        self.value
            .parse_utf8_lossy()
            .map_err(|e| e.with_key(self.key.as_str()))
    }
}

impl RawIter<'_> {
    fn get_next_length_at(&self, start_at: usize) -> Result<usize> {
        let len = i32_from_slice(&self.bytes[start_at..])?;
        if len < 0 {
            Err(Error::malformed_bytes("lengths can't be negative"))
        } else {
            Ok(len as usize)
        }
    }

    fn get_next_kvp(&mut self, offset: usize) -> Result<(ElementType, usize)> {
        let element_type = match ElementType::from(self.bytes[self.offset]) {
            Some(et) => et,
            None => {
                return Err(Error::malformed_bytes(format!(
                    "invalid tag: {}",
                    self.bytes[self.offset]
                )));
            }
        };

        let element_size = match element_type {
            ElementType::Boolean => 1,
            ElementType::Int32 => 4,
            ElementType::Int64 => 8,
            ElementType::Double => 8,
            ElementType::DateTime => 8,
            ElementType::Timestamp => 8,
            ElementType::ObjectId => 12,
            ElementType::Decimal128 => 16,
            ElementType::Null => 0,
            ElementType::Undefined => 0,
            ElementType::MinKey => 0,
            ElementType::MaxKey => 0,
            ElementType::String => read_len(&self.bytes[offset..])?,
            ElementType::EmbeddedDocument => self.next_document_len(offset)?,
            ElementType::Array => self.next_document_len(offset)?,
            ElementType::Binary => self.get_next_length_at(offset)? + 4 + 1,
            ElementType::RegularExpression => {
                let pattern = read_cstring(&self.bytes[offset..])?;
                let options = read_cstring(&self.bytes[offset + pattern.len() + 1..])?;
                pattern.len() + 1 + options.len() + 1
            }
            ElementType::DbPointer => read_len(&self.bytes[offset..])? + 12,
            ElementType::Symbol => read_len(&self.bytes[offset..])?,
            ElementType::JavaScriptCode => read_len(&self.bytes[offset..])?,
            ElementType::JavaScriptCodeWithScope => self.get_next_length_at(offset)?,
        };

        self.verify_enough_bytes(offset, element_size)?;
        self.offset = offset + element_size;

        Ok((element_type, element_size))
    }
}

impl<'a> Iterator for RawIter<'a> {
    type Item = Result<RawElement<'a>>;

    fn next(&mut self) -> Option<Result<RawElement<'a>>> {
        if !self.valid {
            return None;
        } else if self.offset >= self.bytes.len() {
            self.valid = false;
            return Some(Err(Error::malformed_bytes("iteration overflowed document")));
        } else if self.strict {
            if self.offset == self.bytes.len() - 1 {
                return if self.bytes[self.offset] == 0 {
                    None
                } else {
                    self.valid = false;
                    Some(Err(Error::malformed_bytes("document not null terminated")))
                };
            }
        } else if self.bytes[self.offset] == 0 {
            // end of document marker
            return None;
        }

        let key = match read_cstring(&self.bytes[self.offset + 1..]) {
            Ok(k) => k,
            Err(e) => {
                self.valid = false;
                return Some(Err(e));
            }
        };
        let offset = self.offset + 1 + key.len() + 1; // type specifier + key + \0

        Some(match self.get_next_kvp(offset) {
            Ok((kind, size)) => Ok(RawElement {
                key,
                value: RawValue {
                    kind,
                    bytes: &self.bytes[offset..offset + size],
                    source_offset: offset,
                },
            }),
            Err(error) => {
                self.valid = false;
                Err(error.with_key(key.as_str()))
            }
        })
    }
}

pub(crate) enum Utf8LossyBson<'a> {
    String(String),
    JavaScriptCode(String),
    JavaScriptCodeWithScope(Utf8LossyJavaScriptCodeWithScope<'a>),
    Symbol(String),
    DbPointer(crate::DbPointer),
    RegularExpression(crate::Regex),
}

pub(crate) struct Utf8LossyJavaScriptCodeWithScope<'a> {
    pub(crate) code: String,
    pub(crate) scope: &'a RawDocument,
}

impl<'a> From<Utf8LossyBson<'a>> for RawBson {
    fn from(value: Utf8LossyBson<'a>) -> Self {
        match value {
            Utf8LossyBson::String(s) => RawBson::String(s),
            Utf8LossyBson::JavaScriptCode(s) => RawBson::JavaScriptCode(s),
            Utf8LossyBson::JavaScriptCodeWithScope(Utf8LossyJavaScriptCodeWithScope {
                code,
                scope,
            }) => RawBson::JavaScriptCodeWithScope(super::RawJavaScriptCodeWithScope {
                code,
                scope: scope.to_owned(),
            }),
            Utf8LossyBson::Symbol(s) => RawBson::Symbol(s),
            Utf8LossyBson::DbPointer(p) => RawBson::DbPointer(p),
            Utf8LossyBson::RegularExpression(r) => RawBson::RegularExpression(r),
        }
    }
}
