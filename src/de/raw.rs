use std::{
    borrow::Cow,
    io::{ErrorKind, Read},
    sync::Arc,
};

use serde::{
    de::{EnumAccess, Error as SerdeError, IntoDeserializer, MapAccess, VariantAccess},
    forward_to_deserialize_any,
    Deserializer as SerdeDeserializer,
};

use crate::{
    oid::ObjectId,
    raw::{RawBinary, RawBson, RAW_ARRAY_NEWTYPE, RAW_BSON_NEWTYPE, RAW_DOCUMENT_NEWTYPE},
    spec::{BinarySubtype, ElementType},
    uuid::UUID_NEWTYPE_NAME,
    Binary,
    Bson,
    DateTime,
    DbPointer,
    Decimal128,
    JavaScriptCodeWithScope,
    RawDocument,
    Regex,
    Timestamp,
};

use super::{
    read_bool,
    read_f128,
    read_f64,
    read_i32,
    read_i64,
    read_string,
    read_u8,
    Error,
    Result,
    MAX_BSON_SIZE,
};
use crate::de::serde::MapDeserializer;

#[derive(Debug, Clone, Copy)]
enum DeserializerHint {
    None,
    BinarySubtype(BinarySubtype),
    RawBson,
}

/// Deserializer used to parse and deserialize raw BSON bytes.
pub(crate) struct Deserializer<'de> {
    bytes: BsonBuf<'de>,

    /// The type of the element currently being deserialized.
    ///
    /// When the Deserializer is initialized, this will be `ElementType::EmbeddedDocument`, as the
    /// only top level type is a document. The "embedded" portion is incorrect in this context,
    /// but given that there's no difference between deserializing an embedded document and a
    /// top level one, the distinction isn't necessary.
    current_type: ElementType,
}

enum DocumentType {
    Array,
    EmbeddedDocument,
}

impl<'de> Deserializer<'de> {
    pub(crate) fn new(buf: &'de [u8], utf8_lossy: bool) -> Self {
        Self {
            bytes: BsonBuf::new(buf, utf8_lossy),
            current_type: ElementType::EmbeddedDocument,
        }
    }

    /// Ensure the entire document was visited, returning an error if not.
    /// Will read the trailing null byte if necessary (i.e. the visitor stopped after visiting
    /// exactly the number of elements in the document).
    fn end_document(&mut self, length_remaining: i32) -> Result<()> {
        match length_remaining.cmp(&1) {
            std::cmp::Ordering::Equal => {
                let nullbyte = read_u8(&mut self.bytes)?;
                if nullbyte != 0 {
                    return Err(Error::custom(format!(
                        "expected null byte at end of document, got {:#x} instead",
                        nullbyte
                    )));
                }
            }
            std::cmp::Ordering::Greater => {
                return Err(Error::custom(format!(
                    "document has bytes remaining that were not visited: {}",
                    length_remaining
                )));
            }
            std::cmp::Ordering::Less => {
                if length_remaining < 0 {
                    return Err(Error::custom("length of document was too short"));
                }
            }
        }
        Ok(())
    }

