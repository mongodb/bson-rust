use serde::{
    ser::{Error as SerdeError, SerializeMap, SerializeStruct},
    Serialize,
};

use crate::{
    raw::{RAW_ARRAY_NEWTYPE, RAW_DOCUMENT_NEWTYPE},
    ser::{Error, Result},
    serde_helpers::HUMAN_READABLE_NEWTYPE,
    spec::{BinarySubtype, ElementType},
    uuid::UUID_NEWTYPE_NAME,
};

/// Serializer used to convert a type `T` into raw BSON bytes.
pub(crate) struct Serializer {
    /// Length of all documents visited by the serializer in the order in which they are serialized.
    /// The length of the root document will always appear at index zero.
    lens: Vec<i32>,

    /// Index of each document and sub-document we are computing the length of.
    /// For well-formed serialization requests this will always contain at least one element.
    lens_stack: Vec<usize>,

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

impl Serializer {
    pub(crate) fn new() -> Self {
        Self {
            lens: vec![],
            lens_stack: vec![],
            hint: SerializerHint::None,
            human_readable: false,
        }
    }

    pub(crate) fn into_lens(self) -> Vec<i32> {
        assert!(self.lens_stack.is_empty());
        self.lens
    }

    #[inline]
    fn enter_doc(&mut self) {
        let index = self.lens.len();
        self.lens.push(0);
        self.lens_stack.push(index);
    }

    #[inline]
    fn exit_doc(&mut self) {
        let index = self
            .lens_stack
            .pop()
            .expect("document enter and exit are paired");
        self.lens[index] += 4 + 1; // i32 doc len + null terminator.
        let len = self.lens[index];
        if let Some(parent_index) = self.lens_stack.last() {
            // propagate length back up to parent, if present.
            self.lens[*parent_index] += len;
        }
    }

    #[inline]
    fn add_bytes(&mut self, bytes: i32) -> Result<()> {
        if let Some(index) = self.lens_stack.last() {
            self.lens[*index] += bytes;
            Ok(())
        } else {
            Err(Error::custom(format!(
                "attempted to encode a non-document type at the top level",
            )))
        }
    }

    #[inline]
    fn add_element_name_and_type(&mut self, len: usize) -> Result<()> {
        // type + length + null terminator.
        self.add_bytes(1 + len as i32 + 1)
    }

    #[inline]
    fn add_cstr_bytes(&mut self, len: usize) -> Result<()> {
        self.add_bytes(len as i32 + 1)
    }

    #[inline]
    fn add_bin_bytes(&mut self, len: usize, subtype: BinarySubtype) -> Result<()> {
        let total_len = if subtype == BinarySubtype::BinaryOld {
            4 + 1 + 4 + len as i32
        } else {
            4 + 1 + len as i32
        };
        self.add_bytes(total_len)
    }

    #[inline]
    fn add_str_bytes(&mut self, len: usize) -> Result<()> {
        self.add_bytes(4 + len as i32 + 1)
    }
}

impl<'a> serde::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = DocumentSerializer<'a>;
    type SerializeTuple = DocumentSerializer<'a>;
    type SerializeTupleStruct = DocumentSerializer<'a>;
    type SerializeTupleVariant = VariantSerializer<'a>;
    type SerializeMap = DocumentSerializer<'a>;
    type SerializeStruct = StructSerializer<'a>;
    type SerializeStructVariant = VariantSerializer<'a>;

    fn is_human_readable(&self) -> bool {
        self.human_readable
    }

