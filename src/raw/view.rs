//! The element view API: structural-only iteration that defers UTF-8 validation to the
//! caller.
//!
//! Construct [`RawElementViewIter`] via
//! [`RawDocument::iter_element_views`](crate::RawDocument::iter_element_views),
//! [`RawDocumentBuf::iter_element_views`](crate::RawDocumentBuf::iter_element_views), or
//! [`RawArray::iter_element_views`](crate::RawArray::iter_element_views).

use std::convert::TryInto;

use crate::{
    raw::{CStr, Error, Result, cstring_bytes, try_to_str, value::RawValue},
    spec::ElementType,
};

use super::{
    RawBsonRef,
    RawDocument,
    iter::{RawElement, get_next_length_at, next_document_len, verify_enough_bytes},
    read_len,
};

/// An iterator over views of the elements in a document. Construct by calling
/// [`RawDocument::iter_element_views`](crate::RawDocument::iter_element_views),
/// [`RawDocumentBuf::iter_element_views`](crate::RawDocumentBuf::iter_element_views), or
/// [`RawArray::iter_element_views`](crate::RawArray::iter_element_views).
///
/// This iterator only validates the structure needed to locate each element (element type
/// tags, lengths, null terminators, and bounds). It does not perform any UTF-8 validation;
/// keys and string-like values are exposed as raw bytes so that the caller can decide how to
/// decode them. An error is returned and iteration ends if an element cannot be located.
///
/// Iteration ends cleanly (i.e. [`Iterator::next`] returns `None`) only when the terminating
/// null byte is reached as the final byte of the document. Any byte of the document's body
/// that is not consumed by an element (including a premature null type tag) produces an
/// error, so a completed iteration accounts for the entire document.
pub struct RawElementViewIter<'a> {
    bytes: &'a [u8],
    offset: usize,
    /// Whether the underlying doc is assumed to be valid or if an error has been encountered.
    /// After an error, all subsequent iterations will return None.
    valid: bool,
    /// The offset in `bytes` of the null byte that terminates the document being iterated.
    /// Iteration ends only when it reaches this offset; elements running past it, or a
    /// missing null byte there, are errors.
    terminator: usize,
}

impl<'a> RawElementViewIter<'a> {
    pub(crate) fn new(doc: &'a RawDocument) -> Self {
        Self {
            bytes: doc.as_bytes(),
            offset: 4,
            valid: true,
            terminator: doc.as_bytes().len().saturating_sub(1),
        }
    }

    /// Calculates the size of the value of an element of `element_type` whose value begins at
    /// `offset`, performing only the structural validation needed to locate the element's
    /// extent (lengths, null terminators, and bounds). No UTF-8 validation is performed.
    fn value_size_at(&self, offset: usize, element_type: ElementType) -> Result<usize> {
        let size = match element_type {
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
            ElementType::String | ElementType::Symbol | ElementType::JavaScriptCode => {
                read_len(&self.bytes[offset..])?
            }
            ElementType::EmbeddedDocument | ElementType::Array => {
                next_document_len(self.bytes, offset)?
            }
            ElementType::Binary => get_next_length_at(self.bytes, offset)? + 4 + 1,
            ElementType::RegularExpression => {
                let pattern = cstring_bytes(&self.bytes[offset..])?;
                let options = cstring_bytes(&self.bytes[offset + pattern.len() + 1..])?;
                pattern.len() + 1 + options.len() + 1
            }
            ElementType::DbPointer => read_len(&self.bytes[offset..])? + 12,
            ElementType::JavaScriptCodeWithScope => get_next_length_at(self.bytes, offset)?,
        };

        verify_enough_bytes(self.bytes, offset, size)?;
        Ok(size)
    }
}

impl<'a> Iterator for RawElementViewIter<'a> {
    type Item = Result<RawElementView<'a>>;