    /// Read a string from the BSON.
    ///
    /// If utf8_lossy, this will be an owned string if invalid UTF-8 is encountered in the string,
    /// otherwise it will be borrowed.
    fn deserialize_str(&mut self) -> Result<Cow<'de, str>> {
        self.bytes.read_str()
    }

    fn deserialize_cstr(&mut self) -> Result<Cow<'de, str>> {
        self.bytes.read_cstr()
    }

    fn deserialize_document<V>(
        &mut self,
        visitor: V,
        hint: DeserializerHint,
        document_type: DocumentType,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let is_array = match document_type {
            DocumentType::Array => true,
            DocumentType::EmbeddedDocument => false,
        };

        match hint {
            DeserializerHint::RawBson => {
                let mut len = self.bytes.slice(4)?;
                let len = read_i32(&mut len)?;

                let doc = RawDocument::new(self.bytes.read_slice(len as usize)?)
                    .map_err(Error::custom)?;

                let access = if is_array {
                    RawDocumentAccess::for_array(doc)
                } else {
                    RawDocumentAccess::new(doc)
                };

                visitor.visit_map(access)
            }
            _ if is_array => self.access_document(|access| visitor.visit_seq(access)),
            _ => self.access_document(|access| visitor.visit_map(access)),
        }
    }

    /// Construct a `DocumentAccess` and pass it into the provided closure, returning the
    /// result of the closure if no other errors are encountered.
    fn access_document<F, O>(&mut self, f: F) -> Result<O>
    where
        F: FnOnce(DocumentAccess<'_, 'de>) -> Result<O>,
    {
        println!("in access");
        let mut length_remaining = read_i32(&mut self.bytes)? - 4;
        let out = f(DocumentAccess {
            root_deserializer: self,
            length_remaining: &mut length_remaining,
        });

        if out.is_ok() {
            self.end_document(length_remaining)?;
        }
        out
    }

    /// Deserialize the next element type and update `current_type` accordingly.
    /// Returns `None` if a null byte is read.
    fn deserialize_next_type(&mut self) -> Result<Option<ElementType>> {
        let tag = read_u8(&mut self.bytes)?;
        if tag == 0 {
            return Ok(None);
        }

        let element_type = ElementType::from(tag)
            .ok_or_else(|| Error::custom(format!("invalid element type: {}", tag)))?;

        self.current_type = element_type;
        Ok(Some(element_type))
    }

    fn deserialize_next<V>(&mut self, visitor: V, hint: DeserializerHint) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if let DeserializerHint::BinarySubtype(expected_st) = hint {
            if self.current_type != ElementType::Binary {
                return Err(Error::custom(format!(
                    "expected Binary with subtype {:?}, instead got {:?}",
                    expected_st, self.current_type
                )));
            }
        }

        match self.current_type {
            ElementType::Int32 => visitor.visit_i32(read_i32(&mut self.bytes)?),
            ElementType::Int64 => visitor.visit_i64(read_i64(&mut self.bytes)?),
            ElementType::Double => visitor.visit_f64(read_f64(&mut self.bytes)?),
            ElementType::String => match self.deserialize_str()? {
                Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
                Cow::Owned(string) => visitor.visit_string(string),
            },
            ElementType::Boolean => visitor.visit_bool(read_bool(&mut self.bytes)?),
            ElementType::Null => visitor.visit_unit(),
            ElementType::ObjectId => {
                let oid = ObjectId::from_reader(&mut self.bytes)?;
                visitor.visit_map(ObjectIdAccess::new(oid, hint))
            }
            ElementType::EmbeddedDocument => {
                self.deserialize_document(visitor, hint, DocumentType::EmbeddedDocument)
            }
            ElementType::Array => self.deserialize_document(visitor, hint, DocumentType::Array),
            ElementType::Binary => {
                let len = read_i32(&mut self.bytes)?;
                if !(0..=MAX_BSON_SIZE).contains(&len) {
                    return Err(Error::invalid_length(
                        len as usize,
                        &format!("binary length must be between 0 and {}", MAX_BSON_SIZE).as_str(),
                    ));
                }
                let subtype = BinarySubtype::from(read_u8(&mut self.bytes)?);

                if let DeserializerHint::BinarySubtype(expected_subtype) = hint {
                    if subtype != expected_subtype {
                        return Err(Error::custom(format!(
                            "expected binary subtype {:?} instead got {:?}",
                            expected_subtype, subtype
                        )));
                    }
                }

                match subtype {
                    BinarySubtype::Generic => {
                        visitor.visit_borrowed_bytes(self.bytes.read_slice(len as usize)?)
                    }
                    _ if matches!(hint, DeserializerHint::RawBson) => {
                        let binary = RawBinary::from_slice_with_len_and_payload(
                            self.bytes.read_slice(len as usize)?,
                            len,
                            subtype,
                        )?;
                        let mut d = BinaryDeserializer::borrowed(binary);
                        visitor.visit_map(BinaryAccess {
                            deserializer: &mut d,
                        })
                    }
                    _ => {
                        let binary = Binary::from_reader_with_len_and_payload(
                            &mut self.bytes,
                            len,
                            subtype,
                        )?;
                        let mut d = BinaryDeserializer::new(binary);
                        visitor.visit_map(BinaryAccess {
                            deserializer: &mut d,
                        })
                    }
                }
            }
            ElementType::Undefined => {
                visitor.visit_map(RawBsonAccess::new("$undefined", BsonContent::Boolean(true)))
            }
            ElementType::DateTime => {
                let dti = read_i64(&mut self.bytes)?;
                let dt = DateTime::from_millis(dti);
                let mut d = DateTimeDeserializer::new(dt, hint);
                visitor.visit_map(DateTimeAccess {
                    deserializer: &mut d,
                })
            }
            ElementType::RegularExpression => {
                let mut de = RegexDeserializer::new(&mut *self);
                visitor.visit_map(RegexAccess::new(&mut de))
            }
            ElementType::DbPointer => {
                let mut de = DbPointerDeserializer::new(&mut *self, hint);
                visitor.visit_map(DbPointerAccess::new(&mut de))
            }
            ElementType::JavaScriptCode => {
                let utf8_lossy = self.bytes.utf8_lossy;

                match hint {
                    DeserializerHint::RawBson => visitor.visit_map(RawBsonAccess::new(
                        "$code",
                        BsonContent::Str(self.bytes.read_borrowed_str()?),
                    )),
                    _ => {
                        let code = read_string(&mut self.bytes, utf8_lossy)?;
                        let doc = Bson::JavaScriptCode(code).into_extended_document();
                        visitor.visit_map(MapDeserializer::new(doc))
                    }
                }
            }
            ElementType::JavaScriptCodeWithScope => {
                let _len = read_i32(&mut self.bytes)?;
                let mut de = CodeWithScopeDeserializer::new(&mut *self, hint);
                visitor.visit_map(CodeWithScopeAccess::new(&mut de))
            }
            ElementType::Symbol => {
                let utf8_lossy = self.bytes.utf8_lossy;

                match hint {
                    DeserializerHint::RawBson => visitor.visit_map(RawBsonAccess::new(
                        "$symbol",
                        BsonContent::Str(self.bytes.read_borrowed_str()?),
                    )),
                    _ => {
                        let symbol = read_string(&mut self.bytes, utf8_lossy)?;
                        let doc = Bson::Symbol(symbol).into_extended_document();
                        visitor.visit_map(MapDeserializer::new(doc))
                    }
                }
            }
            ElementType::Timestamp => {
                let ts = Timestamp::from_reader(&mut self.bytes)?;
                let mut d = TimestampDeserializer::new(ts);
                visitor.visit_map(TimestampAccess {
                    deserializer: &mut d,
                })
            }
            ElementType::Decimal128 => {
                let d128 = read_f128(&mut self.bytes)?;
                visitor.visit_map(Decimal128Access::new(d128))
            }
            ElementType::MaxKey => {
                visitor.visit_map(RawBsonAccess::new("$maxKey", BsonContent::Int32(1)))
            }
            ElementType::MinKey => {
                visitor.visit_map(RawBsonAccess::new("$minKey", BsonContent::Int32(1)))
            }
        }
    }
}

