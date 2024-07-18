use std::borrow::Cow;

use serde::{
    de::{value::BorrowedStrDeserializer, Error as SerdeError, IntoDeserializer, MapAccess},
    forward_to_deserialize_any,
    Deserializer as SerdeDeserializer,
};

use crate::{
    oid::ObjectId,
    raw::{
        RawBinaryRef,
        RawElement,
        RawIter,
        Utf8LossyBson,
        Utf8LossyJavaScriptCodeWithScope,
        RAW_ARRAY_NEWTYPE,
        RAW_BSON_NEWTYPE,
        RAW_DOCUMENT_NEWTYPE,
    },
    serde_helpers::HUMAN_READABLE_NEWTYPE,
    spec::{BinarySubtype, ElementType},
    uuid::UUID_NEWTYPE_NAME,
    DateTime,
    DbPointer,
    Decimal128,
    RawBsonRef,
    RawDbPointerRef,
    RawDocument,
    RawJavaScriptCodeWithScopeRef,
    RawRegexRef,
    Regex,
    Timestamp,
};

use super::{DeserializerHint, Error, Result};
use crate::de::serde::MapDeserializer;

/// Deserializer mapping from raw bson to serde's data model.
pub(crate) struct Deserializer<'de> {
    element: RawElement<'de>,
    options: DeserializerOptions,
}

#[derive(Debug, Clone)]
struct DeserializerOptions {
    utf8_lossy: bool,
    human_readable: bool,
}

impl<'de> Deserializer<'de> {
    pub(crate) fn new(buf: &'de [u8], utf8_lossy: bool) -> Result<Self> {
        Ok(Self {
            element: RawElement::toplevel(buf).map_err(Error::deserialization)?,
            options: DeserializerOptions {
                utf8_lossy,
                human_readable: false,
            },
        })
    }

