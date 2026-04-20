use std::convert::TryInto;

use crate::{
    Bson,
    DateTime,
    Decimal128,
    RawArray,
    RawBinaryRef,
    RawBson,
    RawDbPointerRef,
    RawJavaScriptCodeWithScopeRef,
    RawRegexRef,
    Timestamp,
    oid::ObjectId,
    raw::{
        CStr,
        Error,
        MIN_BSON_DOCUMENT_SIZE,
        MIN_CODE_WITH_SCOPE_SIZE,
        Result,
        cstring_bytes,
        read_cstring,
    },
    spec::{BinarySubtype, ElementType},
};

use super::{
    RawBsonRef,
    RawDocument,
    bool_from_slice,
    checked_add,
    f64_from_slice,
    i32_from_slice,
    i64_from_slice,
    read_len,
    read_lenencode,
    read_lenencode_bytes,
    try_to_str,
};

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
}

impl<'a> RawIter<'a> {
    pub(crate) fn new(doc: &'a RawDocument) -> Self {
        Self::new_unchecked(doc.as_bytes(), 4)
    }

    pub(crate) fn new_unchecked(bytes: &'a [u8], offset: usize) -> Self {
        Self {
            bytes,
            offset,
            valid: true,
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
    kind: ElementType,
    bytes: &'a [u8],
    start_at: usize,
    size: usize,
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
    pub(crate) fn toplevel(bytes: &'a [u8]) -> Result<Self> {
        use crate::raw::cstr;

        let doc = RawDocument::from_bytes(bytes)?;
        Ok(Self {
            key: cstr!("TOPLEVEL"),
            kind: ElementType::EmbeddedDocument,
            doc,
            start_at: 0,
            size: doc.as_bytes().len(),
        })
    }

    /// The size of the element.
    pub fn size(&self) -> usize {
        self.size
    }

    /// The document key the element corresponds to.
    pub fn key(&self) -> &'a CStr {
        self.key
    }

    /// The type of the element.
    pub fn element_type(&self) -> ElementType {
        self.kind
    }

    /// Parses this element into a [`RawBsonRef`] and returns an error if the underlying bytes are
    /// invalid.
    pub fn value(&self) -> Result<RawBsonRef<'a>> {
        Ok(match self.kind {
            ElementType::Null => RawBsonRef::Null,
            ElementType::Undefined => RawBsonRef::Undefined,
            ElementType::MinKey => RawBsonRef::MinKey,
            ElementType::MaxKey => RawBsonRef::MaxKey,
            ElementType::ObjectId => RawBsonRef::ObjectId(self.get_oid_at(self.start_at)?),
            ElementType::Int32 => RawBsonRef::Int32(i32_from_slice(self.value_bytes())?),
            ElementType::Int64 => RawBsonRef::Int64(i64_from_slice(self.value_bytes())?),
            ElementType::Double => RawBsonRef::Double(f64_from_slice(self.value_bytes())?),
            ElementType::String => RawBsonRef::String(self.read_str()?),
            ElementType::EmbeddedDocument => {
                RawBsonRef::Document(RawDocument::from_bytes(self.value_bytes())?)
            }
            ElementType::Array => RawBsonRef::Array(RawArray::from_doc(RawDocument::from_bytes(
                self.value_bytes(),
            )?)),
            ElementType::Boolean => RawBsonRef::Boolean(
                bool_from_slice(self.value_bytes()).map_err(|e| self.malformed_error(e))?,
            ),
            ElementType::DateTime => {
                RawBsonRef::DateTime(DateTime::from_millis(i64_from_slice(self.value_bytes())?))
            }
            ElementType::Decimal128 => RawBsonRef::Decimal128(Decimal128::from_bytes(
                self.value_bytes()
                    .try_into()
                    .map_err(|e| self.malformed_error(e))?,
            )),
            ElementType::JavaScriptCode => RawBsonRef::JavaScriptCode(self.read_str()?),
            ElementType::Symbol => RawBsonRef::Symbol(self.read_str()?),
            ElementType::DbPointer => RawBsonRef::DbPointer(RawDbPointerRef {
                namespace: read_lenencode(self.value_bytes())?,
                id: self.get_oid_at(self.start_at + (self.size - 12))?,
            }),
            ElementType::RegularExpression => {
                let pattern = read_cstring(&self.bytes[self.start_at..])?;
                RawBsonRef::RegularExpression(RawRegexRef {
                    pattern,
                    options: read_cstring(&self.bytes[self.start_at + pattern.len() + 1..])?,
                })
            }
            ElementType::Timestamp => RawBsonRef::Timestamp({
                let bytes: [u8; 8] = self.value_bytes()[0..8]
                    .try_into()
                    .map_err(|e| self.malformed_error(e))?;
                Timestamp::from_le_bytes(bytes)
            }),
            ElementType::Binary => {
                let len = self.size.checked_sub(4 + 1).ok_or_else(|| {
                    self.malformed_error(format!("length exceeds maximum: {}", self.size))
                })?;

                let data_start = self.start_at + 4 + 1;

                if self.size >= i32::MAX as usize {
                    return Err(
                        self.malformed_error(format!("binary length exceeds maximum: {}", len))
                    );
                }

                let subtype = BinarySubtype::from(self.bytes[self.start_at + 4]);
                let data = match subtype {
                    BinarySubtype::BinaryOld => {
                        if len < 4 {
                            return Err(self.malformed_error(
                                "old binary subtype has no inner declared length",
                            ));
                        }
                        let oldlength = i32_from_slice(&self.bytes[data_start..])? as usize;
                        if checked_add(oldlength, 4)? != len {
                            return Err(self.malformed_error(
                                "old binary subtype has wrong inner declared length",
                            ));
                        }
                        self.slice_bounds(data_start + 4, len - 4)
                    }
                    _ => self.slice_bounds(data_start, len),
                };
                RawBsonRef::Binary(RawBinaryRef {
                    subtype,
                    bytes: data,
                })
            }
            ElementType::JavaScriptCodeWithScope => {
                if self.size < MIN_CODE_WITH_SCOPE_SIZE as usize {
                    return Err(self.malformed_error("code with scope length too small"));
                }

                let slice = self.value_bytes();
                let code = read_lenencode(&slice[4..])?;
                let scope_start = 4 + 4 + code.len() + 1;
                let scope = RawDocument::from_bytes(&slice[scope_start..])?;

                RawBsonRef::JavaScriptCodeWithScope(RawJavaScriptCodeWithScopeRef { code, scope })
            }
        })
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
        Ok(Some(match self.kind {
            ElementType::String => Utf8LossyBson::String(self.read_utf8_lossy()),
            ElementType::JavaScriptCode => Utf8LossyBson::JavaScriptCode(self.read_utf8_lossy()),
            ElementType::JavaScriptCodeWithScope => {
                if self.size < MIN_CODE_WITH_SCOPE_SIZE as usize {
                    return Err(self.malformed_error("code with scope length too small"));
                }

                let slice = self.value_bytes();
                let code = String::from_utf8_lossy(read_lenencode_bytes(&slice[4..])?).into_owned();
                let scope_start = 4 + 4 + code.len() + 1;
                if scope_start >= slice.len() {
                    return Err(self.malformed_error("code with scope length overrun"));
                }
                let scope = RawDocument::from_bytes(&slice[scope_start..])?;

                Utf8LossyBson::JavaScriptCodeWithScope(Utf8LossyJavaScriptCodeWithScope {
                    code,
                    scope,
                })
            }
            ElementType::Symbol => Utf8LossyBson::Symbol(self.read_utf8_lossy()),
            ElementType::DbPointer => Utf8LossyBson::DbPointer(crate::DbPointer {
                namespace: String::from_utf8_lossy(read_lenencode_bytes(self.value_bytes())?)
                    .into_owned(),
                id: self.get_oid_at(self.start_at + (self.size - 12))?,
            }),
            ElementType::RegularExpression => {
                let pattern = String::from_utf8_lossy(cstring_bytes(&self.bytes[self.start_at..])?)
                    .into_owned();
                let pattern_len = pattern.len();
                Utf8LossyBson::RegularExpression(crate::Regex {
                    pattern: pattern.try_into()?,
                    options: String::from_utf8_lossy(cstring_bytes(
                        &self.bytes[self.start_at + pattern_len + 1..],
                    )?)
                    .into_owned()
                    .try_into()?,
                })
            }
            _ => return Ok(None),
        }))
    }

