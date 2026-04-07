//! Facet format support for BSON.

use std::borrow::Cow;

use facet::Facet;
use facet_format::{FormatSerializer, ScalarValue, SerializeError};
use facet_reflect::ReflectError;

use crate::{
    Binary,
    Bson,
    DateTime,
    DbPointer,
    Decimal128,
    Document,
    JavaScriptCodeWithScope,
    RawBinaryRef,
    RawBsonRef,
    Regex,
    Timestamp,
    error::{Error, Result},
    oid::ObjectId,
    raw::{CStr, MIN_BSON_DOCUMENT_SIZE},
    spec::{BinarySubtype, ElementType},
};

/// Serialize a value to BSON bytes.
pub fn to_vec<'facet, T: Facet<'facet>>(value: &T) -> Result<Vec<u8>> {
    let mut s = Serializer::new();
    facet_format::serialize_root(&mut s, facet_reflect::Peek::new(value)).map_err(|e| match e {
        SerializeError::Backend(e) => e,
        _ => Error::serialization(format!("{e}")),
    })?;
    Ok(s.bytes)
}

#[derive(Debug)]
struct Serializer {
    bytes: Vec<u8>,
    doc_size_pos: Vec<usize>,
    elem_type_pos: Option<usize>,
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

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use crate::{Bson, Document, cstr};

    use super::*;

    use facet::Facet;

    #[test]
    fn simple_serialize() {
        #[derive(Facet, Debug)]
        struct Inner {
            value: i32,
        }

        #[derive(Facet, Debug)]
        struct Outer {
            inner: Inner,
            other: i32,
        }

        let bytes = to_vec(&Outer {
            inner: Inner { value: 42 },
            other: 13,
        })
        .unwrap();
        let doc = Document::from_reader(Cursor::new(bytes)).unwrap();
        assert_eq!(doc, doc! { "inner": { "value": 42 }, "other": 13 });
    }

    #[test]
    fn complex_serialize() {
        #[derive(Facet, Debug)]
        struct Inner {
            value: i32,
            arr: Vec<&'static str>,
        }

        #[derive(Facet, Debug)]
        struct Outer {
            inner: Vec<Inner>,
            other: i32,
            more: bool,
        }

        let bytes = to_vec(&Outer {
            inner: vec![
                Inner {
                    value: 42,
                    arr: vec!["hello", "world"],
                },
                Inner {
                    value: 13,
                    arr: vec!["goodbye", "serde"],
                },
            ],
            other: 1066,
            more: true,
        })
        .unwrap();
        let doc = Document::from_reader(Cursor::new(bytes)).unwrap();
        assert_eq!(
            doc,
            doc! {
                "inner": [
                    { "value": 42, "arr": ["hello", "world"] },
                    { "value": 13, "arr": ["goodbye", "serde"] },
                ],
                "other": 1066,
                "more": true,
            }
        );
    }

    #[test]
    fn array_serialize() {
        #[derive(Facet, Debug)]
        struct Outer {
            value: Vec<i32>,
        }

        let bytes = to_vec(&Outer {
            value: vec![42, 13],
        })
        .unwrap();
        let doc = Document::from_reader(Cursor::new(bytes)).unwrap();
        assert_eq!(doc, doc! { "value": [42, 13] });
    }

    fn value_serialize<T: Facet<'static> + Into<Bson> + Clone>(v: T) {
        #[derive(Facet)]
        struct Outer<T> {
            value: T,
        }
        let bytes = to_vec(&Outer { value: v.clone() }).unwrap();
        let doc = Document::from_reader(Cursor::new(bytes)).unwrap();
        assert_eq!(doc, doc! { "value": v });
    }

    #[test]
    fn regex_serialize() {
        value_serialize(Regex {
            pattern: cstr!("foo.*bar").to_owned(),
            options: cstr!("").to_owned(),
        });
    }

    #[test]
    fn binary_serialize() {
        value_serialize(Binary {
            subtype: BinarySubtype::Generic,
            bytes: vec![1, 2, 3, 4],
        });
    }

    #[test]
    fn timestamp_serialize() {
        value_serialize(Timestamp {
            time: 1234,
            increment: 5,
        });
    }

    #[test]
    fn object_id_serialize() {
        value_serialize(ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap());
    }

    #[test]
    fn datetime_serialize() {
        value_serialize(DateTime::from_millis(1_000_000_000_000));
    }

    #[test]
    fn decimal128_serialize() {
        value_serialize("3.14".parse::<Decimal128>().unwrap());
    }

    #[test]
    fn javascript_code_with_scope_serialize() {
        value_serialize(JavaScriptCodeWithScope {
            code: "function(x) { return x + n; }".into(),
            scope: doc! { "n": 1 },
        });
    }

    #[test]
    fn db_pointer_serialize() {
        let id = ObjectId::parse_str("507f1f77bcf86cd799439011").unwrap();
        value_serialize(DbPointer {
            namespace: "test.coll".into(),
            id,
        });
    }

    #[test]
    fn document_serialize() {
        value_serialize(doc! { "hello": "world" });
    }

    #[test]
    fn bson_serialize() {
        value_serialize(Bson::Null);
        value_serialize(Bson::Int32(13));
    }
}