impl<'de, 'a> serde::de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_next(visitor, DeserializerHint::None)
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.current_type {
            ElementType::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.current_type {
            ElementType::String => visitor.visit_enum(self.deserialize_str()?.into_deserializer()),
            ElementType::EmbeddedDocument => {
                self.access_document(|access| visitor.visit_enum(access))
            }
            t => Err(Error::custom(format!("expected enum, instead got {:?}", t))),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.current_type {
            ElementType::ObjectId => visitor.visit_borrowed_bytes(self.bytes.read_slice(12)?),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_newtype_struct<V>(self, name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match name {
            UUID_NEWTYPE_NAME => self.deserialize_next(
                visitor,
                DeserializerHint::BinarySubtype(BinarySubtype::Uuid),
            ),
            RAW_BSON_NEWTYPE => self.deserialize_next(visitor, DeserializerHint::RawBson),
            _ => visitor.visit_newtype_struct(self),
        }
    }

    fn is_human_readable(&self) -> bool {
        false
    }

    forward_to_deserialize_any! {
        bool char str byte_buf unit unit_struct string
        identifier seq tuple tuple_struct struct
        map ignored_any i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
    }
}

/// Struct for accessing documents for deserialization purposes.
/// This is used to deserialize maps, structs, sequences, and enums.
struct DocumentAccess<'d, 'de> {
    root_deserializer: &'d mut Deserializer<'de>,
    length_remaining: &'d mut i32,
}

impl<'d, 'de> DocumentAccess<'d, 'de> {
    /// Read the next element type and update the root deserializer with it.
    ///
    /// Returns `Ok(None)` if the document has been fully read and has no more elements.
    fn read_next_type(&mut self) -> Result<Option<ElementType>> {
        let t = self.read(|s| s.root_deserializer.deserialize_next_type())?;

        if t.is_none() && *self.length_remaining != 0 {
            return Err(Error::custom(format!(
                "got null byte but still have length {} remaining",
                self.length_remaining
            )));
        }

        Ok(t)
    }

    /// Executes a closure that reads from the BSON bytes and returns an error if the number of
    /// bytes read exceeds length_remaining.
    ///
    /// A mutable reference to this `DocumentAccess` is passed into the closure.
    fn read<F, O>(&mut self, f: F) -> Result<O>
    where
        F: FnOnce(&mut Self) -> Result<O>,
    {
        let start_bytes = self.root_deserializer.bytes.bytes_read();
        let out = f(self);
        let bytes_read = self.root_deserializer.bytes.bytes_read() - start_bytes;
        *self.length_remaining -= bytes_read as i32;

        if *self.length_remaining < 0 {
            return Err(Error::custom("length of document too short"));
        }
        out
    }

    /// Read the next value from the document.
    fn read_next_value<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        self.read(|s| seed.deserialize(&mut *s.root_deserializer))
    }
}

impl<'d, 'de> serde::de::MapAccess<'de> for DocumentAccess<'d, 'de> {
    type Error = crate::de::Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if self.read_next_type()?.is_none() {
            return Ok(None);
        }

        self.read(|s| {
            seed.deserialize(DocumentKeyDeserializer {
                root_deserializer: &mut *s.root_deserializer,
            })
        })
        .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        self.read_next_value(seed)
    }
}

impl<'d, 'de> serde::de::SeqAccess<'de> for DocumentAccess<'d, 'de> {
    type Error = Error;

    fn next_element_seed<S>(&mut self, seed: S) -> Result<Option<S::Value>>
    where
        S: serde::de::DeserializeSeed<'de>,
    {
        if self.read_next_type()?.is_none() {
            return Ok(None);
        }
        let _index = self.read(|s| s.root_deserializer.deserialize_cstr())?;
        self.read_next_value(seed).map(Some)
    }
}