    fn malformed_error(&self, e: impl ToString) -> Error {
        Error::malformed_bytes(e).with_key(self.key.as_str())
    }

    /// The raw bytes of the value.
    pub fn value_bytes(&self) -> &'a [u8] {
        self.slice_bounds(self.start_at, self.size)
    }

    fn slice_bounds(&self, start_at: usize, size: usize) -> &'a [u8] {
        &self.bytes[start_at..(start_at + size)]
    }

    fn str_bytes(&self) -> &'a [u8] {
        self.slice_bounds(self.start_at + 4, self.size - 4 - 1)
    }

    fn read_str(&self) -> Result<&'a str> {
        try_to_str(self.str_bytes())
    }

    fn read_utf8_lossy(&self) -> String {
        String::from_utf8_lossy(self.str_bytes()).into_owned()
    }

    fn get_oid_at(&self, start_at: usize) -> Result<ObjectId> {
        Ok(ObjectId::from_bytes(
            self.bytes[start_at..(start_at + 12)]
                .try_into()
                .map_err(|e| Error::malformed_bytes(e).with_key(self.key.as_str()))?,
        ))
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
        } else if self.offset == self.bytes.len() - 1 {
            if self.bytes[self.offset] == 0 {
                // end of document marker
                return None;
            } else {
                self.valid = false;
                return Some(Err(Error::malformed_bytes("document not null terminated")));
            }
        } else if self.offset >= self.bytes.len() {
            self.valid = false;
            return Some(Err(Error::malformed_bytes("iteration overflowed document")));
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
                kind,
                bytes: self.bytes,
                start_at: offset,
                size,
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