    fn value(&self) -> Result<RawBsonRef<'de>> {
        self.element.value().map_err(Error::deserialization)
    }

    /// Deserialize the element, using the type of the element along with the
    /// provided hint to determine how to visit the data.
    fn deserialize_hint<V>(&self, visitor: V, hint: DeserializerHint) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        if self.options.utf8_lossy {
            if let Some(lossy) = self
                .element
                .value_utf8_lossy()
                .map_err(Error::deserialization)?
            {
                return match lossy {
                    Utf8LossyBson::String(s) => visitor.visit_string(s),
                    Utf8LossyBson::RegularExpression(re) => {
                        visitor.visit_map(RegexAccess::new(BsonCow::Owned(re)))
                    }
                    Utf8LossyBson::JavaScriptCode(code) => visitor.visit_map(MapDeserializer::new(
                        doc! { "$code": code },
                        #[allow(deprecated)]
                        crate::DeserializerOptions::builder()
                            .human_readable(false)
                            .build(),
                    )),
                    Utf8LossyBson::JavaScriptCodeWithScope(jsc) => visitor.visit_map(
                        CodeWithScopeAccess::new(BsonCow::Owned(jsc), hint, self.options.clone()),
                    ),
                    Utf8LossyBson::DbPointer(dbp) => {
                        visitor.visit_map(DbPointerAccess::new(BsonCow::Owned(dbp), hint))
                    }
                    Utf8LossyBson::Symbol(s) => visitor.visit_map(MapDeserializer::new(
                        doc! { "$symbol": s },
                        #[allow(deprecated)]
                        crate::DeserializerOptions::builder()
                            .human_readable(false)
                            .build(),
                    )),
                };
            }
        }
        match self.value()? {
            RawBsonRef::Int32(i) => visitor.visit_i32(i),
            RawBsonRef::Int64(i) => visitor.visit_i64(i),
            RawBsonRef::Double(d) => visitor.visit_f64(d),
            RawBsonRef::String(s) => visitor.visit_borrowed_str(s),
            RawBsonRef::Boolean(b) => visitor.visit_bool(b),
            RawBsonRef::Null => visitor.visit_unit(),
            RawBsonRef::ObjectId(oid) => visitor.visit_map(ObjectIdAccess::new(oid, hint)),
            RawBsonRef::Document(doc) => match hint {
                DeserializerHint::RawBson => visitor.visit_map(RawDocumentAccess::new(doc)),
                _ => visitor.visit_map(DocumentAccess::new(doc, self.options.clone())?),
            },
            RawBsonRef::Array(arr) => match hint {
                DeserializerHint::RawBson => {
                    visitor.visit_map(RawDocumentAccess::for_array(arr.as_doc()))
                }
                _ => visitor.visit_seq(DocumentAccess::new(arr.as_doc(), self.options.clone())?),
            },
            RawBsonRef::Binary(bin) => {
                if let DeserializerHint::BinarySubtype(expected_subtype) = hint {
                    if bin.subtype != expected_subtype {
                        return Err(Error::custom(format!(
                            "expected binary subtype {:?} instead got {:?}",
                            expected_subtype, bin.subtype
                        )));
                    }
                }

                match bin.subtype {
                    BinarySubtype::Generic => visitor.visit_borrowed_bytes(bin.bytes),
                    _ => {
                        let mut d = BinaryDeserializer::new(bin, hint);
                        visitor.visit_map(BinaryAccess {
                            deserializer: &mut d,
                        })
                    }
                }
            }
            RawBsonRef::Undefined => {
                visitor.visit_map(RawBsonAccess::new("$undefined", BsonContent::Boolean(true)))
            }
            RawBsonRef::DateTime(dt) => {
                let mut d = DateTimeDeserializer::new(dt, hint);
                visitor.visit_map(DateTimeAccess {
                    deserializer: &mut d,
                })
            }
            RawBsonRef::RegularExpression(re) => {
                visitor.visit_map(RegexAccess::new(BsonCow::Borrowed(re)))
            }
            RawBsonRef::DbPointer(dbp) => {
                visitor.visit_map(DbPointerAccess::new(BsonCow::Borrowed(dbp), hint))
            }
            RawBsonRef::JavaScriptCode(s) => {
                visitor.visit_map(RawBsonAccess::new("$code", BsonContent::Str(s)))
            }
            RawBsonRef::JavaScriptCodeWithScope(jsc) => visitor.visit_map(
                CodeWithScopeAccess::new(BsonCow::Borrowed(jsc), hint, self.options.clone()),
            ),
            RawBsonRef::Symbol(s) => {
                visitor.visit_map(RawBsonAccess::new("$symbol", BsonContent::Str(s)))
            }
            RawBsonRef::Timestamp(ts) => {
                let mut d = TimestampDeserializer::new(ts);
                visitor.visit_map(TimestampAccess {
                    deserializer: &mut d,
                })
            }
            RawBsonRef::Decimal128(d128) => visitor.visit_map(Decimal128Access::new(d128)),
            RawBsonRef::MaxKey => {
                visitor.visit_map(RawBsonAccess::new("$maxKey", BsonContent::Int32(1)))
            }
            RawBsonRef::MinKey => {
                visitor.visit_map(RawBsonAccess::new("$minKey", BsonContent::Int32(1)))
            }
        }
    }

    fn get_string(&self) -> Result<Cow<'de, str>> {
        if self.options.utf8_lossy {
            let value = self
                .element
                .value_utf8_lossy()
                .map_err(Error::deserialization)?;
            let s = match value {
                Some(Utf8LossyBson::String(s)) => s,
                _ => {
                    return Err(Error::deserialization(
                        "internal error: unexpected non-string",
                    ))
                }
            };
            Ok(Cow::Owned(s))
        } else {
            match self.value()? {
                RawBsonRef::String(s) => Ok(Cow::Borrowed(s)),
                _ => {
                    return Err(Error::deserialization(
                        "internal error: unexpected non-string",
                    ))
                }
            }
        }
    }
}

