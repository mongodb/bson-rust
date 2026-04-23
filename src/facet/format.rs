//! Facet format support for BSON.

pub(crate) mod opaque;

use std::borrow::Cow;

use facet::Facet;
use facet_format::{
    ContainerKind,
    DeserializeErrorKind,
    FieldKey,
    FieldLocationHint,
    FormatSerializer,
    ParseError,
    ParseEvent,
    ParseEventKind,
    ScalarValue,
    SerializeError,
};
use facet_reflect::{ReflectError, Span};

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
    RawBsonRef,
    RawDocumentBuf,
    RawJavaScriptCodeWithScope,
    Regex,
    Timestamp,
    error::{Error, Result},
    oid::ObjectId,
    raw::{CStr, CString, MIN_BSON_DOCUMENT_SIZE, RawIter},
    spec::{BinarySubtype, ElementType},
};

/// Serialize a value to BSON bytes.
pub fn serialize_to_vec<'facet, T: Facet<'facet>>(value: &T) -> Result<Vec<u8>> {
    let mut s = Serializer::new();
    facet_format::serialize_root(&mut s, facet_reflect::Peek::new(value)).map_err(|e| match e {
        SerializeError::Backend(e) => e,
        _ => Error::serialization(e),
    })?;
    Ok(s.bytes)
}

#[derive(Debug)]
struct Serializer {
    /// The output buffer.
    bytes: Vec<u8>,
    /// A stack of document size offsets into the buffer for documents that are in the process of
    /// being written.  When one is finished, it pops the offset off the stack and updates that
    /// spot in the buffer with the now-known value.
    doc_size_pos: Vec<usize>,
    /// The offset into the buffer of the element type tag of the current field being added.  BSON
    /// bytes are in [tag, name, value] order but events happen in [name, typed value] order, so
    /// when the name is written it writes a placeholder for the value write to update.
    elem_type_pos: Option<usize>,
    /// A stack of index values for arrays in the process of being written.  The current array will
    /// increment the top value to synthesize a string key and pop it off when the array is closed.
    array_ix: Vec<usize>,
}

impl Serializer {
    fn new() -> Self {
        Self {
            bytes: vec![],
            doc_size_pos: vec![],
            elem_type_pos: None,
            array_ix: vec![],
        }
    }

    fn write_elem_type(&mut self, t: ElementType) -> Result<()> {
        // synthesize a field key if we're in an array
        if self.elem_type_pos.is_none()
            && let Some(ix) = self.array_ix.pop()
        {
            // synthesize the key: index as string
            self.field_key(&ix.to_string())?;
            self.array_ix.push(ix + 1);
        }
        // write the type for a non-toplevel value
        if let Some(type_pos) = self.elem_type_pos.take() {
            self.bytes[type_pos] = t as u8;
        }

        Ok(())
    }

    fn write_raw_bson_ref(&mut self, bv: RawBsonRef<'_>) -> Result<()> {
        self.write_elem_type(bv.element_type())?;
        bv.append_to(&mut self.bytes);
        Ok(())
    }

    fn begin_doc(&mut self, is_array: bool) -> Result<()> {
        self.write_elem_type(
            if is_array {
                ElementType::Array
            } else {
                ElementType::EmbeddedDocument
            },
        )?;
        self.doc_size_pos.push(self.bytes.len());
        self.bytes
            .extend_from_slice(MIN_BSON_DOCUMENT_SIZE.to_le_bytes().as_slice()); // placeholder
        Ok(())
    }
}

impl facet_format::FormatSerializer for Serializer {
    type Error = Error;

    fn begin_struct(&mut self) -> Result<()> {
        self.begin_doc(false)
    }

    fn end_struct(&mut self) -> Result<()> {
        let size_pos = self
            .doc_size_pos
            .pop()
            .ok_or_else(|| Error::serialization("mismatched begin_struct / end_struct"))?;
        self.bytes.push(0); // terminal null
        let size = (self.bytes.len() - size_pos) as i32;
        self.bytes[size_pos..size_pos + 4].copy_from_slice(&size.to_le_bytes());
        Ok(())
    }

