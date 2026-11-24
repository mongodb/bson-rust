use std::convert::TryInto;

use crate::{
    Bson,
    RawBson,
    raw::{
        CStr,
        Error,
        MIN_BSON_DOCUMENT_SIZE,
        Result,
        cstring_at,
        read_cstring_with_max_len,
        try_to_str,
        value::RawValue,
    },
    spec::ElementType,
};

use super::{
    RawBsonRef,
    RawDocument,
    checked_add,
    i32_from_slice,
    read_len,
    view::{RawElementView, regex_payload_at},
};

/// An iterator over the key-value pairs in a document. Construct by calling [`RawDocument::iter`]
/// or [`RawDocumentBuf::iter`](crate::RawDocumentBuf::iter).
pub struct Iter<'a> {
    inner: RawIter<'a>,
}

impl<'a> Iter<'a> {
    pub(crate) fn new(doc: &'a RawDocument) -> Self {
        Iter {
            inner: RawIter::new(doc),
        }
    }

    /// The maximum number of bytes the iterator should parse when searching for the null-terminator
    /// for a cstring.
    #[cfg(feature = "sfp-internal")]
    #[doc(hidden)]
    pub fn max_cstr_parse_len(mut self, len: impl Into<Option<usize>>) -> Self {
        self.inner = self.inner.max_cstr_parse_len(len);
        self
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

/// An iterator over the elements in a document. Construct by calling [`RawDocument::iter_elements`]
/// or [`RawDocumentBuf::iter_elements`](crate::RawDocumentBuf::iter_elements).
pub struct RawIter<'a> {
    bytes: &'a [u8],
    offset: usize,
    max_cstr_parse_len: Option<usize>,
    /// Whether the underlying doc is assumed to be valid or if an error has been encountered.
    /// After an error, all subsequent iterations will return None.
    valid: bool,

    /// The offset in `bytes` of the null byte that terminates the document being iterated.
    /// Iteration ends only when it reaches this offset; elements running past it, or a missing
    /// null byte there, are errors.
    terminator: usize,
}

impl<'a> RawIter<'a> {
    pub(crate) fn new(doc: &'a RawDocument) -> Self {
        Self {
            bytes: doc.as_bytes(),
            offset: 4,
            max_cstr_parse_len: None,
            valid: true,
            terminator: doc.as_bytes().len().saturating_sub(1),
        }
    }

    /// Iterate over the elements of a document embedded in `bytes`, starting at `offset` and
    /// ending at the document's null terminator at `terminator`.
    #[cfg(feature = "facet-unstable")]
    pub(crate) fn new_unchecked(bytes: &'a [u8], offset: usize, terminator: usize) -> Self {
        Self {
            bytes,
            offset,
            max_cstr_parse_len: None,
            valid: true,
            terminator,
        }
    }

    /// The maximum number of bytes the iterator should parse when searching for the null-terminator
    /// for a cstring.
    #[cfg(feature = "sfp-internal")]
    #[doc(hidden)]
    pub fn max_cstr_parse_len(mut self, len: impl Into<Option<usize>>) -> Self {
        self.max_cstr_parse_len = len.into();
        self
    }

    #[cfg(feature = "facet-unstable")]
    pub(crate) fn next_document_len(&self, starting_at: usize) -> Result<usize> {
        next_document_len(self.bytes, starting_at)
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
    pub(crate) fn new(key: &'a CStr, value: RawValue<'a>) -> Self {
        Self { key, value }
    }

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
        self.value.bytes().len()
    }

    /// The document key the element corresponds to.
    pub fn key(&self) -> &'a CStr {
        self.key
    }

    /// The type of the element.
    pub fn element_type(&self) -> ElementType {
        self.value.kind()
    }

    /// Parses this element into a [`RawBsonRef`] and returns an error if the underlying bytes are
    /// invalid.
    pub fn value(&self) -> Result<RawBsonRef<'a>> {
        self.value
            .parse()
            .map_err(|e| e.with_key(self.key.as_str()))
    }

    #[cfg(any(feature = "serde", feature = "facet-unstable"))]
    pub(crate) fn value_raw(&self) -> &RawValue<'a> {
        &self.value
    }

    /// Parses this element into [`RawBson`], replacing any invalid UTF-8 in the element's
    /// string-like payloads with the Unicode replacement character. Nested documents and
    /// arrays are returned unvalidated. Returns an error if the underlying bytes are invalid.
    pub fn value_utf8_lossy(&self) -> Result<RawBson> {
        match self.value_utf8_lossy_inner()? {
            Some(v) => Ok(v.into()),
            None => Ok(self.value()?.into()),
        }
    }

    pub(crate) fn value_utf8_lossy_inner(&self) -> Result<Option<Utf8LossyBson<'a>>> {
        // returns None for non-string-like kinds without parsing them, leaving the strict
        // fallback to the caller.
        self.value
            .parse_utf8_lossy()
            .map_err(|e| e.with_key(self.key.as_str()))
    }
}

