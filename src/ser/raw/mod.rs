mod document_serializer;
pub(super) mod len_serializer;
mod value_serializer;

use bytes::BufMut;
use serde::{
    ser::{Error as SerdeError, SerializeMap, SerializeStruct},
    Serialize,
};

use self::value_serializer::{ValueSerializer, ValueType};

use crate::{
    de::MAX_BSON_SIZE,
    raw::{RAW_ARRAY_NEWTYPE, RAW_DOCUMENT_NEWTYPE},
    ser::{Error, Result},
    serde_helpers::HUMAN_READABLE_NEWTYPE,
    spec::{BinarySubtype, ElementType},
    uuid::UUID_NEWTYPE_NAME,
};
use document_serializer::DocumentSerializer;

/// Serializer used to convert a type `T` into raw BSON bytes.
pub(crate) struct Serializer<B> {
    buf: B,

    lens: std::vec::IntoIter<i32>,
    started: bool,

    next_key: Option<Key>,

    /// Hint provided by the type being serialized.
    hint: SerializerHint,

    human_readable: bool,
}

/// Various bits of information that the serialized type can provide to the serializer to
/// inform the purpose of the next serialization step.
#[derive(Debug, Clone, Copy)]
enum SerializerHint {
    None,

    /// The next call to `serialize_bytes` is for the purposes of serializing a UUID.
    Uuid,

    /// The next call to `serialize_bytes` is for the purposes of serializing a raw document.
    RawDocument,

    /// The next call to `serialize_bytes` is for the purposes of serializing a raw array.
    RawArray,
}

#[derive(Debug, Clone)]
enum Key {
    Static(&'static str),
    Owned(String),
    Index(usize),
}

impl SerializerHint {
    fn take(&mut self) -> SerializerHint {
        std::mem::replace(self, SerializerHint::None)
    }
}

impl<B: BufMut> Serializer<B> {
    pub(crate) fn new(buf: B, lens: std::vec::IntoIter<i32>) -> Self {
        Self {
            buf,
            lens,
            started: false,
            next_key: None,
            hint: SerializerHint::None,
            human_readable: false,
        }
    }

    /// Convert this serializer into the vec of the serialized bytes.
    pub(crate) fn into_buf(self) -> B {
        self.buf
    }

    // XXX fix sig, this is not falliable.
    #[inline]
    fn write_next_len(&mut self) -> Result<()> {
        self.buf
            .put_i32_le(self.lens.next().expect("pre-recorded len"));
        self.started = true;
        Ok(())
    }

    #[inline]
    fn set_next_key(&mut self, key: Key) {
        self.next_key = Some(key);
    }

    #[inline]
    fn write_key(&mut self, t: ElementType) -> Result<()> {
        if let Some(key) = self.next_key.take() {
            self.buf.put_u8(t as u8);
            match key {
                Key::Static(k) => self.write_cstring(k),
                Key::Owned(k) => self.write_cstring(&k),
                Key::Index(i) => self.write_cstring(&i.to_string()),
            }
        } else {
            if !self.started && t == ElementType::EmbeddedDocument {
                // don't need to write element type and key for top-level document.
                Ok(())
            } else {
                Err(Error::custom(format!(
                    "attempted to encode a non-document type at the top level: {:?}",
                    t
                )))
            }
        }
    }

    #[inline]
    fn write_cstring(&mut self, s: &str) -> Result<()> {
        if s.contains('\0') {
            return Err(Error::InvalidCString(s.into()));
        }
        self.buf.put_slice(s.as_bytes());
        self.buf.put_u8(0);
        Ok(())
    }

    #[inline]
    fn write_string(&mut self, s: &str) {
        self.buf.put_i32_le(s.len() as i32 + 1);
        self.buf.put_slice(s.as_bytes());
        self.buf.put_u8(0);
    }

