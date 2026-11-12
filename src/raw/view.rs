//! The element view API: structural-only iteration that defers UTF-8 validation to the
//! caller.
//!
//! Construct [`RawElementViewIter`] via
//! [`RawDocument::iter_element_views`](crate::RawDocument::iter_element_views),
//! [`RawDocumentBuf::iter_element_views`](crate::RawDocumentBuf::iter_element_views), or
//! [`RawArray::iter_element_views`](crate::RawArray::iter_element_views).

use std::convert::TryInto;

use crate::{
    DateTime,
    Decimal128,
    Timestamp,
    oid::ObjectId,
    raw::{
        CStr,
        Error,
        MIN_CODE_WITH_SCOPE_SIZE,
        Result,
        bool_from_slice,
        cstring_bytes,
        f64_from_slice,
        i32_from_slice,
        i64_from_slice,
        read_lenencode_bytes,
        try_to_str,
        value::RawValue,
    },
    spec::ElementType,
};

use super::{
    RawArray,
    RawBinaryRef,
    RawBsonRef,
    RawDbPointerRef,
    RawDocument,
    RawJavaScriptCodeWithScopeRef,
    RawRegexRef,
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
                let (pattern, options) = regex_payload_at(&self.bytes[offset..])?;
                pattern.len() + 1 + options.len() + 1
            }
            ElementType::DbPointer => read_len(&self.bytes[offset..])? + 12,
            ElementType::JavaScriptCodeWithScope => get_next_length_at(self.bytes, offset)?,
        };

        verify_enough_bytes(self.bytes, offset, size)?;
        Ok(size)
    }
}

/// The pattern and options cstrings of a regular expression value, as raw bytes excluding
/// their null terminators. No UTF-8 validation is performed.
pub(super) fn regex_payload_at(bytes: &[u8]) -> Result<(&[u8], &[u8])> {
    let pattern = cstring_bytes(bytes)?;
    let options = cstring_bytes(&bytes[pattern.len() + 1..])?;
    Ok((pattern, options))
}

/// The namespace bytes (excluding the length prefix and null terminator) and referenced id of
/// a DbPointer value. No UTF-8 validation is performed.
pub(super) fn dbpointer_payload_at(bytes: &[u8]) -> Result<(&[u8], ObjectId)> {
    let namespace = read_lenencode_bytes(bytes)?;
    if bytes.len() < 12 {
        return Err(Error::malformed_bytes("dbpointer too short"));
    }
    let id = ObjectId::from_bytes(
        bytes[bytes.len() - 12..]
            .try_into()
            .map_err(Error::malformed_bytes)?,
    );
    Ok((namespace, id))
}

/// The code bytes (excluding the length prefix and null terminator) and the raw bytes of the
/// scope document of a code-with-scope value. No UTF-8 validation is performed.
pub(super) fn code_with_scope_payload_at(bytes: &[u8]) -> Result<(&[u8], &[u8])> {
    if bytes.len() < MIN_CODE_WITH_SCOPE_SIZE as usize {
        return Err(Error::malformed_bytes("code with scope length too small"));
    }

    let code = read_lenencode_bytes(&bytes[4..])?;
    // the scope follows the code's raw bytes; using the encoded length here would shift the
    // offset whenever the code contains multi-byte sequences
    let scope_start = 4 + 4 + code.len() + 1;
    if scope_start >= bytes.len() {
        return Err(Error::malformed_bytes("code with scope length overrun"));
    }
    Ok((code, &bytes[scope_start..]))
}