impl<'d, 'de> EnumAccess<'de> for DocumentAccess<'d, 'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(mut self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        if self.read_next_type()?.is_none() {
            return Err(Error::EndOfStream);
        }

        let key = self.read(|s| {
            seed.deserialize(DocumentKeyDeserializer {
                root_deserializer: &mut *s.root_deserializer,
            })
        })?;

        Ok((key, self))
    }
}

impl<'d, 'de> VariantAccess<'de> for DocumentAccess<'d, 'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Err(Error::custom(
            "expected a string enum, got a document instead",
        ))
    }

    fn newtype_variant_seed<S>(mut self, seed: S) -> Result<S::Value>
    where
        S: serde::de::DeserializeSeed<'de>,
    {
        self.read_next_value(seed)
    }

    fn tuple_variant<V>(mut self, _len: usize, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.read(|s| s.root_deserializer.deserialize_seq(visitor))
    }

    fn struct_variant<V>(mut self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.read(|s| s.root_deserializer.deserialize_map(visitor))
    }
}

/// Deserializer used specifically for deserializing a document's cstring keys.
struct DocumentKeyDeserializer<'d, 'de> {
    root_deserializer: &'d mut Deserializer<'de>,
}

impl<'d, 'de> serde::de::Deserializer<'de> for DocumentKeyDeserializer<'d, 'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let s = self.root_deserializer.deserialize_cstr()?;
        match s {
            Cow::Borrowed(b) => visitor.visit_borrowed_str(b),
            Cow::Owned(string) => visitor.visit_string(string),
        }
    }

    forward_to_deserialize_any! {
        bool char str bytes byte_buf option unit unit_struct string
        identifier newtype_struct seq tuple tuple_struct struct map enum
        ignored_any i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
    }
}

/// Deserializer used to deserialize the given field name without any copies.
struct FieldDeserializer {
    field_name: &'static str,
}

impl<'de> serde::de::Deserializer<'de> for FieldDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.field_name)
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

struct RawDocumentAccess<'d> {
    deserializer: RawDocumentDeserializer<'d>,
    first: bool,
    array: bool,
}

impl<'de> RawDocumentAccess<'de> {
    fn new(doc: &'de RawDocument) -> Self {
        Self {
            deserializer: RawDocumentDeserializer { raw_doc: doc },
            first: true,
            array: false,
        }
    }

    fn for_array(doc: &'de RawDocument) -> Self {
        Self {
            deserializer: RawDocumentDeserializer { raw_doc: doc },
            first: true,
            array: true,
        }
    }
}

impl<'de> serde::de::MapAccess<'de> for RawDocumentAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if self.first {
            self.first = false;
            seed.deserialize(FieldDeserializer {
                field_name: if self.array {
                    RAW_ARRAY_NEWTYPE
                } else {
                    RAW_DOCUMENT_NEWTYPE
                },
            })
            .map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.deserializer)
    }
}

#[derive(Clone, Copy)]
struct RawDocumentDeserializer<'a> {
    raw_doc: &'a RawDocument,
}

impl<'de> serde::de::Deserializer<'de> for RawDocumentDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_borrowed_bytes(self.raw_doc.as_bytes())
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

struct ObjectIdAccess {
    oid: ObjectId,
    visited: bool,
    hint: DeserializerHint,
}

impl ObjectIdAccess {
    fn new(oid: ObjectId, hint: DeserializerHint) -> Self {
        Self {
            oid,
            visited: false,
            hint,
        }
    }
}

impl<'de> serde::de::MapAccess<'de> for ObjectIdAccess {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if self.visited {
            return Ok(None);
        }
        self.visited = true;
        seed.deserialize(FieldDeserializer { field_name: "$oid" })
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(ObjectIdDeserializer {
            oid: self.oid,
            hint: self.hint,
        })
    }
}

struct ObjectIdDeserializer {
    oid: ObjectId,
    hint: DeserializerHint,
}

impl<'de> serde::de::Deserializer<'de> for ObjectIdDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        println!("oid hint {:?}", self.hint);
        println!("visitor: {:?}", std::any::type_name::<V>());
        // save an allocation when deserializing to raw bson
        match self.hint {
            DeserializerHint::RawBson => visitor.visit_bytes(&self.oid.bytes()),
            _ => visitor.visit_string(self.oid.to_hex()),
        }
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

pub(crate) struct Decimal128Access {
    decimal: Decimal128,
    visited: bool,
}

impl Decimal128Access {
    pub(crate) fn new(decimal: Decimal128) -> Self {
        Self {
            decimal,
            visited: false,
        }
    }
}

impl<'de> serde::de::MapAccess<'de> for Decimal128Access {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if self.visited {
            return Ok(None);
        }
        self.visited = true;
        seed.deserialize(FieldDeserializer {
            field_name: "$numberDecimalBytes",
        })
        .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(Decimal128Deserializer(self.decimal))
    }
}

struct Decimal128Deserializer(Decimal128);

impl<'de> serde::de::Deserializer<'de> for Decimal128Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_bytes(&self.0.bytes)
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

enum TimestampDeserializationStage {
    TopLevel,
    Time,
    Increment,
    Done,
}

struct TimestampAccess<'d> {
    deserializer: &'d mut TimestampDeserializer,
}

