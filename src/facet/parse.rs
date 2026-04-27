use std::borrow::Cow;

use facet::Facet;
use facet_format::{
    ContainerKind,
    DeserializeErrorKind,
    FieldKey,
    FieldLocationHint,
    ParseError,
    ParseEvent,
    ParseEventKind,
    ScalarValue,
};
use facet_reflect::Span;

use crate::{
    RawBinaryRef,
    RawBsonRef,
    RawDocument,
    error::{Error, Result},
    raw::{RawElement, RawIter},
    spec::{BinarySubtype, ElementType},
};

struct Parser<'de> {
    bytes: &'de [u8],
    state: ParseState,
    saved: Option<ParseState>,
}

#[derive(Debug, Clone)]
struct ParseState {
    offset: usize,
    expects: Expect,
    outer: Vec<Expect>,
}

#[derive(Debug, Copy, Clone)]
enum Expect {
    DocStart,
    DocStartRaw,
    ElemKey,
    ElemValue { is_array: bool },
    ElemValueRaw { is_array: bool },
    Eof,
}

impl<'de> Parser<'de> {
    fn new(bytes: &'de [u8]) -> Self {
        Self {
            bytes,
            state: ParseState {
                offset: 0,
                expects: Expect::DocStart,
                outer: vec![],
            },
            saved: None,
        }
    }

    fn peek(&self) -> Result<Option<(ParseEvent<'de>, ParseState)>> {
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
                    outer: vec![Expect::Eof],
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
                    expects: Expect::Eof,
                    outer: vec![],
                };
            }
            Expect::ElemKey => {
                let Some(elt) = iter.next().transpose()? else {
                    return self.container_end(false).map(Some);
                };
                event = ParseEvent::new(
                    ParseEventKind::FieldKey(FieldKey::new(
                        elt.key().as_str(),
                        FieldLocationHint::KeyValue,
                    )),
                    Span::new(self.state.offset, elt.size()),
                );
                next = ParseState {
                    offset: self.state.offset,
                    expects: Expect::ElemValue { is_array: false },
                    outer: self.state.outer.clone(),
                }
            }
            Expect::ElemValue { is_array } => {
                let Some(elt) = iter.next().transpose()? else {
                    if is_array {
                        return self.container_end(true).map(Some);
                    } else {
                        // End of document is detected in Expect::ElemKey; finding it here indicates
                        // invalid wire bytes.
                        return Err(Error::deserialization("unexpected document end"));
                    }
                };
                let next_expect = if is_array {
                    Expect::ElemValue { is_array: true }
                } else {
                    Expect::ElemKey
                };
                event = elt.as_event()?;
                match &event.kind {
                    ParseEventKind::Scalar(_) => {
                        next = ParseState {
                            offset: event.span.end(),
                            expects: next_expect,
                            outer: self.state.outer.clone(),
                        };
                    }
                    ParseEventKind::StructStart(_) => {
                        next = ParseState {
                            offset: elt.value_raw().source_offset() + 4,
                            expects: Expect::ElemKey,
                            outer: vec_and(&self.state.outer, next_expect),
                        };
                    }
                    ParseEventKind::SequenceStart(_) => {
                        next = ParseState {
                            offset: elt.value_raw().source_offset() + 4,
                            expects: Expect::ElemValue { is_array: true },
                            outer: vec_and(&self.state.outer, next_expect),
                        }
                    }
                    ek => {
                        return Err(Error::deserialization(format!(
                            "unexpected parse event {ek:?}"
                        )));
                    }
                }
            }
            Expect::ElemValueRaw { is_array } => {
                let Some(elt) = iter.next().transpose()? else {
                    return Err(Error::deserialization("unexpected document end"));
                };
                let mut bytes = elt.value_raw().bytes().to_vec();
                // type tag for parsing a `Bson`/`RawBson` value
                bytes.push(elt.element_type() as u8);
                event = ParseEvent::new(scalar_bytes(bytes), elt.value_raw().span());
                next = ParseState {
                    offset: event.span.end(),
                    expects: if is_array {
                        Expect::ElemValue { is_array: true }
                    } else {
                        Expect::ElemKey
                    },
                    outer: self.state.outer.clone(),
                };
            }
            Expect::Eof => {
                if self.state.offset != self.bytes.len() {
                    return Err(Error::deserialization("unparsed bytes at end of buffer"));
                }
                return Ok(None);
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

    fn container_end(&self, is_array: bool) -> Result<(ParseEvent<'de>, ParseState)> {
        let ev_kind = if is_array {
            ParseEventKind::SequenceEnd
        } else {
            ParseEventKind::StructEnd
        };
        let event = ParseEvent::new(ev_kind, Span::new(self.state.offset, 1));
        let Some((expects, outer)) = self.state.outer.split_last() else {
            return Err(Error::deserialization("mismatched container structure"));
        };
        let next = ParseState {
            offset: self.state.offset + 1, // doc null terminator
            expects: *expects,
            outer: outer.to_vec(),
        };
        Ok((event, next))
    }
}