/// Parses the shape of a value of kind `element_type` beginning at `bytes` into a
/// [`RawBsonRefView`], performing all type-specific framing (length prefixes, null
/// terminators, and nested extents) but no UTF-8 validation.
pub(crate) fn value_view_at<'a>(
    element_type: ElementType,
    bytes: &'a [u8],
) -> Result<RawBsonRefView<'a>> {
    Ok(match element_type {
        ElementType::Null => RawBsonRefView::Null,
        ElementType::Undefined => RawBsonRefView::Undefined,
        ElementType::MinKey => RawBsonRefView::MinKey,
        ElementType::MaxKey => RawBsonRefView::MaxKey,
        ElementType::ObjectId => RawBsonRefView::ObjectId(ObjectId::parse(bytes)?),
        ElementType::Int32 => RawBsonRefView::Int32(i32_from_slice(bytes)?),
        ElementType::Int64 => RawBsonRefView::Int64(i64_from_slice(bytes)?),
        ElementType::Double => RawBsonRefView::Double(f64_from_slice(bytes)?),
        ElementType::Boolean => {
            RawBsonRefView::Boolean(bool_from_slice(bytes).map_err(Error::malformed_bytes)?)
        }
        ElementType::DateTime => RawBsonRefView::DateTime(DateTime::parse(bytes)?),
        ElementType::Decimal128 => RawBsonRefView::Decimal128(Decimal128::parse(bytes)?),
        ElementType::Timestamp => RawBsonRefView::Timestamp(Timestamp::parse(bytes)?),
        ElementType::Binary => RawBsonRefView::Binary(RawBinaryRef::parse(bytes)?),
        ElementType::String => RawBsonRefView::String(read_lenencode_bytes(bytes)?),
        ElementType::Symbol => RawBsonRefView::Symbol(read_lenencode_bytes(bytes)?),
        ElementType::JavaScriptCode => RawBsonRefView::JavaScriptCode(read_lenencode_bytes(bytes)?),
        ElementType::EmbeddedDocument => RawBsonRefView::Document(RawDocument::from_bytes(bytes)?),
        ElementType::Array => {
            RawBsonRefView::Array(RawArray::from_doc(RawDocument::from_bytes(bytes)?))
        }
        ElementType::RegularExpression => {
            let (pattern, options) = regex_payload_at(bytes)?;
            RawBsonRefView::RegularExpression { pattern, options }
        }
        ElementType::DbPointer => {
            let (namespace, id) = dbpointer_payload_at(bytes)?;
            RawBsonRefView::DbPointer { namespace, id }
        }
        ElementType::JavaScriptCodeWithScope => {
            let (code, scope) = code_with_scope_payload_at(bytes)?;
            RawBsonRefView::JavaScriptCodeWithScope {
                code,
                scope: RawDocument::from_bytes(scope)?,
            }
        }
    })
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
/// [`RawElementView::value_view`] to parse the value's shape while leaving string-like payloads
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
///
/// ```
/// # use bson::raw::{RawBsonRefView, RawDocument};
/// // a document whose string value contains invalid UTF-8
/// let doc = RawDocument::from_bytes(b"\x12\0\0\0\x02a\0\x06\0\0\0h\xffllo\0\0")?;
///
/// let mut decoded = Vec::new();
/// for element in doc.iter_element_views() {
///     let element = element?;
///     match element.value_view()? {
///         RawBsonRefView::String(payload) => {
///             // the caller applies its own decoding policy; the framing has been parsed
///             decoded.push((element.key_bytes().to_vec(), payload.to_vec()));
///         }
///         _ => unreachable!(),
///     }
/// }
/// assert_eq!(decoded, [(b"a".to_vec(), b"h\xffllo".to_vec())]);
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

    /// Parses the shape of this element's value into a [`RawBsonRefView`], in which the
    /// payloads of string-like types ([`ElementType::String`], [`ElementType::Symbol`],
    /// [`ElementType::JavaScriptCode`], [`ElementType::JavaScriptCodeWithScope`],
    /// [`ElementType::DbPointer`], and [`ElementType::RegularExpression`]) are exposed as raw
    /// bytes, leaving the caller to apply its own decoding policy. All type-specific framing
    /// (length prefixes, null terminators, and nested extents) is parsed, and an error is
    /// returned if the underlying bytes are structurally invalid. The key is not validated,
    /// and no part of the value is validated as UTF-8.
    pub fn value_view(&self) -> Result<RawBsonRefView<'a>> {
        value_view_at(self.element_type, self.value)
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