impl<'de, 'd> serde::de::MapAccess<'de> for TimestampAccess<'d> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.deserializer.stage {
            TimestampDeserializationStage::TopLevel => seed
                .deserialize(FieldDeserializer {
                    field_name: "$timestamp",
                })
                .map(Some),
            TimestampDeserializationStage::Time => seed
                .deserialize(FieldDeserializer { field_name: "t" })
                .map(Some),
            TimestampDeserializationStage::Increment => seed
                .deserialize(FieldDeserializer { field_name: "i" })
                .map(Some),
            TimestampDeserializationStage::Done => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.deserializer)
    }
}

struct TimestampDeserializer {
    ts: Timestamp,
    stage: TimestampDeserializationStage,
}

impl TimestampDeserializer {
    fn new(ts: Timestamp) -> Self {
        Self {
            ts,
            stage: TimestampDeserializationStage::TopLevel,
        }
    }
}

impl<'de, 'a> serde::de::Deserializer<'de> for &'a mut TimestampDeserializer {
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.stage {
            TimestampDeserializationStage::TopLevel => {
                self.stage = TimestampDeserializationStage::Time;
                visitor.visit_map(TimestampAccess {
                    deserializer: &mut self,
                })
            }
            TimestampDeserializationStage::Time => {
                self.stage = TimestampDeserializationStage::Increment;
                visitor.visit_u32(self.ts.time)
            }
            TimestampDeserializationStage::Increment => {
                self.stage = TimestampDeserializationStage::Done;
                visitor.visit_u32(self.ts.increment)
            }
            TimestampDeserializationStage::Done => {
                Err(Error::custom("timestamp fully deserialized already"))
            }
        }
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

enum DateTimeDeserializationStage {
    TopLevel,
    NumberLong,
    Done,
}

struct DateTimeAccess<'d> {
    deserializer: &'d mut DateTimeDeserializer,
}

// impl<'d> DateTimeAccess<'d> {
//     fn new(deserializer: &'d mut DateTimeDeserializer) -> Self {
//         Self {

//         }
//     }
// }

impl<'de, 'd> serde::de::MapAccess<'de> for DateTimeAccess<'d> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.deserializer.stage {
            DateTimeDeserializationStage::TopLevel => seed
                .deserialize(FieldDeserializer {
                    field_name: "$date",
                })
                .map(Some),
            DateTimeDeserializationStage::NumberLong => seed
                .deserialize(FieldDeserializer {
                    field_name: "$numberLong",
                })
                .map(Some),
            DateTimeDeserializationStage::Done => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.deserializer)
    }
}

struct DateTimeDeserializer {
    dt: DateTime,
    stage: DateTimeDeserializationStage,
    hint: DeserializerHint,
}

impl DateTimeDeserializer {
    fn new(dt: DateTime, hint: DeserializerHint) -> Self {
        Self {
            dt,
            stage: DateTimeDeserializationStage::TopLevel,
            hint,
        }
    }
}

impl<'de, 'a> serde::de::Deserializer<'de> for &'a mut DateTimeDeserializer {
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.stage {
            DateTimeDeserializationStage::TopLevel => match self.hint {
                DeserializerHint::RawBson => {
                    self.stage = DateTimeDeserializationStage::Done;
                    visitor.visit_i64(self.dt.timestamp_millis())
                }
                _ => {
                    self.stage = DateTimeDeserializationStage::NumberLong;
                    visitor.visit_map(DateTimeAccess {
                        deserializer: &mut self,
                    })
                }
            },
            DateTimeDeserializationStage::NumberLong => {
                self.stage = DateTimeDeserializationStage::Done;
                visitor.visit_string(self.dt.timestamp_millis().to_string())
            }
            DateTimeDeserializationStage::Done => {
                Err(Error::custom("DateTime fully deserialized already"))
            }
        }
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

struct BinaryAccess<'d, 'de> {
    deserializer: &'d mut BinaryDeserializer<'de>,
}

impl<'de, 'd> serde::de::MapAccess<'de> for BinaryAccess<'d, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        match self.deserializer.stage {
            BinaryDeserializationStage::TopLevel => seed
                .deserialize(FieldDeserializer {
                    field_name: "$binary",
                })
                .map(Some),
            BinaryDeserializationStage::Subtype => seed
                .deserialize(FieldDeserializer {
                    field_name: "subType",
                })
                .map(Some),
            BinaryDeserializationStage::Bytes => seed
                .deserialize(FieldDeserializer {
                    field_name: "base64",
                })
                .map(Some),
            BinaryDeserializationStage::Done => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.deserializer)
    }
}

enum BinaryContent<'a> {
    Borrowed(RawBinary<'a>),
    Owned(Binary),
}

struct BinaryDeserializer<'a> {
    binary: BinaryContent<'a>,
    stage: BinaryDeserializationStage,
}

impl BinaryDeserializer<'static> {
    fn new(binary: Binary) -> Self {
        Self {
            binary: BinaryContent::Owned(binary),
            stage: BinaryDeserializationStage::TopLevel,
        }
    }
}

