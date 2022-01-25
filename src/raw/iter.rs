use std::convert::TryInto;

use crate::{
    de::{read_bool, MIN_BSON_DOCUMENT_SIZE, MIN_CODE_WITH_SCOPE_SIZE},
    oid::ObjectId,
    raw::{Error, ErrorKind, Result},
    spec::{BinarySubtype, ElementType},
    DateTime,
    Decimal128,
    Timestamp,
};

use super::{
    bson_ref::RawDbPointerRef,
    checked_add,
    error::try_with_key,
    f64_from_slice,
    i32_from_slice,
    i64_from_slice,
    read_lenencoded,
    read_nullterminated,
    RawArray,
    RawBinaryRef,
    RawBsonRef,
    RawDocument,
    RawJavaScriptCodeWithScopeRef,
    RawRegexRef,
};

/// An iterator over the document's entries.
pub struct Iter<'a> {
    doc: &'a RawDocument,
    offset: usize,

    /// Whether the underlying doc is assumed to be valid or if an error has been encountered.
    /// After an error, all subsequent iterations will return None.
    valid: bool,
}

impl<'a> Iter<'a> {
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
            return Err(Error::new_without_key(ErrorKind::MalformedValue {
                message: format!(
                    "length exceeds remaining length of buffer: {} vs {}",
                    num_bytes,
                    self.doc.as_bytes().len() - start
                ),
            }));
        }
        Ok(())
    }

    fn next_oid(&self, starting_at: usize) -> Result<ObjectId> {
        self.verify_enough_bytes(starting_at, 12)?;
        let oid = ObjectId::from_bytes(
            self.doc.as_bytes()[starting_at..(starting_at + 12)]
                .try_into()
                .unwrap(), // ok because we know slice is 12 bytes long
        );
        Ok(oid)
    }

    fn next_document(&self, starting_at: usize) -> Result<&'a RawDocument> {
        self.verify_enough_bytes(starting_at, MIN_BSON_DOCUMENT_SIZE as usize)?;
        let size = i32_from_slice(&self.doc.as_bytes()[starting_at..])? as usize;

        if size < MIN_BSON_DOCUMENT_SIZE as usize {
            return Err(Error::new_without_key(ErrorKind::MalformedValue {
                message: format!("document too small: {} bytes", size),
            }));
        }

        self.verify_enough_bytes(starting_at, size)?;
        let end = starting_at + size;

        if self.doc.as_bytes()[end - 1] != 0 {
            return Err(Error {
                key: None,
                kind: ErrorKind::MalformedValue {
                    message: "not null terminated".into(),
                },
            });
        }
        RawDocument::from_bytes(&self.doc.as_bytes()[starting_at..end])
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<(&'a str, RawBsonRef<'a>)>;

    fn next(&mut self) -> Option<Result<(&'a str, RawBsonRef<'a>)>> {
        if !self.valid {
            return None;
        } else if self.offset == self.doc.as_bytes().len() - 1 {
            if self.doc.as_bytes()[self.offset] == 0 {
                // end of document marker
                return None;
            } else {
                self.valid = false;
                return Some(Err(Error {
                    key: None,
                    kind: ErrorKind::MalformedValue {
                        message: "document not null terminated".into(),
                    },
                }));
            }
        } else if self.offset >= self.doc.as_bytes().len() {
            self.valid = false;
            return Some(Err(Error::new_without_key(ErrorKind::MalformedValue {
                message: "iteration overflowed document".to_string(),
            })));
        }

        let key = match read_nullterminated(&self.doc.as_bytes()[self.offset + 1..]) {
            Ok(k) => k,
            Err(e) => {
                self.valid = false;
                return Some(Err(e));
            }
        };

        let kvp_result = try_with_key(key, || {
            let valueoffset = self.offset + 1 + key.len() + 1; // type specifier + key + \0

            let element_type = match ElementType::from(self.doc.as_bytes()[self.offset]) {
                Some(et) => et,
                None => {
                    return Err(Error::new_with_key(
                        key,
                        ErrorKind::MalformedValue {
                            message: format!("invalid tag: {}", self.doc.as_bytes()[self.offset]),
                        },
                    ))
                }
            };

            let (element, element_size) = match element_type {
                ElementType::Int32 => {
                    let i = i32_from_slice(&self.doc.as_bytes()[valueoffset..])?;
                    (RawBsonRef::Int32(i), 4)
                }
                ElementType::Int64 => {
                    let i = i64_from_slice(&self.doc.as_bytes()[valueoffset..])?;
                    (RawBsonRef::Int64(i), 8)
                }
                ElementType::Double => {
                    let f = f64_from_slice(&self.doc.as_bytes()[valueoffset..])?;
                    (RawBsonRef::Double(f), 8)
                }
                ElementType::String => {
                    let s = read_lenencoded(&self.doc.as_bytes()[valueoffset..])?;
                    (RawBsonRef::String(s), 4 + s.len() + 1)
                }
                ElementType::EmbeddedDocument => {
                    let doc = self.next_document(valueoffset)?;
                    (RawBsonRef::Document(doc), doc.as_bytes().len())
                }
                ElementType::Array => {
                    let doc = self.next_document(valueoffset)?;
                    (
                        RawBsonRef::Array(RawArray::from_doc(doc)),
                        doc.as_bytes().len(),
                    )
                }
                ElementType::Binary => {
                    let len = i32_from_slice(&self.doc.as_bytes()[valueoffset..])? as usize;
                    let data_start = valueoffset + 4 + 1;

                    if len >= i32::MAX as usize {
                        return Err(Error::new_without_key(ErrorKind::MalformedValue {
                            message: format!("binary length exceeds maximum: {}", len),
                        }));
                    }

                    self.verify_enough_bytes(valueoffset + 4, len + 1)?;
                    let subtype = BinarySubtype::from(self.doc.as_bytes()[valueoffset + 4]);
                    let data = match subtype {
                        BinarySubtype::BinaryOld => {
                            if len < 4 {
                                return Err(Error::new_without_key(ErrorKind::MalformedValue {
                                    message: "old binary subtype has no inner declared length"
                                        .into(),
                                }));
                            }
                            let oldlength =
                                i32_from_slice(&self.doc.as_bytes()[data_start..])? as usize;
                            if checked_add(oldlength, 4)? != len {
                                return Err(Error::new_without_key(ErrorKind::MalformedValue {
                                    message: "old binary subtype has wrong inner declared length"
                                        .into(),
                                }));
                            }
                            &self.doc.as_bytes()[(data_start + 4)..(data_start + len)]
                        }
                        _ => &self.doc.as_bytes()[data_start..(data_start + len)],
                    };
                    (
                        RawBsonRef::Binary(RawBinaryRef {
                            subtype,
                            bytes: data,
                        }),
                        4 + 1 + len,
                    )
                }
                ElementType::ObjectId => {
                    let oid = self.next_oid(valueoffset)?;
                    (RawBsonRef::ObjectId(oid), 12)
                }
                ElementType::Boolean => {
                    let b = read_bool(&self.doc.as_bytes()[valueoffset..]).map_err(|e| {
                        Error::new_with_key(
                            key,
                            ErrorKind::MalformedValue {
                                message: e.to_string(),
                            },
                        )
                    })?;
                    (RawBsonRef::Boolean(b), 1)
                }
                ElementType::DateTime => {
                    let ms = i64_from_slice(&self.doc.as_bytes()[valueoffset..])?;
                    (RawBsonRef::DateTime(DateTime::from_millis(ms)), 8)
                }
                ElementType::RegularExpression => {
                    let pattern = read_nullterminated(&self.doc.as_bytes()[valueoffset..])?;
                    let options = read_nullterminated(
                        &self.doc.as_bytes()[(valueoffset + pattern.len() + 1)..],
                    )?;
                    (
                        RawBsonRef::RegularExpression(RawRegexRef { pattern, options }),
                        pattern.len() + 1 + options.len() + 1,
                    )
                }
                ElementType::Null => (RawBsonRef::Null, 0),
                ElementType::Undefined => (RawBsonRef::Undefined, 0),
                ElementType::Timestamp => {
                    let ts = Timestamp::from_reader(&self.doc.as_bytes()[valueoffset..]).map_err(
                        |e| {
                            Error::new_without_key(ErrorKind::MalformedValue {
                                message: e.to_string(),
                            })
                        },
                    )?;
                    (RawBsonRef::Timestamp(ts), 8)
                }
                ElementType::JavaScriptCode => {
                    let code = read_lenencoded(&self.doc.as_bytes()[valueoffset..])?;
                    (RawBsonRef::JavaScriptCode(code), 4 + code.len() + 1)
                }
                ElementType::JavaScriptCodeWithScope => {
                    let length = i32_from_slice(&self.doc.as_bytes()[valueoffset..])? as usize;

                    if length < MIN_CODE_WITH_SCOPE_SIZE as usize {
                        return Err(Error::new_without_key(ErrorKind::MalformedValue {
                            message: "code with scope length too small".to_string(),
                        }));
                    }

                    self.verify_enough_bytes(valueoffset, length)?;
                    let slice = &&self.doc.as_bytes()[valueoffset..(valueoffset + length)];
                    let code = read_lenencoded(&slice[4..])?;
                    let scope_start = 4 + 4 + code.len() + 1;
                    let scope = RawDocument::from_bytes(&slice[scope_start..])?;
                    (
                        RawBsonRef::JavaScriptCodeWithScope(RawJavaScriptCodeWithScopeRef {
                            code,
                            scope,
                        }),
                        length,
                    )
                }
                ElementType::DbPointer => {
                    let namespace = read_lenencoded(&self.doc.as_bytes()[valueoffset..])?;
                    let id = self.next_oid(valueoffset + 4 + namespace.len() + 1)?;
                    (
                        RawBsonRef::DbPointer(RawDbPointerRef { namespace, id }),
                        4 + namespace.len() + 1 + 12,
                    )
                }
                ElementType::Symbol => {
                    let s = read_lenencoded(&self.doc.as_bytes()[valueoffset..])?;
                    (RawBsonRef::Symbol(s), 4 + s.len() + 1)
                }
                ElementType::Decimal128 => {
                    self.verify_enough_bytes(valueoffset, 16)?;
                    (
                        RawBsonRef::Decimal128(Decimal128::from_bytes(
                            self.doc.as_bytes()[valueoffset..(valueoffset + 16)]
                                .try_into()
                                .unwrap(),
                        )),
                        16,
                    )
                }
                ElementType::MinKey => (RawBsonRef::MinKey, 0),
                ElementType::MaxKey => (RawBsonRef::MaxKey, 0),
            };

            self.offset = valueoffset + element_size;
            self.verify_enough_bytes(valueoffset, element_size)?;

            Ok((key, element))
        });

        if kvp_result.is_err() {
            self.valid = false;
        }

        Some(kvp_result)
    }
}