    #[inline]
    fn serialize_bool(self, _v: bool) -> Result<Self::Ok> {
        self.add_bytes(1)
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
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok> {
        self.add_bytes(4)
    }

    #[inline]
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok> {
        self.add_bytes(8)
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
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok> {
        self.add_bytes(8)
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        self.serialize_f64(v.into())
    }

    #[inline]
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok> {
        self.add_bytes(8)
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        let mut s = String::new();
        s.push(v);
        self.serialize_str(&s)
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.add_str_bytes(v.len())
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        match self.hint.take() {
            SerializerHint::RawDocument | SerializerHint::RawArray => {
                if self.lens_stack.is_empty() {
                    // The root document is raw in this case.
                    self.enter_doc();
                    let result = self.add_bytes(v.len() as i32);
                    self.exit_doc();
                    result
                } else {
                    // We don't record these as docs as the lengths aren't computed from multiple inputs.
                    self.add_bytes(v.len() as i32)
                }
            }
            // NB: in this path we would never emit BinaryOld.
            _ => self.add_bin_bytes(v.len(), BinarySubtype::Generic),
        }
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok> {
        // this writes an ElementType::Null, which records 0 following bytes for the value.
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
        let mut d = DocumentSerializer::start(&mut *self)?;
        d.serialize_entry(variant, value)?;
        d.end_doc()?;
        Ok(())
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
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
        VariantSerializer::start(&mut *self, variant)
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
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
        VariantSerializer::start(&mut *self, variant)
    }
}

pub(crate) enum StructSerializer<'a> {
    /// Serialize a BSON value currently represented in serde as a struct (e.g. ObjectId)
    Value(ValueSerializer<'a>),

    /// Serialize the struct as a document.
    Document(DocumentSerializer<'a>),
}

impl SerializeStruct for StructSerializer<'_> {
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

/// Serializer used for enum variants, including both tuple (e.g. Foo::Bar(1, 2, 3)) and
/// struct (e.g. Foo::Bar { a: 1 }).
pub(crate) struct VariantSerializer<'a> {
    root_serializer: &'a mut Serializer,

    /// How many elements have been serialized in the inner document / array so far.
    num_elements_serialized: usize,
}

impl<'a> VariantSerializer<'a> {
    fn start(rs: &'a mut Serializer, variant: &'static str) -> Result<Self> {
        rs.enter_doc(); // outer doc for variant
        rs.add_element_name_and_type(variant.len())?;

        rs.enter_doc(); // inner doc/array containing variant doc/tuple.
        Ok(Self {
            root_serializer: rs,
            num_elements_serialized: 0,
        })
    }

    #[inline]
    fn serialize_element<T>(&mut self, k: &str, v: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.root_serializer.add_element_name_and_type(k.len())?;
        v.serialize(&mut *self.root_serializer)?;
        self.num_elements_serialized += 1;
        Ok(())
    }

    #[inline]
    fn end_both(self) -> Result<()> {
        self.root_serializer.exit_doc(); // close variant doc/array
        self.root_serializer.exit_doc(); // close variant wrapper.
        Ok(())
    }
}

impl serde::ser::SerializeTupleVariant for VariantSerializer<'_> {
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

impl serde::ser::SerializeStructVariant for VariantSerializer<'_> {
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

use serde::ser::Impossible;

use crate::{to_bson, Bson};

/// Serializer used to serialize document or array bodies.
pub(crate) struct DocumentSerializer<'a> {
    root_serializer: &'a mut Serializer,
    num_keys_serialized: usize,
}

impl<'a> DocumentSerializer<'a> {
    pub(crate) fn start(rs: &'a mut Serializer) -> crate::ser::Result<Self> {
        rs.enter_doc();
        Ok(Self {
            root_serializer: rs,
            num_keys_serialized: 0,
        })
    }

    /// Serialize a document key using the provided closure.
    fn serialize_doc_key_custom<F: FnOnce(&mut Serializer) -> Result<()>>(
        &mut self,
        f: F,
    ) -> Result<()> {
        f(self.root_serializer)?;
        self.num_keys_serialized += 1;
        Ok(())
    }

    /// Serialize a document key to string using [`KeySerializer`].
    fn serialize_doc_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        self.serialize_doc_key_custom(|rs| {
            key.serialize(KeySerializer {
                root_serializer: rs,
            })?;
            Ok(())
        })?;
        Ok(())
    }

    pub(crate) fn end_doc(self) -> crate::ser::Result<&'a mut Serializer> {
        self.root_serializer.exit_doc();
        Ok(self.root_serializer)
    }
}