impl<'a> BinaryDeserializer<'a> {
    fn borrowed(binary: RawBinary<'a>) -> Self {
        Self {
            binary: BinaryContent::Borrowed(binary),
            stage: BinaryDeserializationStage::TopLevel,
        }
    }
}

impl<'de, 'a> serde::de::Deserializer<'de> for &'a mut BinaryDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.stage {
            BinaryDeserializationStage::TopLevel => {
                self.stage = BinaryDeserializationStage::Subtype;
                visitor.visit_map(BinaryAccess {
                    deserializer: &mut self,
                })
            }
            BinaryDeserializationStage::Subtype => {
                self.stage = BinaryDeserializationStage::Bytes;
                match self.binary {
                    BinaryContent::Owned(ref b) => {
                        visitor.visit_string(hex::encode([u8::from(b.subtype)]))
                    }
                    BinaryContent::Borrowed(b) => visitor.visit_u8(b.subtype().into()),
                }
            }
            BinaryDeserializationStage::Bytes => {
                self.stage = BinaryDeserializationStage::Done;
                match self.binary {
                    BinaryContent::Owned(ref b) => {
                        visitor.visit_string(base64::encode(b.bytes.as_slice()))
                    }
                    BinaryContent::Borrowed(b) => visitor.visit_borrowed_bytes(b.as_bytes()),
                }
            }
            BinaryDeserializationStage::Done => {
                Err(Error::custom("Binary fully deserialized already"))
            }
        }
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

enum BinaryDeserializationStage {
    TopLevel,
    Subtype,
    Bytes,
    Done,
}

struct CodeWithScopeAccess<'de, 'd, 'a> {
    deserializer: &'a mut CodeWithScopeDeserializer<'de, 'd>,
}

impl<'de, 'd, 'a> CodeWithScopeAccess<'de, 'd, 'a> {
    fn new(deserializer: &'a mut CodeWithScopeDeserializer<'de, 'd>) -> Self {
        Self { deserializer }
    }
}

impl<'de, 'd, 'a> serde::de::MapAccess<'de> for CodeWithScopeAccess<'de, 'd, 'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        println!("key: {:?}", self.deserializer.stage);
        match self.deserializer.stage {
            CodeWithScopeDeserializationStage::Code => seed
                .deserialize(FieldDeserializer {
                    field_name: "$code",
                })
                .map(Some),
            CodeWithScopeDeserializationStage::Scope => seed
                .deserialize(FieldDeserializer {
                    field_name: "$scope",
                })
                .map(Some),
            CodeWithScopeDeserializationStage::Done => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.deserializer)
    }
}

struct CodeWithScopeDeserializer<'de, 'a> {
    root_deserializer: &'a mut Deserializer<'de>,
    stage: CodeWithScopeDeserializationStage,
    hint: DeserializerHint,
}

impl<'de, 'a> CodeWithScopeDeserializer<'de, 'a> {
    fn new(root_deserializer: &'a mut Deserializer<'de>, hint: DeserializerHint) -> Self {
        Self {
            root_deserializer,
            stage: CodeWithScopeDeserializationStage::Code,
            hint,
        }
    }
}

impl<'de, 'a, 'b> serde::de::Deserializer<'de> for &'b mut CodeWithScopeDeserializer<'de, 'a> {
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.stage {
            CodeWithScopeDeserializationStage::Code => {
                self.stage = CodeWithScopeDeserializationStage::Scope;
                match self.root_deserializer.deserialize_str()? {
                    Cow::Borrowed(s) => {
                        println!("visiting code: {}", s);
                        visitor.visit_borrowed_str(s)
                    }
                    Cow::Owned(s) => visitor.visit_string(s),
                }
            }
            CodeWithScopeDeserializationStage::Scope => {
                self.stage = CodeWithScopeDeserializationStage::Done;
                println!("deserializing scope");
                self.root_deserializer.deserialize_document(
                    visitor,
                    self.hint,
                    DocumentType::EmbeddedDocument,
                )
            }
            CodeWithScopeDeserializationStage::Done => Err(Error::custom(
                "JavaScriptCodeWithScope fully deserialized already",
            )),
        }
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

#[derive(Debug)]
enum CodeWithScopeDeserializationStage {
    Code,
    Scope,
    Done,
}

struct DbPointerAccess<'de, 'd, 'a> {
    deserializer: &'a mut DbPointerDeserializer<'de, 'd>,
}

impl<'de, 'd, 'a> DbPointerAccess<'de, 'd, 'a> {
    fn new(deserializer: &'a mut DbPointerDeserializer<'de, 'd>) -> Self {
        Self { deserializer }
    }
}

impl<'de, 'd, 'a> serde::de::MapAccess<'de> for DbPointerAccess<'de, 'd, 'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        println!("key: {:?}", self.deserializer.stage);
        match self.deserializer.stage {
            DbPointerDeserializationStage::TopLevel => seed
                .deserialize(FieldDeserializer {
                    field_name: "$dbPointer",
                })
                .map(Some),
            DbPointerDeserializationStage::Namespace => seed
                .deserialize(FieldDeserializer { field_name: "$ref" })
                .map(Some),
            DbPointerDeserializationStage::Id => seed
                .deserialize(FieldDeserializer { field_name: "$id" })
                .map(Some),
            DbPointerDeserializationStage::Done => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.deserializer)
    }
}

