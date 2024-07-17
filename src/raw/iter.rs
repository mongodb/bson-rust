use std::convert::TryInto;

use crate::{
    de::{read_bool, MIN_BSON_DOCUMENT_SIZE, MIN_CODE_WITH_SCOPE_SIZE},
    oid::ObjectId,
    raw::{Error, ErrorKind, Result},
    spec::{BinarySubtype, ElementType},
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
};

use super::{
    checked_add,
    error::try_with_key,
    f64_from_slice,
    i32_from_slice,
    i64_from_slice,
    read_len,
    read_lenencode,
    read_lenencode_bytes,
    try_to_str,
    RawBsonRef,
    RawDocument,
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
    type Item = Result<(&'a str, RawBsonRef<'a>)>;

    fn next(&mut self) -> Option<Result<(&'a str, RawBsonRef<'a>)>> {
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
    doc: &'a RawDocument,
    offset: usize,

    /// Whether the underlying doc is assumed to be valid or if an error has been encountered.
    /// After an error, all subsequent iterations will return None.
    valid: bool,
}

impl<'a> RawIter<'a> {
    pub(crate) fn new(doc: &'a RawDocument) -> Self {
        Self {
            doc,
            offset: 4,
            valid: true,
        }
    }

    fn verify_enough_bytes(&self, start: usize, num_bytes: usize) -> Result<()> {
        let end = checked_add(start, num_bytes)?;
        if self.doc.as_bytes().get(start..end).is_none() {
            return Err(Error::new_without_key(ErrorKind::new_malformed(format!(
                "length exceeds remaining length of buffer: {} vs {}",
                num_bytes,
                self.doc.as_bytes().len() - start
            ))));
        }
        Ok(())
    }

    fn next_document_len(&self, starting_at: usize) -> Result<usize> {
        self.verify_enough_bytes(starting_at, MIN_BSON_DOCUMENT_SIZE as usize)?;
        let size = i32_from_slice(&self.doc.as_bytes()[starting_at..])? as usize;

        if size < MIN_BSON_DOCUMENT_SIZE as usize {
            return Err(Error::new_without_key(ErrorKind::new_malformed(format!(
                "document too small: {} bytes",
                size
            ))));
        }

        self.verify_enough_bytes(starting_at, size)?;

        if self.doc.as_bytes()[starting_at + size - 1] != 0 {
            return Err(Error::new_without_key(ErrorKind::new_malformed(
                "not null terminated",
            )));
        }
        Ok(size)
    }
}

#[derive(Clone)]
pub struct RawElement<'a> {
    key: &'a str,
    kind: ElementType,
    doc: &'a RawDocument,
    start_at: usize,
    size: usize,
}

impl<'a> TryInto<RawBsonRef<'a>> for RawElement<'a> {
    type Error = Error;

    fn try_into(self) -> Result<RawBsonRef<'a>> {
        self.value()
    }
}

impl<'a> TryInto<RawBson> for RawElement<'a> {
    type Error = Error;

    fn try_into(self) -> Result<RawBson> {
        Ok(self.value()?.to_raw_bson())
    }
}

impl<'a> TryInto<Bson> for RawElement<'a> {
    type Error = Error;

    fn try_into(self) -> Result<Bson> {
        self.value()?.to_raw_bson().try_into()
    }
}

#[allow(clippy::len_without_is_empty)]
impl<'a> RawElement<'a> {
    pub(crate) fn toplevel(bytes: &'a [u8]) -> Result<Self> {
        let doc = RawDocument::from_bytes(bytes)?;
        Ok(Self {
            key: "TOPLEVEL",
            kind: ElementType::EmbeddedDocument,
            doc,
            start_at: 0,
            size: doc.as_bytes().len(),
        })
    }
    pub fn len(&self) -> usize {
        self.size
    }