impl serde::ser::SerializeSeq for DocumentSerializer<'_> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        let index = self.num_keys_serialized;
        self.serialize_doc_key_custom(|rs| rs.add_element_name_and_type(index.to_string().len()))?;
        value.serialize(&mut *self.root_serializer)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

impl serde::ser::SerializeMap for DocumentSerializer<'_> {
    type Ok = ();

    type Error = Error;

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        self.serialize_doc_key(key)
    }

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        value.serialize(&mut *self.root_serializer)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

impl serde::ser::SerializeStruct for DocumentSerializer<'_> {
    type Ok = ();

    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        self.serialize_doc_key(key)?;
        value.serialize(&mut *self.root_serializer)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

impl serde::ser::SerializeTuple for DocumentSerializer<'_> {
    type Ok = ();

    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        self.serialize_doc_key(&self.num_keys_serialized.to_string())?;
        value.serialize(&mut *self.root_serializer)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

impl serde::ser::SerializeTupleStruct for DocumentSerializer<'_> {
    type Ok = ();

    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        self.serialize_doc_key(&self.num_keys_serialized.to_string())?;
        value.serialize(&mut *self.root_serializer)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

/// Serializer used specifically for serializing document keys.
/// Only keys that serialize to strings will be accepted.
struct KeySerializer<'a> {
    root_serializer: &'a mut Serializer,
}

impl KeySerializer<'_> {
    fn invalid_key<T: Serialize>(v: T) -> Error {
        Error::InvalidDocumentKey(to_bson(&v).unwrap_or(Bson::Null))
    }
}

impl serde::Serializer for KeySerializer<'_> {
    type Ok = ();

    type Error = Error;

    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    #[inline]
    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        Err(Self::invalid_key(v))
    }

    #[inline]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        Err(Self::invalid_key(v))
    }

    #[inline]
    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        Err(Self::invalid_key(v))
    }

    #[inline]
    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        Err(Self::invalid_key(v))
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        Err(Self::invalid_key(v))
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        Err(Self::invalid_key(v))
    }

    #[inline]
    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        Err(Self::invalid_key(v))
    }

    #[inline]
    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        Err(Self::invalid_key(v))
    }

    #[inline]
    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        Err(Self::invalid_key(v))
    }

    #[inline]
    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        Err(Self::invalid_key(v))
    }

    #[inline]
    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        Err(Self::invalid_key(v))
    }

    #[inline]
    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        Err(Self::invalid_key(v))
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.root_serializer.add_element_name_and_type(v.len())
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        Err(Self::invalid_key(v))
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok> {
        Err(Self::invalid_key(Bson::Null))
    }

    #[inline]
    fn serialize_some<T>(self, value: &T) -> Result<Self::Ok>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok> {
        Err(Self::invalid_key(Bson::Null))
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        Err(Self::invalid_key(Bson::Null))
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
    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize + ?Sized,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: Serialize + ?Sized,
    {
        Err(Self::invalid_key(value))
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Self::invalid_key(Bson::Array(vec![])))
    }

    #[inline]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Self::invalid_key(Bson::Array(vec![])))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Self::invalid_key(Bson::Document(doc! {})))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Self::invalid_key(Bson::Array(vec![])))
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Self::invalid_key(Bson::Document(doc! {})))
    }

    #[inline]
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(Self::invalid_key(Bson::Document(doc! {})))
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Self::invalid_key(Bson::Document(doc! {})))
    }
}

use crate::{base64, RawDocument, RawJavaScriptCodeWithScopeRef};

/// A serializer used specifically for serializing the serde-data-model form of a BSON type (e.g.
/// [`Binary`]) to raw bytes.
pub(crate) struct ValueSerializer<'a> {
    root_serializer: &'a mut Serializer,
    state: SerializationStep,
}

/// State machine used to track which step in the serialization of a given type the serializer is
/// currently on.
#[derive(Debug)]
enum SerializationStep {
    Oid,

    DateTime,
    DateTimeNumberLong,