    #[inline]
    fn write_binary(&mut self, bytes: &[u8], subtype: BinarySubtype) -> Result<()> {
        let len = if let BinarySubtype::BinaryOld = subtype {
            bytes.len() + 4
        } else {
            bytes.len()
        };

        if len > MAX_BSON_SIZE as usize {
            return Err(Error::custom(format!(
                "binary length {} exceeded maximum size",
                bytes.len()
            )));
        }

        self.buf.put_i32_le(len as i32);
        self.buf.put_u8(subtype.into());

        if let BinarySubtype::BinaryOld = subtype {
            self.buf.put_i32_le(len as i32 - 4);
        };

        self.buf.put_slice(bytes);
        Ok(())
    }
}

impl<'a, B: BufMut> serde::Serializer for &'a mut Serializer<B> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = DocumentSerializer<'a, B>;
    type SerializeTuple = DocumentSerializer<'a, B>;
    type SerializeTupleStruct = DocumentSerializer<'a, B>;
    type SerializeTupleVariant = VariantSerializer<'a, B>;
    type SerializeMap = DocumentSerializer<'a, B>;
    type SerializeStruct = StructSerializer<'a, B>;
    type SerializeStructVariant = VariantSerializer<'a, B>;

    fn is_human_readable(&self) -> bool {
        self.human_readable
    }

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        self.write_key(ElementType::Boolean)?;
        self.buf.put_u8(v as u8);
        Ok(())
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.serialize_i32(v.into())
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        self.serialize_i32(v.into())
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        self.write_key(ElementType::Int32)?;
        self.buf.put_i32_le(v);
        Ok(())
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        self.write_key(ElementType::Int64)?;
        self.buf.put_i64_le(v);
        Ok(())
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        self.serialize_i32(v.into())
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        self.serialize_i32(v.into())
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        self.serialize_i64(v.into())
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        use std::convert::TryFrom;

        match i64::try_from(v) {
            Ok(ivalue) => self.serialize_i64(ivalue),
            Err(_) => Err(Error::UnsignedIntegerExceededRange(v)),
        }
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        self.serialize_f64(v.into())
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        self.write_key(ElementType::Double)?;
        self.buf.put_f64_le(v);
        Ok(())
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        let mut s = String::new();
        s.push(v);
        self.serialize_str(&s)
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.write_key(ElementType::String)?;
        self.write_string(v);
        Ok(())
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        match self.hint.take() {
            SerializerHint::RawDocument => {
                self.write_key(ElementType::EmbeddedDocument)?;
                self.buf.put_slice(v);
            }
            SerializerHint::RawArray => {
                self.write_key(ElementType::Array)?;
                self.buf.put_slice(v);
            }
            hint => {
                self.write_key(ElementType::Binary)?;

                let subtype = if matches!(hint, SerializerHint::Uuid) {
                    BinarySubtype::Uuid
                } else {
                    BinarySubtype::Generic
                };

                self.write_binary(v, subtype)?;
            }
        };
        Ok(())
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok> {
        self.write_key(ElementType::Null)?;
        Ok(())
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: serde::Serialize + ?Sized,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok> {
        self.serialize_none()
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        self.serialize_str(variant)
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: serde::Serialize + ?Sized,
    {
        match name {
            UUID_NEWTYPE_NAME => self.hint = SerializerHint::Uuid,
            RAW_DOCUMENT_NEWTYPE => self.hint = SerializerHint::RawDocument,
            RAW_ARRAY_NEWTYPE => self.hint = SerializerHint::RawArray,
            HUMAN_READABLE_NEWTYPE => {
                let old = self.human_readable;
                self.human_readable = true;
                let result = value.serialize(&mut *self);
                self.human_readable = old;
                return result;
            }
            _ => {}
        }
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: serde::Serialize + ?Sized,
    {
        self.write_key(ElementType::EmbeddedDocument)?;
        let mut d = DocumentSerializer::start(&mut *self)?;
        d.serialize_entry(variant, value)?;
        d.end_doc()?;
        Ok(())
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.write_key(ElementType::Array)?;
        DocumentSerializer::start(&mut *self)
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.write_key(ElementType::EmbeddedDocument)?;
        VariantSerializer::start(&mut *self, variant, VariantInnerType::Tuple)
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.write_key(ElementType::EmbeddedDocument)?;
        DocumentSerializer::start(&mut *self)
    }

    #[inline]
    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        let value_type = match name {
            "$oid" => Some(ValueType::ObjectId),
            "$date" => Some(ValueType::DateTime),
            "$binary" => Some(ValueType::Binary),
            "$timestamp" => Some(ValueType::Timestamp),
            "$minKey" => Some(ValueType::MinKey),
            "$maxKey" => Some(ValueType::MaxKey),
            "$code" => Some(ValueType::JavaScriptCode),
            "$codeWithScope" => Some(ValueType::JavaScriptCodeWithScope),
            "$symbol" => Some(ValueType::Symbol),
            "$undefined" => Some(ValueType::Undefined),
            "$regularExpression" => Some(ValueType::RegularExpression),
            "$dbPointer" => Some(ValueType::DbPointer),
            "$numberDecimal" => Some(ValueType::Decimal128),
            _ => None,
        };

        self.write_key(
            value_type
                .map(Into::into)
                .unwrap_or(ElementType::EmbeddedDocument),
        )?;
        match value_type {
            Some(vt) => Ok(StructSerializer::Value(ValueSerializer::new(self, vt))),
            None => Ok(StructSerializer::Document(DocumentSerializer::start(self)?)),
        }
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.write_key(ElementType::EmbeddedDocument)?;
        VariantSerializer::start(&mut *self, variant, VariantInnerType::Struct)
    }
}

pub(crate) enum StructSerializer<'a, B> {
    /// Serialize a BSON value currently represented in serde as a struct (e.g. ObjectId)
    Value(ValueSerializer<'a, B>),

    /// Serialize the struct as a document.
    Document(DocumentSerializer<'a, B>),
}

impl<B: BufMut> SerializeStruct for StructSerializer<'_, B> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        match self {
            StructSerializer::Value(ref mut v) => (&mut *v).serialize_field(key, value),
            StructSerializer::Document(d) => d.serialize_field(key, value),
        }
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        match self {
            StructSerializer::Document(d) => SerializeStruct::end(d),
            StructSerializer::Value(mut v) => v.end(),
        }
    }
}