impl<'a> RawElement<'a> {
    fn as_event(&self) -> Result<ParseEvent<'a>> {
        let value = self.value()?;
        let span = self.value_raw().span();
        let ek = match value {
            RawBsonRef::Double(f) => ParseEventKind::Scalar(ScalarValue::F64(f)),
            RawBsonRef::String(s) => ParseEventKind::Scalar(ScalarValue::Str(Cow::Borrowed(s))),
            RawBsonRef::Boolean(b) => ParseEventKind::Scalar(ScalarValue::Bool(b)),
            RawBsonRef::Null => ParseEventKind::Scalar(ScalarValue::Null),
            RawBsonRef::Int32(i) => ParseEventKind::Scalar(ScalarValue::I64(i as i64)),
            RawBsonRef::Int64(i) => ParseEventKind::Scalar(ScalarValue::I64(i)),
            RawBsonRef::Binary(RawBinaryRef {
                subtype: BinarySubtype::Generic,
                bytes,
            }) => ParseEventKind::Scalar(ScalarValue::Bytes(Cow::Borrowed(bytes))),

            RawBsonRef::Array(_) => ParseEventKind::SequenceStart(ContainerKind::Array),
            RawBsonRef::Document(_) => ParseEventKind::StructStart(ContainerKind::Object),

            RawBsonRef::RegularExpression(_)
            | RawBsonRef::JavaScriptCode(_)
            | RawBsonRef::JavaScriptCodeWithScope(_)
            | RawBsonRef::Timestamp(_)
            | RawBsonRef::Binary(_)
            | RawBsonRef::ObjectId(_)
            | RawBsonRef::DateTime(_)
            | RawBsonRef::Symbol(_)
            | RawBsonRef::Decimal128(_)
            | RawBsonRef::Undefined
            | RawBsonRef::MaxKey
            | RawBsonRef::MinKey
            | RawBsonRef::DbPointer(_) => {
                return Err(Error::deserialization(format!(
                    "composite BSON type {:?} must be parsed into its corresponding Rust type",
                    value.element_type()
                )));
            }
        };
        Ok(ParseEvent::new(ek, span))
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

    fn hint_byte_sequence(&mut self) -> bool {
        if let Some(new) = self.state.expects.raw() {
            self.state.expects = new;
            true
        } else {
            false
        }
    }

    fn next_event(&mut self) -> std::result::Result<Option<ParseEvent<'de>>, ParseError> {
        let Some((ev, next)) = self.peek().map_err(|e| self.parse_err(e))? else {
            return Ok(None);
        };
        self.state = next;
        Ok(Some(ev))
    }

    fn peek_event(&mut self) -> std::result::Result<Option<ParseEvent<'de>>, ParseError> {
        self.peek()
            .map(|opt| opt.map(|(e, _)| e))
            .map_err(|e| self.parse_err(e))
    }

    fn skip_value(&mut self) -> std::result::Result<(), ParseError> {
        match &self.state.expects {
            Expect::ElemValue { is_array } | Expect::ElemValueRaw { is_array } => {
                let mut iter = RawIter::new_unchecked(self.bytes, self.state.offset);
                let Some(elt) = iter.next().transpose().map_err(|e| self.parse_err(e))? else {
                    return Ok(());
                };
                self.state = ParseState {
                    offset: elt.value_raw().span().end(),
                    expects: if *is_array {
                        Expect::ElemValue { is_array: true }
                    } else {
                        Expect::ElemKey
                    },
                    outer: self.state.outer.clone(),
                };

                Ok(())
            }
            ex => Err(self.parse_err(Error::deserialization(format!(
                "unexpected skip_value for {ex:?}"
            )))),
        }
    }

    fn save(&mut self) -> facet_format::SavePoint {
        self.saved = Some(self.state.clone());
        facet_format::SavePoint::new(self.state.offset as u64)
    }

    fn restore(&mut self, save_point: facet_format::SavePoint) {
        debug_assert!(self.saved.is_some());
        if let Some(saved) = self.saved.take() {
            debug_assert!(save_point.0 == saved.offset as u64);
            self.state = saved;
        }
    }
}

impl Expect {
    fn raw(self) -> Option<Self> {
        Some(match self {
            Self::DocStart => Self::DocStartRaw,
            Self::ElemValue { is_array } => Self::ElemValueRaw { is_array },
            _ => return None,
        })
    }
}

/// Deserialize a value from a binary BSON document.
pub fn deserialize_from_slice<T: Facet<'static>>(bytes: &[u8]) -> Result<T> {
    RawDocument::from_bytes(bytes)?;
    facet_format::FormatDeserializer::new_owned(&mut Parser::new(bytes))
        .deserialize()
        .map_err(Error::deserialization)
}

fn vec_and<T: Clone>(vs: &[T], v: T) -> Vec<T> {
    let mut out = vs.to_vec();
    out.push(v);
    out
}