    fn field_key(&mut self, key: &str) -> Result<()> {
        if self.elem_type_pos.is_some() {
            return Err(Error::serialization("unexpected field_key"));
        }
        self.elem_type_pos = Some(self.bytes.len());
        self.bytes.push(0); // placeholder
        let key: &CStr = key.try_into()?;
        key.append_to(&mut self.bytes);
        Ok(())
    }

    fn scalar(&mut self, scalar: ScalarValue<'_>) -> Result<()> {
        let tmp_s;
        let tmp_b;
        let bv = match scalar {
            ScalarValue::Unit => RawBsonRef::Null,
            ScalarValue::Null => RawBsonRef::Null,
            ScalarValue::Bool(b) => RawBsonRef::Boolean(b),
            ScalarValue::Char(c) => {
                tmp_s = c.to_string();
                RawBsonRef::String(&tmp_s)
            }
            ScalarValue::I64(i) => RawBsonRef::Int64(i),
            ScalarValue::U64(u) => RawBsonRef::Int64(u.try_into().map_err(|e| {
                Error::serialization(format!("cannot store {u} as a BSON int64: {e}"))
            })?),
            ScalarValue::I128(i) => RawBsonRef::Int64(i.try_into().map_err(|e| {
                Error::serialization(format!("cannot store {i} as a BSON int64: {e}"))
            })?),
            ScalarValue::U128(u) => RawBsonRef::Int64(u.try_into().map_err(|e| {
                Error::serialization(format!("cannot store {u} as a BSON int64: {e}"))
            })?),
            ScalarValue::F64(f) => RawBsonRef::Double(f),
            ScalarValue::Str(c) => match c {
                Cow::Borrowed(s) => RawBsonRef::String(s),
                Cow::Owned(s) => {
                    tmp_s = s;
                    RawBsonRef::String(&tmp_s)
                }
            },
            ScalarValue::Bytes(c) => {
                let bytes = match c {
                    Cow::Borrowed(b) => b,
                    Cow::Owned(b) => {
                        tmp_b = b;
                        &tmp_b
                    }
                };
                RawBsonRef::Binary(RawBinaryRef {
                    subtype: BinarySubtype::Generic,
                    bytes,
                })
            }
        };
        self.write_raw_bson_ref(bv)
    }

    fn serialize_opaque_scalar(
        &mut self,
        _shape: &'static facet::Shape,
        value: facet_reflect::Peek<'_, '_>,
    ) -> Result<bool> {
        // Types that can be directly represented as a RawBsonRef
        let rbr = if let Ok(v) = value.get::<i32>() {
            Some(RawBsonRef::Int32(*v))
        } else if let Ok(re) = value.get::<Regex>() {
            Some(re.into())
        } else if let Ok(b) = value.get::<Binary>() {
            Some(b.into())
        } else if let Ok(ts) = value.get::<Timestamp>() {
            Some((*ts).into())
        } else if let Ok(oid) = value.get::<ObjectId>() {
            Some((*oid).into())
        } else if let Ok(dt) = value.get::<DateTime>() {
            Some((*dt).into())
        } else if let Ok(d) = value.get::<Decimal128>() {
            Some((*d).into())
        } else if let Ok(dbp) = value.get::<DbPointer>() {
            Some(dbp.into())
        } else if let Ok(rd) = value.get::<RawDocumentBuf>() {
            Some(RawBsonRef::Document(rd))
        } else if let Ok(ra) = value.get::<RawArrayBuf>() {
            Some(RawBsonRef::Array(ra))
        } else if let Ok(rjscws) = value.get::<RawJavaScriptCodeWithScope>() {
            Some(RawBsonRef::JavaScriptCodeWithScope(rjscws.into()))
        } else if let Ok(cs) = value.get::<CString>() {
            Some(RawBsonRef::String(cs.as_str()))
        } else if let Ok(rb) = value.get::<crate::RawBson>() {
            Some(rb.as_raw_bson_ref())
        } else {
            None
        };
        if let Some(rbr) = rbr {
            self.write_raw_bson_ref(rbr)?;
            return Ok(true);
        }

        // Types that need special handling
        if let Ok(jscws) = value.get::<JavaScriptCodeWithScope>() {
            self.write_elem_type(ElementType::JavaScriptCodeWithScope)?;
            jscws.append_to(&mut self.bytes)?;

            return Ok(true);
        } else if let Ok(doc) = value.get::<Document>() {
            self.write_elem_type(ElementType::EmbeddedDocument)?;
            doc.append_to(&mut self.bytes)?;

            return Ok(true);
        } else if let Ok(b) = value.get::<Bson>() {
            self.write_elem_type(b.element_type())?;
            b.append_to(&mut self.bytes)?;

            return Ok(true);
        }

        Ok(false)
    }