impl<'de> serde::de::Deserializer<'de> for Deserializer<'de> {
    type Error = Error;

    #[inline]
    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_hint(visitor, DeserializerHint::None)
    }

    #[inline]
    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.element.element_type() {
            ElementType::Null => visitor.visit_none(),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.element.element_type() {
            ElementType::String => visitor.visit_enum(self.get_string()?.into_deserializer()),
            ElementType::EmbeddedDocument => {
                let doc = match self.value()? {
                    RawBsonRef::Document(doc) => doc,
                    _ => {
                        return Err(Error::deserialization(
                            "internal error: unexpected non-document",
                        ))
                    }
                };
                visitor.visit_enum(DocumentAccess::new(doc, self.options.clone())?)
            }
            t => Err(Error::custom(format!("expected enum, instead got {:?}", t))),
        }
    }

    fn deserialize_bytes<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.element.element_type() {
            ElementType::ObjectId => visitor.visit_borrowed_bytes(self.element.slice()),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_newtype_struct<V>(
        self,
        name: &'static str,
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match name {
            UUID_NEWTYPE_NAME => self.deserialize_hint(
                visitor,
                DeserializerHint::BinarySubtype(BinarySubtype::Uuid),
            ),
            RAW_BSON_NEWTYPE => self.deserialize_hint(visitor, DeserializerHint::RawBson),
            RAW_DOCUMENT_NEWTYPE => {
                if self.element.element_type() != ElementType::EmbeddedDocument {
                    return Err(serde::de::Error::custom(format!(
                        "expected raw document, instead got {:?}",
                        self.element.element_type()
                    )));
                }

                self.deserialize_hint(visitor, DeserializerHint::RawBson)
            }
            RAW_ARRAY_NEWTYPE => {
                if self.element.element_type() != ElementType::Array {
                    return Err(serde::de::Error::custom(format!(
                        "expected raw array, instead got {:?}",
                        self.element.element_type()
                    )));
                }

                self.deserialize_hint(visitor, DeserializerHint::RawBson)
            }
            HUMAN_READABLE_NEWTYPE => {
                let mut inner = self;
                inner.options.human_readable = true;
                visitor.visit_newtype_struct(inner)
            }
            _ => visitor.visit_newtype_struct(self),
        }
    }

    fn is_human_readable(&self) -> bool {
        self.options.human_readable
    }

    forward_to_deserialize_any! {
        bool char str byte_buf unit unit_struct string
        identifier seq tuple tuple_struct struct
        map ignored_any i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
    }
}

struct DocumentAccess<'de> {
    iter: RawIter<'de>,
    elem: Option<RawElement<'de>>,
    options: DeserializerOptions,
}

impl<'de> DocumentAccess<'de> {
    fn new(doc: &'de RawDocument, options: DeserializerOptions) -> Result<Self> {
        Ok(Self {
            iter: doc.iter_elements(),
            elem: None,
            options,
        })
    }

    fn advance(&mut self) -> Result<()> {
        self.elem = self
            .iter
            .next()
            .transpose()
            .map_err(Error::deserialization)?;
        Ok(())
    }

    fn deserializer(self) -> Result<Deserializer<'de>> {
        let elem = match self.elem {
            Some(e) => e,
            None => {
                return Err(Error::deserialization(
                    "internal error: no element for deserializer",
                ))
            }
        };
        Ok(Deserializer {
            element: elem.clone(),
            options: self.options.clone(),
        })
    }
}

impl<'de> serde::de::MapAccess<'de> for DocumentAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> std::result::Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        self.advance()?;
        match &self.elem {
            None => Ok(None),
            Some(elem) => seed
                .deserialize(BorrowedStrDeserializer::new(elem.key()))
                .map(Some),
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        match &self.elem {
            None => Err(Error::deserialization("too many values requested")),
            Some(elem) => seed.deserialize(Deserializer {
                element: elem.clone(),
                options: self.options.clone(),
            }),
        }
    }
}

impl<'de> serde::de::SeqAccess<'de> for DocumentAccess<'de> {
    type Error = Error;

    fn next_element_seed<T>(
        &mut self,
        seed: T,
    ) -> std::result::Result<Option<T::Value>, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        self.advance()?;
        match &self.elem {
            None => Ok(None),
            Some(elem) => seed
                .deserialize(Deserializer {
                    element: elem.clone(),
                    options: self.options.clone(),
                })
                .map(Some),
        }
    }
}

impl<'de> serde::de::EnumAccess<'de> for DocumentAccess<'de> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(
        mut self,
        seed: V,
    ) -> std::result::Result<(V::Value, Self::Variant), Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        self.advance()?;
        let elem = match &self.elem {
            Some(e) => e,
            None => return Err(Error::EndOfStream),
        };
        let de: BorrowedStrDeserializer<'_, Error> = BorrowedStrDeserializer::new(elem.key());
        let key = seed.deserialize(de)?;
        Ok((key, self))
    }
}