impl<'a> RawIter<'a> {
    /// The shared walk behind [`Iterator::next`] and [`RawIter::next_view`]: framing check,
    /// tag parse, and key scan, returning the element's type, raw key bytes, and the offset
    /// its value starts at. `key_bound` bounds the key's cstring scan. This step does not
    /// validate UTF-8.
    fn next_element_header(
        &mut self,
        key_bound: Option<usize>,
    ) -> Option<Result<(ElementType, &'a [u8], usize)>> {
        if !self.valid {
            return None;
        }
        match framing_status(self.bytes, self.offset, self.terminator) {
            FramingStatus::End => None,
            FramingStatus::Malformed(error) => {
                self.valid = false;
                Some(Err(error))
            }
            FramingStatus::Element => {
                let element_type = match ElementType::from(self.bytes[self.offset]) {
                    Some(et) => et,
                    None => {
                        self.valid = false;
                        return Some(Err(Error::malformed_bytes(format!(
                            "invalid tag: {} (at byte offset {})",
                            self.bytes[self.offset], self.offset
                        ))));
                    }
                };
                match cstring_at(self.bytes, self.offset + 1, key_bound) {
                    Ok((key, value_offset)) => Some(Ok((element_type, key, value_offset))),
                    Err(error) => {
                        self.valid = false;
                        Some(Err(error))
                    }
                }
            }
        }
    }

    /// Sizes the value located by [`next_element_header`] and advances past the element.
    /// `size_bound` bounds cstring scans during sizing. The view path passes `None` because
    /// the bounded arm UTF-8-validates regex cstrings. View iteration must not validate
    /// UTF-8.
    fn finish_element(
        &mut self,
        value_offset: usize,
        element_type: ElementType,
        size_bound: Option<usize>,
    ) -> Result<usize> {
        match element_size_at(self.bytes, value_offset, element_type, size_bound) {
            Ok(size) => {
                self.offset = value_offset + size;
                Ok(size)
            }
            Err(error) => {
                self.valid = false;
                Err(error)
            }
        }
    }

    /// Advances the iterator and returns a view of the next element's unvalidated bytes.
    /// The view exposes the key and any string-like value payloads as raw bytes for the
    /// caller to decode. Shares the iterator's cursor with [`Iterator::next`], so the two
    /// calls may be mixed. The iterator's `max_cstr_parse_len` bound applies to the key
    /// scan only.
    ///
    /// ```
    /// # use bson::raw::RawDocument;
    /// // a document with an invalid UTF-8 key.
    /// let doc = RawDocument::from_bytes(b"\x12\0\0\0\x02\xff\0\x06\0\0\0value\0\0")?;
    /// let mut iter = doc.iter_elements();
    /// let element = iter.next_view().unwrap()?;
    /// assert_eq!(element.key_bytes(), b"\xff");
    /// assert!(iter.next_view().is_none());
    /// # Ok::<(), bson::error::Error>(())
    /// ```
    pub fn next_view(&mut self) -> Option<Result<RawElementView<'a>>> {
        let (element_type, key, value_offset) =
            match self.next_element_header(self.max_cstr_parse_len) {
                Some(Ok(parts)) => parts,
                Some(Err(error)) => return Some(Err(error)),
                None => return None,
            };
        Some(
            match self.finish_element(value_offset, element_type, None) {
                Ok(size) => Ok(RawElementView::new(
                    element_type,
                    key,
                    &self.bytes[value_offset..value_offset + size],
                    value_offset,
                )),
                Err(error) => Err(error),
            },
        )
    }
}

impl<'a> Iterator for RawIter<'a> {
    type Item = Result<RawElement<'a>>;