    pub fn key(&self) -> &'a str {
        self.key
    }

    pub fn element_type(&self) -> ElementType {
        self.kind
    }

    pub fn value(&self) -> Result<RawBsonRef<'a>> {
        Ok(match self.kind {
            ElementType::Null => RawBsonRef::Null,
            ElementType::Undefined => RawBsonRef::Undefined,
            ElementType::MinKey => RawBsonRef::MinKey,
            ElementType::MaxKey => RawBsonRef::MaxKey,
            ElementType::ObjectId => RawBsonRef::ObjectId(self.get_oid_at(self.start_at)?),
            ElementType::Int32 => RawBsonRef::Int32(i32_from_slice(self.slice())?),
            ElementType::Int64 => RawBsonRef::Int64(i64_from_slice(self.slice())?),
            ElementType::Double => RawBsonRef::Double(f64_from_slice(self.slice())?),
            ElementType::String => RawBsonRef::String(self.read_str()?),
            ElementType::EmbeddedDocument => {
                RawBsonRef::Document(RawDocument::from_bytes(self.slice())?)
            }
            ElementType::Array => {
                RawBsonRef::Array(RawArray::from_doc(RawDocument::from_bytes(self.slice())?))
            }
            ElementType::Boolean => {
                RawBsonRef::Boolean(read_bool(self.slice()).map_err(|e| self.malformed_error(e))?)
            }
            ElementType::DateTime => {
                RawBsonRef::DateTime(DateTime::from_millis(i64_from_slice(self.slice())?))
            }
            ElementType::Decimal128 => RawBsonRef::Decimal128(Decimal128::from_bytes(
                self.slice()
                    .try_into()
                    .map_err(|e| self.malformed_error(e))?,
            )),
            ElementType::JavaScriptCode => RawBsonRef::JavaScriptCode(self.read_str()?),
            ElementType::Symbol => RawBsonRef::Symbol(self.read_str()?),
            ElementType::DbPointer => RawBsonRef::DbPointer(RawDbPointerRef {
                namespace: read_lenencode(self.slice())?,
                id: self.get_oid_at(self.start_at + (self.size - 12))?,
            }),
            ElementType::RegularExpression => {
                let pattern = self.doc.read_cstring_at(self.start_at)?;
                RawBsonRef::RegularExpression(RawRegexRef {
                    pattern,
                    options: self
                        .doc
                        .read_cstring_at(self.start_at + pattern.len() + 1)?,
                })
            }
            ElementType::Timestamp => RawBsonRef::Timestamp(
                Timestamp::from_reader(self.slice()).map_err(|e| self.malformed_error(e))?,
            ),
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

                let subtype = BinarySubtype::from(self.doc.as_bytes()[self.start_at + 4]);
                let data = match subtype {
                    BinarySubtype::BinaryOld => {
                        if len < 4 {
                            return Err(self.malformed_error(
                                "old binary subtype has no inner declared length",
                            ));
                        }
                        let oldlength =
                            i32_from_slice(&self.doc.as_bytes()[data_start..])? as usize;
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

                let slice = self.slice();
                let code = read_lenencode(&slice[4..])?;
                let scope_start = 4 + 4 + code.len() + 1;
                let scope = RawDocument::from_bytes(&slice[scope_start..])?;

                RawBsonRef::JavaScriptCodeWithScope(RawJavaScriptCodeWithScopeRef { code, scope })
            }
        })
    }

    pub(crate) fn value_utf8_lossy(&self) -> Result<Option<Utf8LossyBson<'a>>> {
        Ok(Some(match self.kind {
            ElementType::String => Utf8LossyBson::String(self.read_utf8_lossy()),
            ElementType::JavaScriptCode => Utf8LossyBson::JavaScriptCode(self.read_utf8_lossy()),
            ElementType::JavaScriptCodeWithScope => {
                if self.size < MIN_CODE_WITH_SCOPE_SIZE as usize {
                    return Err(self.malformed_error("code with scope length too small"));
                }

                let slice = self.slice();
                let code = String::from_utf8_lossy(read_lenencode_bytes(&slice[4..])?).into_owned();
                let scope_start = 4 + 4 + code.len() + 1;
                let scope = RawDocument::from_bytes(&slice[scope_start..])?;

                Utf8LossyBson::JavaScriptCodeWithScope(Utf8LossyJavaScriptCodeWithScope {
                    code,
                    scope,
                })
            }
            ElementType::Symbol => Utf8LossyBson::Symbol(self.read_utf8_lossy()),
            ElementType::DbPointer => Utf8LossyBson::DbPointer(crate::DbPointer {
                namespace: String::from_utf8_lossy(read_lenencode_bytes(self.slice())?)
                    .into_owned(),
                id: self.get_oid_at(self.start_at + (self.size - 12))?,
            }),
            ElementType::RegularExpression => {
                let pattern =
                    String::from_utf8_lossy(self.doc.cstring_bytes_at(self.start_at)?).into_owned();
                let pattern_len = pattern.len();
                Utf8LossyBson::RegularExpression(crate::Regex {
                    pattern,
                    options: String::from_utf8_lossy(
                        self.doc.cstring_bytes_at(self.start_at + pattern_len + 1)?,
                    )
                    .into_owned(),
                })
            }
            _ => return Ok(None),
        }))
    }

    fn malformed_error(&self, e: impl ToString) -> Error {
        Error::new_with_key(self.key, ErrorKind::new_malformed(e))
    }

    fn slice(&self) -> &'a [u8] {
        self.slice_bounds(self.start_at, self.size)
    }

    fn slice_bounds(&self, start_at: usize, size: usize) -> &'a [u8] {
        &self.doc.as_bytes()[start_at..(start_at + size)]
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
            self.doc.as_bytes()[start_at..(start_at + 12)]
                .try_into()
                .map_err(|e| Error::new_with_key(self.key, ErrorKind::new_malformed(e)))?,
        ))
    }
}