/// A parsed view of a BSON value in which the payloads of string-like types are exposed as
/// raw bytes, leaving the caller to apply its own decoding policy. This is the
/// byte-preserving analog of [`RawBsonRef`]: all other variants are identical, and all
/// type-specific framing has been parsed and validated. Construct by calling
/// [`RawElementView::value_view`].
///
/// Note that `RawBsonRefView`'s [`PartialEq`] implementation is derived, so `Double` compares
/// by `f64` equality, unlike [`RawBsonRef`]'s byte value equality.
///
/// Bytes are borrowed from the source document and are not guaranteed to be valid UTF-8.
#[derive(Clone, Debug, PartialEq)]
pub enum RawBsonRefView<'a> {
    /// 64-bit binary floating point
    Double(f64),
    /// UTF-8 string, as raw bytes excluding the length prefix and null terminator
    String(&'a [u8]),
    /// Array
    Array(&'a RawArray),
    /// Embedded document
    Document(&'a RawDocument),
    /// Boolean value
    Boolean(bool),
    /// Null value
    Null,
    /// Regular expression, with the pattern and options as raw bytes excluding their null
    /// terminators
    RegularExpression {
        /// The pattern bytes
        pattern: &'a [u8],
        /// The options bytes
        options: &'a [u8],
    },
    /// JavaScript code, as raw bytes excluding the length prefix and null terminator
    JavaScriptCode(&'a [u8]),
    /// JavaScript code w/ scope, with the code as raw bytes excluding the length prefix and
    /// null terminator, and the scope document validated for structural validity only
    JavaScriptCodeWithScope {
        /// The code bytes
        code: &'a [u8],
        /// The scope document
        scope: &'a RawDocument,
    },
    /// 32-bit signed integer
    Int32(i32),
    /// 64-bit signed integer
    Int64(i64),
    /// Timestamp
    Timestamp(Timestamp),
    /// Binary data
    Binary(RawBinaryRef<'a>),
    /// [ObjectId](http://dochub.mongodb.org/core/objectids)
    ObjectId(ObjectId),
    /// UTC datetime
    DateTime(DateTime),
    /// [128-bit decimal floating point](https://github.com/mongodb/specifications/blob/master/source/bson-decimal128/decimal128.rst)
    Decimal128(Decimal128),
    /// DBPointer (Deprecated), with the namespace as raw bytes excluding the length prefix
    /// and null terminator
    DbPointer {
        /// The namespace bytes
        namespace: &'a [u8],
        /// The referenced id
        id: ObjectId,
    },
    /// Symbol (Deprecated), as raw bytes excluding the length prefix and null terminator
    Symbol(&'a [u8]),
    /// Undefined value (Deprecated)
    Undefined,
    /// Max key
    MaxKey,
    /// Min key
    MinKey,
}

impl<'a> RawBsonRefView<'a> {
    /// Applies the crate's strict decoding policy to this view: every string-like payload is
    /// validated as UTF-8 (and, for regular expressions, as a valid C string), producing the
    /// corresponding [`RawBsonRef`]. All other variants are passed through unchanged. Returns
    /// an error if any payload is not valid UTF-8.
    pub(crate) fn into_validated(self) -> Result<RawBsonRef<'a>> {
        Ok(match self {
            Self::String(bytes) => RawBsonRef::String(try_to_str(bytes)?),
            Self::Symbol(bytes) => RawBsonRef::Symbol(try_to_str(bytes)?),
            Self::JavaScriptCode(bytes) => RawBsonRef::JavaScriptCode(try_to_str(bytes)?),
            Self::RegularExpression { pattern, options } => {
                RawBsonRef::RegularExpression(RawRegexRef {
                    pattern: try_to_str(pattern)?.try_into()?,
                    options: try_to_str(options)?.try_into()?,
                })
            }
            Self::DbPointer { namespace, id } => RawBsonRef::DbPointer(RawDbPointerRef {
                namespace: try_to_str(namespace)?,
                id,
            }),
            Self::JavaScriptCodeWithScope { code, scope } => {
                RawBsonRef::JavaScriptCodeWithScope(RawJavaScriptCodeWithScopeRef {
                    code: try_to_str(code)?,
                    scope,
                })
            }
            Self::Double(d) => RawBsonRef::Double(d),
            Self::Array(v) => RawBsonRef::Array(v),
            Self::Document(v) => RawBsonRef::Document(v),
            Self::Boolean(v) => RawBsonRef::Boolean(v),
            Self::Null => RawBsonRef::Null,
            Self::Int32(v) => RawBsonRef::Int32(v),
            Self::Int64(v) => RawBsonRef::Int64(v),
            Self::Timestamp(v) => RawBsonRef::Timestamp(v),
            Self::Binary(v) => RawBsonRef::Binary(v),
            Self::ObjectId(v) => RawBsonRef::ObjectId(v),
            Self::DateTime(v) => RawBsonRef::DateTime(v),
            Self::Decimal128(v) => RawBsonRef::Decimal128(v),
            Self::Undefined => RawBsonRef::Undefined,
            Self::MaxKey => RawBsonRef::MaxKey,
            Self::MinKey => RawBsonRef::MinKey,
        })
    }
}