    fn next(&mut self) -> Option<Result<RawElement<'a>>> {
        let (element_type, key_bytes, value_offset) =
            match self.next_element_header(self.max_cstr_parse_len) {
                Some(Ok(parts)) => parts,
                Some(Err(error)) => return Some(Err(error)),
                None => return None,
            };
        let key: &CStr = match try_to_str(key_bytes).and_then(|s| s.try_into()) {
            Ok(key) => key,
            Err(error) => {
                self.valid = false;
                return Some(Err(error));
            }
        };
        Some(
            match self.finish_element(value_offset, element_type, self.max_cstr_parse_len) {
                Ok(size) => Ok(RawElement::new(
                    key,
                    RawValue::new_at(
                        element_type,
                        &self.bytes[value_offset..value_offset + size],
                        value_offset,
                    ),
                )),
                Err(error) => Err(error.with_key(key.as_str())),
            },
        )
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

/// The position of an element iterator's offset within a document's framing.
pub(super) enum FramingStatus {
    /// The offset is inside the document body.
    Element,
    /// The offset is the document's terminating null byte, so iteration ends cleanly.
    End,
    /// The framing is malformed at the offset.
    Malformed(Error),
}

/// Classifies `offset` within a document's framing.
pub(super) fn framing_status(bytes: &[u8], offset: usize, terminator: usize) -> FramingStatus {
    if offset > terminator || offset >= bytes.len() {
        FramingStatus::Malformed(Error::malformed_bytes("iteration overflowed document"))
    } else if offset == terminator {
        if bytes[offset] == 0 {
            FramingStatus::End
        } else {
            FramingStatus::Malformed(Error::malformed_bytes("document not null terminated"))
        }
    } else {
        FramingStatus::Element
    }
}

/// Calculates the size of the value of an element of `element_type` whose value begins at
/// `offset` in `bytes`, validating only the structure needed to locate the element's extent
/// (lengths, null terminators, and bounds). No UTF-8 validation is performed, except that a
/// regular expression's cstrings are UTF-8-validated when `max_cstr_parse_len` is set, as
/// in the strict iterator.
pub(super) fn element_size_at(
    bytes: &[u8],
    offset: usize,
    element_type: ElementType,
    max_cstr_parse_len: Option<usize>,
) -> Result<usize> {
    let size = match element_type {
        ElementType::Boolean => 1,
        ElementType::Int32 => 4,
        ElementType::Int64 => 8,
        ElementType::Double => 8,
        ElementType::DateTime => 8,
        ElementType::Timestamp => 8,
        ElementType::ObjectId => 12,
        ElementType::Decimal128 => 16,
        ElementType::Null | ElementType::Undefined | ElementType::MinKey | ElementType::MaxKey => 0,
        ElementType::String | ElementType::Symbol | ElementType::JavaScriptCode => {
            read_len(&bytes[offset..])?
        }
        ElementType::EmbeddedDocument | ElementType::Array => next_document_len(bytes, offset)?,
        ElementType::Binary => get_next_length_at(bytes, offset)? + 4 + 1,
        ElementType::RegularExpression => match max_cstr_parse_len {
            Some(_) => {
                let pattern = read_cstring_with_max_len(&bytes[offset..], max_cstr_parse_len)?;
                let options = read_cstring_with_max_len(
                    &bytes[offset + pattern.len() + 1..],
                    max_cstr_parse_len,
                )?;
                pattern.len() + 1 + options.len() + 1
            }
            None => {
                let (pattern, options) = regex_payload_at(&bytes[offset..])?;
                pattern.len() + 1 + options.len() + 1
            }
        },
        ElementType::DbPointer => read_len(&bytes[offset..])? + 12,
        ElementType::JavaScriptCodeWithScope => get_next_length_at(bytes, offset)?,
    };

    verify_enough_bytes(bytes, offset, size)?;
    Ok(size)
}

// Shared framing helpers used by both RawIter and the view iterator in crate::raw::view.
// Each validates only the structure needed to locate an extent. None validate UTF-8.

/// Returns an error if `bytes` does not contain `num_bytes` starting at `start`.
pub(super) fn verify_enough_bytes(bytes: &[u8], start: usize, num_bytes: usize) -> Result<()> {
    let end = checked_add(start, num_bytes)?;
    if bytes.get(start..end).is_none() {
        return Err(Error::malformed_bytes(format!(
            "length exceeds remaining length of buffer: {} vs {} (at byte offset {})",
            num_bytes,
            bytes.len().saturating_sub(start),
            start
        )));
    }
    Ok(())
}

pub(super) fn get_next_length_at(bytes: &[u8], start_at: usize) -> Result<usize> {
    let len = i32_from_slice(&bytes[start_at..])?;
    if len < 0 {
        Err(Error::malformed_bytes("lengths can't be negative"))
    } else {
        Ok(len as usize)
    }
}

pub(super) fn next_document_len(bytes: &[u8], starting_at: usize) -> Result<usize> {
    verify_enough_bytes(bytes, starting_at, MIN_BSON_DOCUMENT_SIZE as usize)?;
    let size = i32_from_slice(&bytes[starting_at..])? as usize;

    if size < MIN_BSON_DOCUMENT_SIZE as usize {
        return Err(Error::malformed_bytes(format!(
            "document too small: {} bytes (at byte offset {})",
            size, starting_at
        )));
    }

    verify_enough_bytes(bytes, starting_at, size)?;

    if bytes[starting_at + size - 1] != 0 {
        return Err(Error::malformed_bytes(format!(
            "not null terminated (at byte offset {})",
            starting_at + size - 1
        )));
    }
    Ok(size)
}