impl<'de> serde::de::VariantAccess<'de> for DocumentAccess<'de> {
    type Error = Error;

    fn unit_variant(self) -> std::result::Result<(), Self::Error> {
        Err(Error::custom(
            "expected a string enum, got a document instead",
        ))
    }

    fn newtype_variant_seed<T>(self, seed: T) -> std::result::Result<T::Value, Self::Error>
    where
        T: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.deserializer()?)
    }

    fn tuple_variant<V>(self, _len: usize, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserializer()?.deserialize_seq(visitor)
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserializer()?.deserialize_map(visitor)
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

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn is_human_readable(&self) -> bool {
        false
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

/// A [`MapAccess`] used to deserialize entire documents as chunks of bytes without deserializing
/// the individual key/value pairs.
struct RawDocumentAccess<'d> {
    deserializer: RawDocumentDeserializer<'d>,

    /// Whether the first key has been deserialized yet or not.
    deserialized_first: bool,

    /// Whether or not this document being deserialized is for an array or not.
    array: bool,
}

impl<'de> RawDocumentAccess<'de> {
    fn new(doc: &'de RawDocument) -> Self {
        Self {
            deserializer: RawDocumentDeserializer { raw_doc: doc },
            deserialized_first: false,
            array: false,
        }
    }

    fn for_array(doc: &'de RawDocument) -> Self {
        Self {
            deserializer: RawDocumentDeserializer { raw_doc: doc },
            deserialized_first: false,
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
        if !self.deserialized_first {
            self.deserialized_first = true;

            // the newtype name will indicate to the [`RawBson`] enum that the incoming
            // bytes are meant to be treated as a document or array instead of a binary value.
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

    fn is_human_readable(&self) -> bool {
        false
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
        // save an allocation when deserializing to raw bson
        match self.hint {
            DeserializerHint::RawBson => visitor.visit_bytes(&self.oid.bytes()),
            _ => visitor.visit_string(self.oid.to_hex()),
        }
    }

    fn is_human_readable(&self) -> bool {
        false
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

    fn is_human_readable(&self) -> bool {
        false
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

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.stage {
            TimestampDeserializationStage::TopLevel => {
                self.stage = TimestampDeserializationStage::Time;
                visitor.visit_map(TimestampAccess { deserializer: self })
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

    fn is_human_readable(&self) -> bool {
        false
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit newtype_struct
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

/// A [`MapAccess`] providing access to a BSON datetime being deserialized.
///
/// If hinted to be raw BSON, this deserializes the serde data model equivalent
/// of { "$date": <i64 ms from epoch> }.
///
/// Otherwise, this deserializes the serde data model equivalent of
/// { "$date": { "$numberLong": <ms from epoch as a string> } }.
struct DateTimeAccess<'d> {
    deserializer: &'d mut DateTimeDeserializer,
}

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

enum DateTimeDeserializationStage {
    TopLevel,
    NumberLong,
    Done,
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

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
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
                    visitor.visit_map(DateTimeAccess { deserializer: self })
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

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn is_human_readable(&self) -> bool {
        false
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

/// A [`MapAccess`] providing access to a BSON binary being deserialized.
///
/// If hinted to be raw BSON, this deserializes the serde data model equivalent
/// of { "$binary": { "subType": <u8>, "bytes": <borrowed bytes> } }.
///
/// Otherwise, this deserializes the serde data model equivalent of
/// { "$binary": { "subType": <hex string>, "base64": <base64 encoded data> } }.
struct BinaryAccess<'d, 'de> {
    deserializer: &'d mut BinaryDeserializer<'de>,
}

impl<'de, 'd> serde::de::MapAccess<'de> for BinaryAccess<'d, 'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        let field_name = match self.deserializer.stage {
            BinaryDeserializationStage::TopLevel => "$binary",
            BinaryDeserializationStage::Subtype => "subType",
            BinaryDeserializationStage::Bytes => match self.deserializer.hint {
                DeserializerHint::RawBson => "bytes",
                _ => "base64",
            },
            BinaryDeserializationStage::Done => return Ok(None),
        };

        seed.deserialize(FieldDeserializer { field_name }).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.deserializer)
    }
}

struct BinaryDeserializer<'a> {
    binary: RawBinaryRef<'a>,
    hint: DeserializerHint,
    stage: BinaryDeserializationStage,
}

impl<'a> BinaryDeserializer<'a> {
    fn new(binary: RawBinaryRef<'a>, hint: DeserializerHint) -> Self {
        Self {
            binary,
            hint,
            stage: BinaryDeserializationStage::TopLevel,
        }
    }
}

impl<'de, 'a> serde::de::Deserializer<'de> for &'a mut BinaryDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.stage {
            BinaryDeserializationStage::TopLevel => {
                self.stage = BinaryDeserializationStage::Subtype;
                visitor.visit_map(BinaryAccess { deserializer: self })
            }
            BinaryDeserializationStage::Subtype => {
                self.stage = BinaryDeserializationStage::Bytes;
                match self.hint {
                    DeserializerHint::RawBson => visitor.visit_u8(self.binary.subtype.into()),
                    _ => visitor.visit_string(hex::encode([u8::from(self.binary.subtype)])),
                }
            }
            BinaryDeserializationStage::Bytes => {
                self.stage = BinaryDeserializationStage::Done;
                match self.hint {
                    DeserializerHint::RawBson => visitor.visit_borrowed_bytes(self.binary.bytes),
                    _ => visitor.visit_string(base64::encode(self.binary.bytes)),
                }
            }
            BinaryDeserializationStage::Done => {
                Err(Error::custom("Binary fully deserialized already"))
            }
        }
    }

    fn is_human_readable(&self) -> bool {
        false
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

struct CodeWithScopeAccess<'de> {
    cws: BsonCow<RawJavaScriptCodeWithScopeRef<'de>, Utf8LossyJavaScriptCodeWithScope<'de>>,
    hint: DeserializerHint,
    options: DeserializerOptions,
    stage: CodeWithScopeDeserializationStage,
}

impl<'de> CodeWithScopeAccess<'de> {
    fn new(
        cws: BsonCow<RawJavaScriptCodeWithScopeRef<'de>, Utf8LossyJavaScriptCodeWithScope<'de>>,
        hint: DeserializerHint,
        options: DeserializerOptions,
    ) -> Self {
        Self {
            cws,
            hint,
            options,
            stage: CodeWithScopeDeserializationStage::Code,
        }
    }
}

impl<'de> serde::de::MapAccess<'de> for CodeWithScopeAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        let field_name = match self.stage {
            CodeWithScopeDeserializationStage::Code => "$code",
            CodeWithScopeDeserializationStage::Scope => "$scope",
            CodeWithScopeDeserializationStage::Done => return Ok(None),
        };
        seed.deserialize(FieldDeserializer { field_name }).map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let value = seed.deserialize(&*self)?;
        self.stage = match self.stage {
            CodeWithScopeDeserializationStage::Code => CodeWithScopeDeserializationStage::Scope,
            CodeWithScopeDeserializationStage::Scope => CodeWithScopeDeserializationStage::Done,
            CodeWithScopeDeserializationStage::Done => return Err(Error::EndOfStream),
        };
        Ok(value)
    }
}

impl<'a, 'de> serde::de::Deserializer<'de> for &'a CodeWithScopeAccess<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.stage {
            CodeWithScopeDeserializationStage::Code => match &self.cws {
                BsonCow::Borrowed(cws) => visitor.visit_borrowed_str(cws.code),
                BsonCow::Owned(cws) => visitor.visit_str(&cws.code),
            },
            CodeWithScopeDeserializationStage::Scope => {
                let scope = match &self.cws {
                    BsonCow::Borrowed(cws) => cws.scope,
                    BsonCow::Owned(cws) => cws.scope,
                };
                match self.hint {
                    DeserializerHint::RawBson => visitor.visit_map(RawDocumentAccess::new(scope)),
                    _ => visitor.visit_map(DocumentAccess::new(scope, self.options.clone())?),
                }
            }
            CodeWithScopeDeserializationStage::Done => Err(Error::EndOfStream),
        }
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn is_human_readable(&self) -> bool {
        self.options.human_readable
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

#[derive(Debug)]
enum CodeWithScopeDeserializationStage {
    Code,
    Scope,
    Done,
}

/// A [`MapAccess`] providing access to a BSON DB pointer being deserialized.
///
/// Regardless of the hint, this deserializes the serde data model equivalent
/// of { "$dbPointer": { "$ref": <borrowed str>, "$id": <bytes> } }.
struct DbPointerAccess<'de> {
    dbp: BsonCow<RawDbPointerRef<'de>, DbPointer>,
    hint: DeserializerHint,
    stage: DbPointerDeserializationStage,
}

impl<'de> DbPointerAccess<'de> {
    fn new(dbp: BsonCow<RawDbPointerRef<'de>, DbPointer>, hint: DeserializerHint) -> Self {
        Self {
            dbp,
            hint,
            stage: DbPointerDeserializationStage::TopLevel,
        }
    }
}

impl<'de> serde::de::MapAccess<'de> for DbPointerAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> std::result::Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        let name = match self.stage {
            DbPointerDeserializationStage::TopLevel => "$dbPointer",
            DbPointerDeserializationStage::Namespace => "$ref",
            DbPointerDeserializationStage::Id => "$id",
            DbPointerDeserializationStage::Done => return Ok(None),
        };
        seed.deserialize(FieldDeserializer { field_name: name })
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }
}

impl<'a, 'de> serde::de::Deserializer<'de> for &'a mut DbPointerAccess<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.stage {
            DbPointerDeserializationStage::TopLevel => visitor.visit_map(DbPointerAccess {
                dbp: self.dbp.clone(),
                hint: self.hint,
                stage: DbPointerDeserializationStage::Namespace,
            }),
            DbPointerDeserializationStage::Namespace => {
                self.stage = DbPointerDeserializationStage::Id;
                match &self.dbp {
                    BsonCow::Borrowed(dbp) => visitor.visit_borrowed_str(dbp.namespace),
                    BsonCow::Owned(dbp) => visitor.visit_str(&dbp.namespace),
                }
            }
            DbPointerDeserializationStage::Id => {
                self.stage = DbPointerDeserializationStage::Done;
                let oid = match &self.dbp {
                    BsonCow::Borrowed(dbp) => dbp.id,
                    BsonCow::Owned(dbp) => dbp.id,
                };
                visitor.visit_map(ObjectIdAccess::new(oid, self.hint))
            }
            DbPointerDeserializationStage::Done => {
                Err(Error::custom("DbPointer fully deserialized already"))
            }
        }
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn is_human_readable(&self) -> bool {
        false
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit
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

/// A [`MapAccess`] providing access to a BSON regular expression being deserialized.
///
/// Regardless of the hint, this deserializes the serde data model equivalent
/// of { "$regularExpression": { "pattern": <borrowed str>, "options": <borrowed str> } }.
struct RegexAccess<'de> {
    re: BsonCow<RawRegexRef<'de>, Regex>,
    stage: RegexDeserializationStage,
}

impl<'de> RegexAccess<'de> {
    fn new(re: BsonCow<RawRegexRef<'de>, Regex>) -> Self {
        Self {
            re,
            stage: RegexDeserializationStage::TopLevel,
        }
    }
}

impl<'de> serde::de::MapAccess<'de> for RegexAccess<'de> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        let name = match self.stage {
            RegexDeserializationStage::TopLevel => "$regularExpression",
            RegexDeserializationStage::Pattern => "pattern",
            RegexDeserializationStage::Options => "options",
            RegexDeserializationStage::Done => return Ok(None),
        };
        seed.deserialize(FieldDeserializer { field_name: name })
            .map(Some)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> std::result::Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }
}