struct DbPointerDeserializer<'de, 'a> {
    root_deserializer: &'a mut Deserializer<'de>,
    stage: DbPointerDeserializationStage,
    hint: DeserializerHint,
}

impl<'de, 'a> DbPointerDeserializer<'de, 'a> {
    fn new(root_deserializer: &'a mut Deserializer<'de>, hint: DeserializerHint) -> Self {
        Self {
            root_deserializer,
            stage: DbPointerDeserializationStage::TopLevel,
            hint,
        }
    }
}

impl<'de, 'a, 'b> serde::de::Deserializer<'de> for &'b mut DbPointerDeserializer<'de, 'a> {
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        println!("deserializing {:?}", self.stage);
        match self.stage {
            DbPointerDeserializationStage::TopLevel => {
                self.stage = DbPointerDeserializationStage::Namespace;
                visitor.visit_map(DbPointerAccess::new(self))
            }
            DbPointerDeserializationStage::Namespace => {
                self.stage = DbPointerDeserializationStage::Id;
                match self.root_deserializer.deserialize_str()? {
                    Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
                    Cow::Owned(s) => visitor.visit_string(s),
                }
            }
            DbPointerDeserializationStage::Id => {
                self.stage = DbPointerDeserializationStage::Done;
                visitor.visit_borrowed_bytes(self.root_deserializer.bytes.read_slice(12)?)
            }
            DbPointerDeserializationStage::Done => {
                Err(Error::custom("DbPointer fully deserialized already"))
            }
        }
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

#[derive(Debug)]
enum DbPointerDeserializationStage {
    TopLevel,
    Namespace,
    Id,
    Done,
}

struct RegexAccess<'de, 'd, 'a> {
    deserializer: &'a mut RegexDeserializer<'de, 'd>,
}

impl<'de, 'd, 'a> RegexAccess<'de, 'd, 'a> {
    fn new(deserializer: &'a mut RegexDeserializer<'de, 'd>) -> Self {
        Self { deserializer }
    }
}

impl<'de, 'd, 'a> serde::de::MapAccess<'de> for RegexAccess<'de, 'd, 'a> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        println!("key: {:?}", self.deserializer.stage);
        match self.deserializer.stage {
            RegexDeserializationStage::TopLevel => seed
                .deserialize(FieldDeserializer {
                    field_name: "$regularExpression",
                })
                .map(Some),
            RegexDeserializationStage::Pattern => seed
                .deserialize(FieldDeserializer {
                    field_name: "pattern",
                })
                .map(Some),
            RegexDeserializationStage::Options => seed
                .deserialize(FieldDeserializer {
                    field_name: "options",
                })
                .map(Some),
            RegexDeserializationStage::Done => Ok(None),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.deserializer)
    }
}

struct RegexDeserializer<'de, 'a> {
    root_deserializer: &'a mut Deserializer<'de>,
    stage: RegexDeserializationStage,
}

impl<'de, 'a> RegexDeserializer<'de, 'a> {
    fn new(root_deserializer: &'a mut Deserializer<'de>) -> Self {
        Self {
            root_deserializer,
            stage: RegexDeserializationStage::TopLevel,
        }
    }
}

impl<'de, 'a, 'b> serde::de::Deserializer<'de> for &'b mut RegexDeserializer<'de, 'a> {
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.stage {
            RegexDeserializationStage::TopLevel => {
                self.stage.advance();
                visitor.visit_map(RegexAccess::new(self))
            }
            RegexDeserializationStage::Pattern | RegexDeserializationStage::Options => {
                self.stage.advance();
                match self.root_deserializer.deserialize_cstr()? {
                    Cow::Borrowed(s) => visitor.visit_borrowed_str(s),
                    Cow::Owned(s) => visitor.visit_string(s),
                }
            }
            RegexDeserializationStage::Done => {
                Err(Error::custom("DbPointer fully deserialized already"))
            }
        }
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

#[derive(Debug)]
enum RegexDeserializationStage {
    TopLevel,
    Pattern,
    Options,
    Done,
}

impl RegexDeserializationStage {
    fn advance(&mut self) {
        *self = match self {
            RegexDeserializationStage::TopLevel => RegexDeserializationStage::Pattern,
            RegexDeserializationStage::Pattern => RegexDeserializationStage::Options,
            RegexDeserializationStage::Options => RegexDeserializationStage::Done,
            RegexDeserializationStage::Done => RegexDeserializationStage::Done,
        }
    }
}

/// Helper access struct for visiting the extended JSON model of simple BSON types.
/// e.g. Symbol, Timestamp, etc.
struct RawBsonAccess<'a> {
    key: &'static str,
    value: BsonContent<'a>,
    first: bool,
}