    fn begin_seq(&mut self) -> Result<()> {
        self.array_ix.push(0);
        self.begin_doc(true)
    }

    fn end_seq(&mut self) -> Result<()> {
        self.array_ix
            .pop()
            .ok_or_else(|| Error::serialization("mismatched begin_seq / end_seq"))?;
        self.end_struct()
    }
}

impl From<ReflectError> for Error {
    fn from(value: ReflectError) -> Self {
        Error::serialization(format!("{value}"))
    }
}

struct Parser<'de> {
    bytes: &'de [u8],
    state: ParseState,
}

struct ParseState {
    offset: usize,
    expects: Expect,
}

enum Expect {
    DocStart,
    DocStartRaw,
    ElemKey,
    ElemValue,
    ElemValueRaw,
}

impl<'de> Parser<'de> {
    fn new(bytes: &'de [u8]) -> Self {
        Self {
            bytes,
            state: ParseState {
                offset: 0,
                expects: Expect::DocStart,
            },
        }
    }

    fn peek(&self) -> Result<Option<(ParseEvent<'de>, ParseState)>> {
        if self.state.offset == self.bytes.len() {
            return Ok(None);
        }
        let mut iter = RawIter::new_unchecked(self.bytes, self.state.offset);
        let event;
        let next;
        match self.state.expects {
            Expect::DocStart => {
                let len = iter.next_document_len(self.state.offset)?;
                event = ParseEvent::new(
                    ParseEventKind::StructStart(ContainerKind::Object),
                    Span::new(self.state.offset, len),
                );
                next = ParseState {
                    offset: self.state.offset + 4, // doc length
                    expects: Expect::ElemKey,
                };
            }
            Expect::DocStartRaw => {
                let len = iter.next_document_len(self.state.offset)?;
                let mut bytes = self.bytes[self.state.offset..self.state.offset + len].to_vec();
                // type tag for parsing a `Bson`/`RawBson` value
                bytes.push(ElementType::EmbeddedDocument as u8);
                event = ParseEvent::new(scalar_bytes(bytes), Span::new(self.state.offset, len));
                next = ParseState {
                    offset: event.span.end(),
                    expects: Expect::DocStart,
                };
            }
            Expect::ElemKey => match iter.next() {
                None => {
                    event =
                        ParseEvent::new(ParseEventKind::StructEnd, Span::new(self.state.offset, 1));
                    next = ParseState {
                        offset: self.state.offset + 1, // doc null terminator
                        expects: Expect::ElemKey,
                    }
                }
                Some(elt) => {
                    let elt = elt?;
                    event = ParseEvent::new(
                        ParseEventKind::FieldKey(FieldKey::new(
                            elt.key().as_str(),
                            FieldLocationHint::KeyValue,
                        )),
                        Span::new(self.state.offset, elt.size()),
                    );
                    next = ParseState {
                        offset: self.state.offset,
                        expects: Expect::ElemValue,
                    }
                }
            },
            Expect::ElemValue => {
                let Some(elt) = iter.next() else {
                    return Err(Error::deserialization("unexpected document end"));
                };
                let elt = elt?;
                let value = elt.value()?;
                if let Some(scalar) = match value {
                    RawBsonRef::Int32(i) => Some(ScalarValue::I64(i as i64)),
                    _ => None,
                } {
                    event = ParseEvent::new(ParseEventKind::Scalar(scalar), elt.value_raw().span());
                    next = ParseState {
                        offset: event.span.end(),
                        expects: Expect::ElemKey,
                    };
                } else {
                    match value {
                        RawBsonRef::Document(_) => {
                            event = ParseEvent::new(
                                ParseEventKind::StructStart(ContainerKind::Object),
                                elt.value_raw().span(),
                            );
                            next = ParseState {
                                offset: elt.value_raw().source_offset() + 4,
                                expects: Expect::ElemKey,
                            };
                        }
                        _ => todo!(),
                    }
                }
            }
            Expect::ElemValueRaw => {
                let Some(elt) = iter.next() else {
                    return Err(Error::deserialization("unexpected document end"));
                };
                let elt = elt?;
                let mut bytes = elt.value_raw().bytes().to_vec();
                // type tag for parsing a `Bson`/`RawBson` value
                bytes.push(elt.element_type() as u8);
                event = ParseEvent::new(scalar_bytes(bytes), elt.value_raw().span());
                next = ParseState {
                    offset: event.span.end(),
                    expects: Expect::ElemKey,
                };
            }
        }
        Ok(Some((event, next)))
    }