    Binary,
    /// This step can either transition to the raw or base64 steps depending
    /// on whether a string or bytes are serialized.
    BinaryBytes,
    BinarySubType {
        base64: String,
    },
    RawBinarySubType {
        bytes: Vec<u8>,
    },

    Symbol,

    RegEx,
    RegExPattern,
    RegExOptions,

    Timestamp,
    TimestampTime,
    TimestampIncrement,

    DbPointer,
    DbPointerRef,
    DbPointerId,

    Code,

    CodeWithScopeCode,
    CodeWithScopeScope {
        code: String,
        raw: bool,
    },

    MinKey,

    MaxKey,

    Undefined,

    Decimal128,
    Decimal128Value,

    Done,
}

/// Enum of BSON "value" types that this serializer can serialize.
#[derive(Debug, Clone, Copy)]
pub(super) enum ValueType {
    DateTime,
    Binary,
    ObjectId,
    Symbol,
    RegularExpression,
    Timestamp,
    DbPointer,
    JavaScriptCode,
    JavaScriptCodeWithScope,
    MinKey,
    MaxKey,
    Decimal128,
    Undefined,
}

impl From<ValueType> for ElementType {
    fn from(vt: ValueType) -> Self {
        match vt {
            ValueType::Binary => ElementType::Binary,
            ValueType::DateTime => ElementType::DateTime,
            ValueType::DbPointer => ElementType::DbPointer,
            ValueType::Decimal128 => ElementType::Decimal128,
            ValueType::Symbol => ElementType::Symbol,
            ValueType::RegularExpression => ElementType::RegularExpression,
            ValueType::Timestamp => ElementType::Timestamp,
            ValueType::JavaScriptCode => ElementType::JavaScriptCode,
            ValueType::JavaScriptCodeWithScope => ElementType::JavaScriptCodeWithScope,
            ValueType::MaxKey => ElementType::MaxKey,
            ValueType::MinKey => ElementType::MinKey,
            ValueType::Undefined => ElementType::Undefined,
            ValueType::ObjectId => ElementType::ObjectId,
        }
    }
}

impl<'a> ValueSerializer<'a> {
    pub(super) fn new(rs: &'a mut Serializer, value_type: ValueType) -> Self {
        let state = match value_type {
            ValueType::DateTime => SerializationStep::DateTime,
            ValueType::Binary => SerializationStep::Binary,
            ValueType::ObjectId => SerializationStep::Oid,
            ValueType::Symbol => SerializationStep::Symbol,
            ValueType::RegularExpression => SerializationStep::RegEx,
            ValueType::Timestamp => SerializationStep::Timestamp,
            ValueType::DbPointer => SerializationStep::DbPointer,
            ValueType::JavaScriptCode => SerializationStep::Code,
            ValueType::JavaScriptCodeWithScope => SerializationStep::CodeWithScopeCode,
            ValueType::MinKey => SerializationStep::MinKey,
            ValueType::MaxKey => SerializationStep::MaxKey,
            ValueType::Decimal128 => SerializationStep::Decimal128,
            ValueType::Undefined => SerializationStep::Undefined,
        };
        Self {
            root_serializer: rs,
            state,
        }
    }

    fn invalid_step(&self, primitive_type: &'static str) -> Error {
        Error::custom(format!(
            "cannot serialize {} at step {:?}",
            primitive_type, self.state
        ))
    }
}