    fn next(&mut self) -> Option<Result<RawElementView<'a>>> {
        if !self.valid {
            return None;
        } else if self.offset > self.terminator || self.offset >= self.bytes.len() {
            self.valid = false;
            return Some(Err(Error::malformed_bytes("iteration overflowed document")));
        } else if self.offset == self.terminator {
            return if self.bytes[self.offset] == 0 {
                None
            } else {
                self.valid = false;
                Some(Err(Error::malformed_bytes("document not null terminated")))
            };
        }

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

        // Locate the key's null terminator without validating it as UTF-8.
        let key_start = self.offset + 1;
        let key_len = match self.bytes[key_start..].iter().position(|&b| b == 0) {
            Some(len) => len,
            None => {
                self.valid = false;
                return Some(Err(Error::malformed_bytes(format!(
                    "expected null terminator for key (at byte offset {})",
                    key_start
                ))));
            }
        };
        let key = &self.bytes[key_start..key_start + key_len];
        let value_offset = key_start + key_len + 1;

        Some(match self.value_size_at(value_offset, element_type) {
            Ok(size) => {
                self.offset = value_offset + size;
                Ok(RawElementView {
                    element_type,
                    key,
                    value: &self.bytes[value_offset..value_offset + size],
                    value_offset,
                })
            }
            Err(error) => {
                self.valid = false;
                Err(error)
            }
        })
    }
}

/// A view of the bytes of an element contained in a [`RawDocument`] or
/// [`RawDocumentBuf`](crate::RawDocumentBuf), yielded by [`RawElementViewIter`]. Neither the
/// key nor the value has been validated as UTF-8.
///
/// Call [`RawElementView::value`] to parse the value (validating all strings as UTF-8), call
/// `RawElementView::value_view` to parse the value's shape while leaving string-like payloads
/// as raw bytes, or convert the view into a [`RawElement`] via its `TryFrom` implementation
/// to validate the key.
///
/// ```
/// # use bson::raw::RawDocument;
/// // a document with an invalid UTF-8 key
/// let doc = RawDocument::from_bytes(b"\x12\0\0\0\x02\xff\0\x06\0\0\0value\0\0")?;
/// let mut iter = doc.iter_element_views();
/// let element = iter.next().unwrap()?;
/// assert_eq!(element.key_bytes(), b"\xff");
/// assert_eq!(element.element_type(), bson::spec::ElementType::String);
/// assert!(iter.next().is_none());
/// # Ok::<(), bson::error::Error>(())
/// ```
#[derive(Clone, Debug)]
pub struct RawElementView<'a> {
    element_type: ElementType,
    key: &'a [u8],
    value: &'a [u8],
    value_offset: usize,
}

impl<'a> RawElementView<'a> {
    /// The type of the element.
    pub fn element_type(&self) -> ElementType {
        self.element_type
    }

    /// The bytes of the document key the element corresponds to, excluding the null
    /// terminator. These bytes are not guaranteed to be valid UTF-8.
    pub fn key_bytes(&self) -> &'a [u8] {
        self.key
    }

    /// The bytes of the element's value, as encoded in the document. This includes any
    /// type-specific framing; for example, the value bytes of a string include its length
    /// prefix and null terminator. These bytes are not guaranteed to contain valid UTF-8.
    pub fn value_bytes(&self) -> &'a [u8] {
        self.value
    }

    /// Parses this element's value into a [`RawBsonRef`] and returns an error if the
    /// underlying bytes are invalid, including if any strings in the value are not valid
    /// UTF-8. The key is not validated.
    pub fn value(&self) -> Result<RawBsonRef<'a>> {
        RawValue::new_at(self.element_type, self.value, self.value_offset).parse()
    }
}

impl<'a> TryFrom<RawElementView<'a>> for RawElement<'a> {
    type Error = Error;

    /// Validates the view's key and converts it into a [`RawElement`]. Returns an error if
    /// the key is not valid UTF-8.
    fn try_from(view: RawElementView<'a>) -> Result<Self> {
        let key: &CStr = try_to_str(view.key)?.try_into()?;
        Ok(Self::new(
            key,
            RawValue::new_at(view.element_type, view.value, view.value_offset),
        ))
    }
}