/// Enum value representing some cached BSON data needed to represent a given
/// BSON type's extended JSON model.
#[derive(Debug, Clone, Copy)]
enum BsonContent<'a> {
    Str(&'a str),
    Int32(i32),
    Boolean(bool),
}

impl<'a> RawBsonAccess<'a> {
    fn new(key: &'static str, value: BsonContent<'a>) -> Self {
        Self {
            key,
            value,
            first: true,
        }
    }
}

impl<'de> MapAccess<'de> for RawBsonAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        if self.first {
            self.first = false;
            seed.deserialize(FieldDeserializer {
                field_name: self.key,
            })
            .map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(RawBsonDeserializer { value: self.value })
    }
}

struct RawBsonDeserializer<'a> {
    value: BsonContent<'a>,
}

impl<'de, 'a> serde::de::Deserializer<'de> for RawBsonDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.value {
            BsonContent::Boolean(b) => visitor.visit_bool(b),
            BsonContent::Str(s) => visitor.visit_borrowed_str(s),
            BsonContent::Int32(i) => visitor.visit_i32(i),
        }
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

/// Struct wrapping a slice of BSON bytes.
struct BsonBuf<'a> {
    bytes: &'a [u8],
    index: usize,

    /// Whether or not to insert replacement characters in place of invalid UTF-8 sequences when
    /// deserializing strings.
    utf8_lossy: bool,
}

impl<'a> Read for BsonBuf<'a> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.index_check()?;
        let bytes_read = self.bytes[self.index..].as_ref().read(buf)?;
        self.index += bytes_read;
        Ok(bytes_read)
    }
}

impl<'a> BsonBuf<'a> {
    fn new(bytes: &'a [u8], utf8_lossy: bool) -> Self {
        Self {
            bytes,
            index: 0,
            utf8_lossy,
        }
    }

    fn bytes_read(&self) -> usize {
        self.index
    }

    /// Verify the index has not run out of bounds.
    fn index_check(&self) -> std::io::Result<()> {
        if self.index >= self.bytes.len() {
            return Err(ErrorKind::UnexpectedEof.into());
        }
        Ok(())
    }

    /// Get the string starting at the provided index and ending at the buffer's current index.
    ///
    /// Can optionally override the global UTF-8 lossy setting to ensure bytes are not allocated.
    fn str(&mut self, start: usize, utf8_lossy_override: Option<bool>) -> Result<Cow<'a, str>> {
        let bytes = &self.bytes[start..self.index];
        let s = if utf8_lossy_override.unwrap_or(self.utf8_lossy) {
            String::from_utf8_lossy(bytes)
        } else {
            Cow::Borrowed(std::str::from_utf8(bytes).map_err(Error::custom)?)
        };

        // consume the null byte
        if self.bytes[self.index] != 0 {
            return Err(Error::custom("string was not null-terminated"));
        }
        self.index += 1;
        self.index_check()?;

        Ok(s)
    }

    /// Attempts to read a null-terminated UTF-8 cstring from the data.
    ///
    /// If utf8_lossy and invalid UTF-8 is encountered, the unicode replacement character will be
    /// inserted in place of the offending data, resulting in an owned `String`. Otherwise, the
    /// data will be borrowed as-is.
    fn read_cstr(&mut self) -> Result<Cow<'a, str>> {
        let start = self.index;
        while self.index < self.bytes.len() && self.bytes[self.index] != 0 {
            self.index += 1
        }

        self.index_check()?;

        self.str(start, None)
    }

    fn advance_to_str(&mut self) -> Result<usize> {
        let len = read_i32(self)?;
        let start = self.index;

        // UTF-8 String must have at least 1 byte (the last 0x00).
        if len < 1 {
            return Err(Error::invalid_length(
                len as usize,
                &"UTF-8 string must have at least 1 byte",
            ));
        }

        self.index += (len - 1) as usize;
        self.index_check()?;

        Ok(start)
    }

    /// Attempts to read a null-terminated UTF-8 string from the data.
    ///
    /// If invalid UTF-8 is encountered, the unicode replacement character will be inserted in place
    /// of the offending data, resulting in an owned `String`. Otherwise, the data will be
    /// borrowed as-is.
    fn read_str(&mut self) -> Result<Cow<'a, str>> {
        let start = self.advance_to_str()?;
        self.str(start, None)
    }

    /// Attempts to read a null-terminated UTF-8 string from the data.
    fn read_borrowed_str(&mut self) -> Result<&'a str> {
        let start = self.advance_to_str()?;
        match self.str(start, Some(false))? {
            Cow::Borrowed(s) => Ok(s),
            Cow::Owned(_) => panic!("should have errored when encountering invalid UTF-8"),
        }
    }

    fn slice(&self, length: usize) -> Result<&'a [u8]> {
        if self.index + length > self.bytes.len() {
            return Err(Error::Io(Arc::new(
                std::io::ErrorKind::UnexpectedEof.into(),
            )));
        }

        Ok(&self.bytes[self.index..(self.index + length)])
    }

    fn read_slice(&mut self, length: usize) -> Result<&'a [u8]> {
        let slice = self.slice(length)?;
        self.index += length;
        Ok(slice)
    }
}