impl<'b> serde::Serializer for &'b mut ValueSerializer<'_> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = CodeWithScopeSerializer<'b>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Impossible<(), Error>;

    #[inline]
    fn serialize_bool(self, _v: bool) -> Result<Self::Ok> {
        Err(self.invalid_step("bool"))
    }

    #[inline]
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok> {
        Err(self.invalid_step("i8"))
    }

    #[inline]
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok> {
        Err(self.invalid_step("i16"))
    }

    #[inline]
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok> {
        Err(self.invalid_step("i32"))
    }

    #[inline]
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok> {
        match self.state {
            SerializationStep::TimestampTime => {
                self.state = SerializationStep::TimestampIncrement;
                Ok(())
            }
            SerializationStep::TimestampIncrement => self.root_serializer.add_bytes(8),
            _ => Err(self.invalid_step("i64")),
        }
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        match self.state {
            SerializationStep::RawBinarySubType { ref bytes } => {
                self.root_serializer.add_bin_bytes(bytes.len(), v.into())?;
                self.state = SerializationStep::Done;
                Ok(())
            }
            _ => Err(self.invalid_step("u8")),
        }
    }

    #[inline]
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok> {
        Err(self.invalid_step("u16"))
    }

    #[inline]
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok> {
        Err(self.invalid_step("u32"))
    }

    #[inline]
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok> {
        Err(self.invalid_step("u64"))
    }

    #[inline]
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok> {
        Err(self.invalid_step("f32"))
    }

    #[inline]
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok> {
        Err(self.invalid_step("f64"))
    }

    #[inline]
    fn serialize_char(self, _v: char) -> Result<Self::Ok> {
        Err(self.invalid_step("char"))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        match &self.state {
            SerializationStep::DateTimeNumberLong => {
                self.root_serializer.add_bytes(8)?;
            }
            SerializationStep::Oid => {
                self.root_serializer.add_bytes(12)?;
            }
            SerializationStep::BinaryBytes => {
                self.state = SerializationStep::BinarySubType {
                    base64: v.to_string(),
                };
            }
            SerializationStep::BinarySubType { base64 } => {
                let subtype_byte = hex::decode(v).map_err(Error::custom)?;
                let subtype: BinarySubtype = subtype_byte[0].into();
                let bytes = base64::decode(base64.as_str()).map_err(Error::custom)?;
                self.root_serializer.add_bin_bytes(bytes.len(), subtype)?;
            }
            SerializationStep::Symbol | SerializationStep::DbPointerRef => {
                self.root_serializer.add_str_bytes(v.len())?;
            }
            SerializationStep::RegExPattern => {
                self.root_serializer.add_cstr_bytes(v.len())?;
            }
            SerializationStep::RegExOptions => {
                self.root_serializer.add_cstr_bytes(v.len())?;
            }
            SerializationStep::Code => {
                self.root_serializer.add_str_bytes(v.len())?;
            }
            SerializationStep::CodeWithScopeCode => {
                self.state = SerializationStep::CodeWithScopeScope {
                    code: v.to_string(),
                    raw: false,
                };
            }
            s => {
                return Err(Error::custom(format!(
                    "can't serialize string for step {:?}",
                    s
                )))
            }
        }
        Ok(())
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        match self.state {
            SerializationStep::Decimal128Value => self.root_serializer.add_bytes(16),
            SerializationStep::BinaryBytes => {
                self.state = SerializationStep::RawBinarySubType { bytes: v.to_vec() };
                Ok(())
            }
            SerializationStep::CodeWithScopeScope { ref code, raw } if raw => {
                let raw = RawJavaScriptCodeWithScopeRef {
                    code,
                    scope: RawDocument::from_bytes(v).map_err(Error::custom)?,
                };
                self.root_serializer.add_bytes(4)?;
                self.root_serializer.add_str_bytes(code.len())?;
                self.root_serializer.add_bytes(raw.len())?;
                self.state = SerializationStep::Done;
                Ok(())
            }
            _ => Err(self.invalid_step("&[u8]")),
        }
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok> {
        Err(self.invalid_step("none"))
    }

    #[inline]
    fn serialize_some<T>(self, _value: &T) -> Result<Self::Ok>
    where
        T: Serialize + ?Sized,
    {
        Err(self.invalid_step("some"))
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok> {
        Err(self.invalid_step("unit"))
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        Err(self.invalid_step("unit_struct"))
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok> {
        Err(self.invalid_step("unit_variant"))
    }

    #[inline]
    fn serialize_newtype_struct<T>(self, name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize + ?Sized,
    {
        match (&mut self.state, name) {
            (
                SerializationStep::CodeWithScopeScope {
                    code: _,
                    ref mut raw,
                },
                RAW_DOCUMENT_NEWTYPE,
            ) => {
                *raw = true;
                value.serialize(self)
            }
            _ => Err(self.invalid_step("newtype_struct")),
        }
    }

    #[inline]
    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok>
    where
        T: Serialize + ?Sized,
    {
        Err(self.invalid_step("newtype_variant"))
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(self.invalid_step("seq"))
    }

    #[inline]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(self.invalid_step("newtype_tuple"))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(self.invalid_step("tuple_struct"))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(self.invalid_step("tuple_variant"))
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        match self.state {
            SerializationStep::CodeWithScopeScope { ref code, raw } if !raw => {
                CodeWithScopeSerializer::start(code.as_str(), self.root_serializer)
            }
            _ => Err(self.invalid_step("map")),
        }
    }

    #[inline]
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(self)
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(self.invalid_step("struct_variant"))
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

impl SerializeStruct for &mut ValueSerializer<'_> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        match (&self.state, key) {
            (SerializationStep::DateTime, "$date") => {
                self.state = SerializationStep::DateTimeNumberLong;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::DateTimeNumberLong, "$numberLong") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Oid, "$oid") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Binary, "$binary") => {
                self.state = SerializationStep::BinaryBytes;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::BinaryBytes, key) if key == "bytes" || key == "base64" => {
                // state is updated in serialize
                value.serialize(&mut **self)?;
            }
            (SerializationStep::RawBinarySubType { .. }, "subType") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::BinarySubType { .. }, "subType") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Symbol, "$symbol") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::RegEx, "$regularExpression") => {
                self.state = SerializationStep::RegExPattern;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::RegExPattern, "pattern") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::RegExOptions;
            }
            (SerializationStep::RegExOptions, "options") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Timestamp, "$timestamp") => {
                self.state = SerializationStep::TimestampTime;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::TimestampTime, "t") => {
                // state is updated in serialize
                value.serialize(&mut **self)?;
            }
            (SerializationStep::TimestampIncrement { .. }, "i") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::DbPointer, "$dbPointer") => {
                self.state = SerializationStep::DbPointerRef;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::DbPointerRef, "$ref") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::DbPointerId;
            }
            (SerializationStep::DbPointerId, "$id") => {
                self.state = SerializationStep::Oid;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::Code, "$code") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::CodeWithScopeCode, "$code") => {
                // state is updated in serialize
                value.serialize(&mut **self)?;
            }
            (SerializationStep::CodeWithScopeScope { .. }, "$scope") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::MinKey, "$minKey") => {
                self.state = SerializationStep::Done;
            }
            (SerializationStep::MaxKey, "$maxKey") => {
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Undefined, "$undefined") => {
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Decimal128, "$numberDecimal")
            | (SerializationStep::Decimal128, "$numberDecimalBytes") => {
                self.state = SerializationStep::Decimal128Value;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::Decimal128Value, "$numberDecimal") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Done, k) => {
                return Err(Error::custom(format!(
                    "expected to end serialization of type, got extra key \"{}\"",
                    k
                )));
            }
            (state, k) => {
                return Err(Error::custom(format!(
                    "mismatched serialization step and next key: {:?} + \"{}\"",
                    state, k
                )));
            }
        }

        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

pub(crate) struct CodeWithScopeSerializer<'a> {
    doc: DocumentSerializer<'a>,
}

impl<'a> CodeWithScopeSerializer<'a> {
    #[inline]
    fn start(code: &str, rs: &'a mut Serializer) -> Result<Self> {
        rs.enter_doc();
        rs.add_str_bytes(code.len())?;

        let doc = DocumentSerializer::start(rs)?;
        Ok(Self { doc })
    }
}

impl SerializeMap for CodeWithScopeSerializer<'_> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.doc.serialize_key(key)
    }

    #[inline]
    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.doc.serialize_value(value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        let rs = self.doc.end_doc()?;
        // code with scope does not have an additional null terminator.
        rs.add_bytes(-1)?;
        rs.exit_doc();
        Ok(())
    }
}