enum VariantInnerType {
    Tuple,
    Struct,
}

/// Serializer used for enum variants, including both tuple (e.g. Foo::Bar(1, 2, 3)) and
/// struct (e.g. Foo::Bar { a: 1 }).
pub(crate) struct VariantSerializer<'a, B> {
    root_serializer: &'a mut Serializer<B>,

    /// How many elements have been serialized in the inner document / array so far.
    num_elements_serialized: usize,
}

impl<'a, B: BufMut> VariantSerializer<'a, B> {
    fn start(
        rs: &'a mut Serializer<B>,
        variant: &'static str,
        inner_type: VariantInnerType,
    ) -> Result<Self> {
        rs.write_next_len()?;

        let inner = match inner_type {
            VariantInnerType::Struct => ElementType::EmbeddedDocument,
            VariantInnerType::Tuple => ElementType::Array,
        };
        rs.buf.put_u8(inner as u8);
        rs.write_cstring(&variant)?;
        rs.write_next_len()?;

        Ok(Self {
            root_serializer: rs,
            num_elements_serialized: 0,
        })
    }

    #[inline]
    fn serialize_element<T>(&mut self, k: Key, v: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.root_serializer.set_next_key(k);
        v.serialize(&mut *self.root_serializer)?;

        self.num_elements_serialized += 1;
        Ok(())
    }

    #[inline]
    fn end_both(self) -> Result<()> {
        // null byte for the inner
        self.root_serializer.buf.put_u8(0);
        // null byte for document
        self.root_serializer.buf.put_u8(0);
        Ok(())
    }
}

impl<B: BufMut> serde::ser::SerializeTupleVariant for VariantSerializer<'_, B> {
    type Ok = ();

    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.serialize_element(Key::Index(self.num_elements_serialized), value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.end_both()
    }
}

impl<B: BufMut> serde::ser::SerializeStructVariant for VariantSerializer<'_, B> {
    type Ok = ();

    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.serialize_element(Key::Static(key), value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.end_both()
    }
}
