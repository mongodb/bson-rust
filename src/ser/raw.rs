mod document_serializer;
mod value_serializer;

use serde::{
    ser::{Error as SerdeError, SerializeMap, SerializeStruct},
    Serialize,
};

use self::value_serializer::{ValueSerializer, ValueType};

use crate::{
    raw::{CStr, RAW_ARRAY_NEWTYPE, RAW_DOCUMENT_NEWTYPE},
    ser::{Error, Result},
    serde_helpers::HUMAN_READABLE_NEWTYPE,
    spec::{BinarySubtype, ElementType},
    uuid::UUID_NEWTYPE_NAME,
    RawArray,
    RawBinaryRef,
    RawBsonRef,
    RawDocument,
};
use document_serializer::DocumentSerializer;

/// Serializer used to convert a type `T` into raw BSON bytes.
pub(crate) struct Serializer<'a> {
    bytes: &'a mut Vec<u8>,

    /// The index into `bytes` where the current serialization started.
    start_index: usize,

    /// The index into `bytes` where the current element type will need to be stored.
    /// This needs to be set retroactively because in BSON, the element type comes before the key,
    /// but in serde, the serializer learns of the type after serializing the key.
    type_index: usize,

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

impl SerializerHint {
    fn take(&mut self) -> SerializerHint {
        std::mem::replace(self, SerializerHint::None)
    }
}

impl<'a> Serializer<'a> {
    pub(crate) fn new(bytes: &'a mut Vec<u8>) -> Self {
        let start_index = bytes.len();
        Self {
            bytes,
            start_index,
            type_index: start_index,
            hint: SerializerHint::None,
            human_readable: false,
        }
    }

    /// Reserve a spot for the element type to be set retroactively via `update_element_type`.
    #[inline]
    fn reserve_element_type(&mut self) {
        self.type_index = self.bytes.len(); // record index
        self.bytes.push(0); // push temporary placeholder
    }

    /// Retroactively set the element type of the most recently serialized element.
    #[inline]
    fn update_element_type(&mut self, t: ElementType) -> Result<()> {
        if self.type_index == self.start_index {
            if matches!(t, ElementType::EmbeddedDocument) {
                // don't need to set the element type for the top level document
                return Ok(());
            } else {
                return Err(Error::custom(format!(
                    "attempted to encode a non-document type at the top level: {:?}",
                    t
                )));
            }
        }

        self.bytes[self.type_index] = t as u8;
        Ok(())
    }

    /// Replace an i32 value at the given index with the given value.
    #[inline]
    fn replace_i32(&mut self, at: usize, with: i32) {
        let portion = &mut self.bytes[at..at + 4];
        portion.copy_from_slice(&with.to_le_bytes());
    }

    fn serialize_raw(&mut self, v: RawBsonRef) -> Result<()> {
        self.update_element_type(v.element_type())?;
        v.append_to(self.bytes);
        Ok(())
    }
}

impl<'a, 'b> serde::Serializer for &'a mut Serializer<'b> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = DocumentSerializer<'a, 'b>;
    type SerializeTuple = DocumentSerializer<'a, 'b>;
    type SerializeTupleStruct = DocumentSerializer<'a, 'b>;
    type SerializeTupleVariant = VariantSerializer<'a, 'b>;
    type SerializeMap = DocumentSerializer<'a, 'b>;
    type SerializeStruct = StructSerializer<'a, 'b>;
    type SerializeStructVariant = VariantSerializer<'a, 'b>;

    fn is_human_readable(&self) -> bool {
        self.human_readable
    }

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        self.serialize_raw(RawBsonRef::Boolean(v))
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
        self.serialize_raw(RawBsonRef::Int32(v))
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        self.serialize_raw(RawBsonRef::Int64(v))
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
            Err(_) => Err(Error::too_large_integer(v)),
        }
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        self.serialize_f64(v.into())
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        self.serialize_raw(RawBsonRef::Double(v))
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        let mut s = String::new();
        s.push(v);
        self.serialize_str(&s)
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.serialize_raw(RawBsonRef::String(v))
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        match self.hint.take() {
            SerializerHint::RawDocument => {
                self.serialize_raw(RawBsonRef::Document(RawDocument::new_unchecked(v)))?;
            }
            SerializerHint::RawArray => {
                self.serialize_raw(RawBsonRef::Array(RawArray::from_doc(
                    RawDocument::new_unchecked(v),
                )))?;
            }
            hint => {
                let subtype = if matches!(hint, SerializerHint::Uuid) {
                    BinarySubtype::Uuid
                } else {
                    BinarySubtype::Generic
                };
                self.serialize_raw(RawBsonRef::Binary(RawBinaryRef { subtype, bytes: v }))?;
            }
        };
        Ok(())
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok> {
        self.serialize_raw(RawBsonRef::Null)
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
        self.update_element_type(ElementType::EmbeddedDocument)?;
        let mut d = DocumentSerializer::start(&mut *self);
        d.serialize_entry(variant, value)?;
        d.end_doc()?;
        Ok(())
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.update_element_type(ElementType::Array)?;
        Ok(DocumentSerializer::start(&mut *self))
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
        self.update_element_type(ElementType::EmbeddedDocument)?;
        Ok(VariantSerializer::start(
            &mut *self,
            variant.try_into()?,
            VariantInnerType::Tuple,
        ))
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        self.update_element_type(ElementType::EmbeddedDocument)?;
        Ok(DocumentSerializer::start(&mut *self))
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

        self.update_element_type(
            value_type
                .map(Into::into)
                .unwrap_or(ElementType::EmbeddedDocument),
        )?;
        match value_type {
            Some(vt) => Ok(StructSerializer::Value(ValueSerializer::new(self, vt))),
            None => Ok(StructSerializer::Document(DocumentSerializer::start(self))),
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
        self.update_element_type(ElementType::EmbeddedDocument)?;
        Ok(VariantSerializer::start(
            &mut *self,
            variant.try_into()?,
            VariantInnerType::Struct,
        ))
    }
}