impl<'a> RawIter<'a> {
    fn get_next_length_at(&self, start_at: usize) -> Result<usize> {
        let len = i32_from_slice(&self.doc.as_bytes()[start_at..])?;
        if len < 0 {
            Err(Error::new_without_key(ErrorKind::new_malformed(
                "lengths can't be negative",
            )))
        } else {
            Ok(len as usize)
        }
    }
}

impl<'a> Iterator for RawIter<'a> {
    type Item = Result<RawElement<'a>>;

    fn next(&mut self) -> Option<Result<RawElement<'a>>> {
        if !self.valid {
            return None;
        } else if self.offset == self.doc.as_bytes().len() - 1 {
            if self.doc.as_bytes()[self.offset] == 0 {
                // end of document marker
                return None;
            } else {
                self.valid = false;
                return Some(Err(Error::new_without_key(ErrorKind::new_malformed(
                    "document not null terminated",
                ))));
            }
        } else if self.offset >= self.doc.as_bytes().len() {
            self.valid = false;
            return Some(Err(Error::new_without_key(ErrorKind::new_malformed(
                "iteration overflowed document",
            ))));
        }

        let key = match self.doc.read_cstring_at(self.offset + 1) {
            Ok(k) => k,
            Err(e) => {
                self.valid = false;
                return Some(Err(e));
            }
        };

        let offset = self.offset + 1 + key.len() + 1; // type specifier + key + \0
        let kvp_result = try_with_key(key, || {
            let element_type = match ElementType::from(self.doc.as_bytes()[self.offset]) {
                Some(et) => et,
                None => {
                    return Err(Error::new_with_key(
                        key,
                        ErrorKind::new_malformed(format!(
                            "invalid tag: {}",
                            self.doc.as_bytes()[self.offset]
                        )),
                    ))
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
                ElementType::String => read_len(&self.doc.as_bytes()[offset..])?,
                ElementType::EmbeddedDocument => self.next_document_len(offset)?,
                ElementType::Array => self.next_document_len(offset)?,
                ElementType::Binary => self.get_next_length_at(offset)? + 4 + 1,
                ElementType::RegularExpression => {
                    let pattern = self.doc.read_cstring_at(offset)?;
                    let options = self.doc.read_cstring_at(offset + pattern.len() + 1)?;
                    pattern.len() + 1 + options.len() + 1
                }
                ElementType::DbPointer => read_len(&self.doc.as_bytes()[offset..])? + 12,
                ElementType::Symbol => read_len(&self.doc.as_bytes()[offset..])?,
                ElementType::JavaScriptCode => read_len(&self.doc.as_bytes()[offset..])?,
                ElementType::JavaScriptCodeWithScope => self.get_next_length_at(offset)?,
            };

            self.verify_enough_bytes(offset, element_size)?;
            self.offset = offset + element_size;

            Ok((element_type, element_size))
        });

        if kvp_result.is_err() {
            self.valid = false;
        }

        Some(match kvp_result {
            Ok((kind, size)) => Ok(RawElement {
                key,
                kind,
                doc: self.doc,
                start_at: offset,
                size,
            }),
            Err(e) => Err(e),
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