impl<'a, 'de> serde::de::Deserializer<'de> for &'a mut RegexAccess<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.stage {
            RegexDeserializationStage::TopLevel => visitor.visit_map(RegexAccess {
                re: self.re.clone(),
                stage: RegexDeserializationStage::Pattern,
            }),
            RegexDeserializationStage::Pattern => {
                self.stage = RegexDeserializationStage::Options;
                match &self.re {
                    BsonCow::Borrowed(re) => visitor.visit_borrowed_str(re.pattern),
                    BsonCow::Owned(re) => visitor.visit_str(&re.pattern),
                }
            }
            RegexDeserializationStage::Options => {
                self.stage = RegexDeserializationStage::Done;
                match &self.re {
                    BsonCow::Borrowed(re) => visitor.visit_borrowed_str(re.options),
                    BsonCow::Owned(re) => visitor.visit_str(&re.options),
                }
            }
            RegexDeserializationStage::Done => {
                Err(Error::custom("Regex fully deserialized already"))
            }
        }
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn is_human_readable(&self) -> bool {
        false
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit
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

impl<'de> serde::de::Deserializer<'de> for RawBsonDeserializer<'de> {
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

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn is_human_readable(&self) -> bool {
        false
    }

    serde::forward_to_deserialize_any! {
        bool u8 u16 u32 u64 i8 i16 i32 i64 f32 f64 char str string seq
        bytes byte_buf map struct option unit
        ignored_any unit_struct tuple_struct tuple enum identifier
    }
}

#[derive(Debug, Copy, Clone)]
enum BsonCow<Borrowed, Owned> {
    Borrowed(Borrowed),
    Owned(Owned),
}