pub(crate) enum StructSerializer<'a, 'b> {
    /// Serialize a BSON value currently represented in serde as a struct (e.g. ObjectId)
    Value(ValueSerializer<'a, 'b>),

    /// Serialize the struct as a document.
    Document(DocumentSerializer<'a, 'b>),
}

impl SerializeStruct for StructSerializer<'_, '_> {
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
pub(crate) struct VariantSerializer<'a, 'b> {
    root_serializer: &'a mut Serializer<'b>,

    /// Variants are serialized as documents of the form `{ <variant name>: <document or array> }`,
    /// and `doc_start` indicates the index at which the outer document begins.
    doc_start: usize,

    /// `inner_start` indicates the index at which the inner document or array begins.
    inner_start: usize,

    /// How many elements have been serialized in the inner document / array so far.
    num_elements_serialized: usize,
}

impl<'a, 'b> VariantSerializer<'a, 'b> {
    fn start(
        rs: &'a mut Serializer<'b>,
        variant: &'static CStr,
        inner_type: VariantInnerType,
    ) -> Self {
        let doc_start = rs.bytes.len();
        // write placeholder length for document, will be updated at end
        static ZERO: RawBsonRef = RawBsonRef::Int32(0);
        ZERO.append_to(rs.bytes);

        let inner = match inner_type {
            VariantInnerType::Struct => ElementType::EmbeddedDocument,
            VariantInnerType::Tuple => ElementType::Array,
        };
        rs.bytes.push(inner as u8);
        variant.append_to(rs.bytes);
        let inner_start = rs.bytes.len();
        // write placeholder length for inner, will be updated at end
        ZERO.append_to(rs.bytes);

        Self {
            root_serializer: rs,
            num_elements_serialized: 0,
            doc_start,
            inner_start,
        }
    }

    #[inline]
    fn serialize_element<T>(&mut self, k: &str, v: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.root_serializer.reserve_element_type();
        CStr::from_str(k)?.append_to(self.root_serializer.bytes);
        v.serialize(&mut *self.root_serializer)?;

        self.num_elements_serialized += 1;
        Ok(())
    }

    #[inline]
    fn end_both(self) -> Result<()> {
        // null byte for the inner
        self.root_serializer.bytes.push(0);
        let arr_length = (self.root_serializer.bytes.len() - self.inner_start) as i32;
        self.root_serializer
            .replace_i32(self.inner_start, arr_length);

        // null byte for document
        self.root_serializer.bytes.push(0);
        let doc_length = (self.root_serializer.bytes.len() - self.doc_start) as i32;
        self.root_serializer.replace_i32(self.doc_start, doc_length);
        Ok(())
    }
}

impl serde::ser::SerializeTupleVariant for VariantSerializer<'_, '_> {
    type Ok = ();

    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.serialize_element(format!("{}", self.num_elements_serialized).as_str(), value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.end_both()
    }
}

impl serde::ser::SerializeStructVariant for VariantSerializer<'_, '_> {
    type Ok = ();

    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.serialize_element(key, value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.end_both()
    }
}