    fn parse_err(&self, source: Error) -> ParseError {
        ParseError::new(
            Span::new(self.state.offset, 0),
            DeserializeErrorKind::InvalidValue {
                message: Cow::Owned(source.to_string()),
            },
        )
    }
}

fn scalar_bytes(bytes: Vec<u8>) -> ParseEventKind<'static> {
    ParseEventKind::Scalar(ScalarValue::Bytes(Cow::Owned(bytes)))
}

impl<'de> facet_format::FormatParser<'de> for Parser<'de> {
    fn is_self_describing(&self) -> bool {
        false
    }

    fn input(&self) -> Option<&'de [u8]> {
        Some(self.bytes)
    }

    fn hint_opaque_scalar(
        &mut self,
        type_identifier: &'static str,
        _shape: &'static facet::Shape,
    ) -> bool {
        eprintln!("opaque scalar: {type_identifier:?}");
        false
    }

    fn hint_byte_sequence(&mut self) -> bool {
        eprintln!("hint byte sequence");
        match self.state.expects {
            Expect::DocStart => {
                self.state.expects = Expect::DocStartRaw;
                true
            }
            Expect::ElemValue => {
                self.state.expects = Expect::ElemValueRaw;
                true
            }
            _ => false,
        }
    }

    fn next_event(&mut self) -> std::result::Result<Option<ParseEvent<'de>>, ParseError> {
        let Some((ev, next)) = self.peek().map_err(|e| self.parse_err(e))? else {
            return Ok(None);
        };
        self.state = next;
        eprintln!("next: {ev:#?}");
        Ok(Some(ev))
    }

    fn peek_event(&mut self) -> std::result::Result<Option<ParseEvent<'de>>, ParseError> {
        self.peek()
            .map(|opt| {
                opt.map(|(e, _)| {
                    eprintln!("peek: {e:#?}");
                    e
                })
            })
            .map_err(|e| self.parse_err(e))
    }

    fn skip_value(&mut self) -> std::result::Result<(), ParseError> {
        todo!()
    }

    fn save(&mut self) -> facet_format::SavePoint {
        todo!()
    }

    fn restore(&mut self, _save_point: facet_format::SavePoint) {
        todo!()
    }
}

/// Deserialize a value from BSON bytes.
pub fn deserialize_from_slice<T: Facet<'static>>(bytes: &[u8]) -> Result<T> {
    // Approach:
    // * FacetOpaqueAdapter for all bson types
    // * deserialize_build parses from input as normal
    // * serialize_map:
    //   * for byte buffer / leaf value wrappers, can return a pointer to the buffer
    //   * for others, return a pointer to a static marker that's opaque
    facet_format::FormatDeserializer::new_owned(&mut Parser::new(bytes))
        .deserialize()
        .map_err(|e| Error::deserialization(e))
}
