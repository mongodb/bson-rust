use std::io::{ErrorKind, Read};

use serde::{
    de::{EnumAccess, Error as SerdeError, IntoDeserializer, VariantAccess},
    forward_to_deserialize_any,
    Deserializer as SerdeDeserializer,
};

use crate::{
    oid::ObjectId,
    spec::{BinarySubtype, ElementType},
    Binary,
    Bson,
    DateTime,
    DbPointer,
    Decimal128,
    JavaScriptCodeWithScope,
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

impl<'de> Deserializer<'de> {
    pub(crate) fn new(buf: &'de [u8]) -> Self {
        Self {
            bytes: BsonBuf::new(buf),
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
            _ => (),
        }
        Ok(())
    }

    /// Read a string from the BSON.
    fn parse_string(&mut self) -> Result<String> {
        read_string(&mut self.bytes, false)
    }

    /// Construct a `DocumentAccess` and pass it into the provided closure, returning the
    /// result of the closure if no other errors are encountered.
    fn deserialize_document<F, O>(&mut self, f: F) -> Result<O>
    where
        F: FnOnce(DocumentAccess<'_, 'de>) -> Result<O>,
    {
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
}

impl<'de, 'a> serde::de::Deserializer<'de> for &'a mut Deserializer<'de> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.current_type {
            ElementType::Int32 => visitor.visit_i32(read_i32(&mut self.bytes)?),
            ElementType::Int64 => visitor.visit_i64(read_i64(&mut self.bytes)?),
            ElementType::Double => visitor.visit_f64(read_f64(&mut self.bytes)?),
            ElementType::String => visitor.visit_string(self.parse_string()?),
            ElementType::Boolean => visitor.visit_bool(read_bool(&mut self.bytes)?),
            ElementType::Null => visitor.visit_unit(),
            ElementType::ObjectId => {
                let oid = ObjectId::from_reader(&mut self.bytes)?;
                visitor.visit_map(ObjectIdAccess::new(oid))
            }
            ElementType::EmbeddedDocument => {
                self.deserialize_document(|access| visitor.visit_map(access))
            }
            ElementType::Array => self.deserialize_document(|access| visitor.visit_seq(access)),
            ElementType::Binary => {
                let len = read_i32(&mut self.bytes)?;
                if !(0..=MAX_BSON_SIZE).contains(&len) {
                    return Err(Error::invalid_length(
                        len as usize,
                        &format!("binary length must be between 0 and {}", MAX_BSON_SIZE).as_str(),
                    ));
                }
                let subtype = BinarySubtype::from(read_u8(&mut self.bytes)?);
                match subtype {
                    BinarySubtype::Generic => {
                        visitor.visit_borrowed_bytes(self.bytes.read_slice(len as usize)?)
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
                let doc = Bson::Undefined.into_extended_document();
                visitor.visit_map(MapDeserializer::new(doc))
            }
            ElementType::DateTime => {
                let dti = read_i64(&mut self.bytes)?;
                let dt = DateTime::from_millis(dti);
                let mut d = DateTimeDeserializer::new(dt);
                visitor.visit_map(DateTimeAccess {
                    deserializer: &mut d,
                })
            }
            ElementType::RegularExpression => {
                let doc = Bson::RegularExpression(Regex::from_reader(&mut self.bytes)?)
                    .into_extended_document();
                visitor.visit_map(MapDeserializer::new(doc))
            }
            ElementType::DbPointer => {
                let doc = Bson::DbPointer(DbPointer::from_reader(&mut self.bytes)?)
                    .into_extended_document();
                visitor.visit_map(MapDeserializer::new(doc))
            }
            ElementType::JavaScriptCode => {
                let code = read_string(&mut self.bytes, false)?;
                let doc = Bson::JavaScriptCode(code).into_extended_document();
                visitor.visit_map(MapDeserializer::new(doc))
            }
            ElementType::JavaScriptCodeWithScope => {
                let code_w_scope = JavaScriptCodeWithScope::from_reader(&mut self.bytes)?;
                let doc = Bson::JavaScriptCodeWithScope(code_w_scope).into_extended_document();
                visitor.visit_map(MapDeserializer::new(doc))
            }
            ElementType::Symbol => {
                let symbol = read_string(&mut self.bytes, false)?;
                let doc = Bson::Symbol(symbol).into_extended_document();
                visitor.visit_map(MapDeserializer::new(doc))
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
                let doc = Bson::MaxKey.into_extended_document();
                visitor.visit_map(MapDeserializer::new(doc))
            }
            ElementType::MinKey => {
                let doc = Bson::MinKey.into_extended_document();
                visitor.visit_map(MapDeserializer::new(doc))
            }
        }
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
            ElementType::String => visitor.visit_enum(self.parse_string()?.into_deserializer()),
            ElementType::EmbeddedDocument => {
                self.deserialize_document(|access| visitor.visit_enum(access))
            }
            t => Err(Error::custom(format!("expected enum, instead got {:?}", t))),
        }
    }

    forward_to_deserialize_any! {
        bool char str bytes byte_buf unit unit_struct string
            newtype_struct seq tuple tuple_struct struct map
            ignored_any i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let s = self.bytes.read_cstr()?;
        visitor.visit_borrowed_str(s)
    }

    fn is_human_readable(&self) -> bool {
        false
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
    fn read_next_tag(&mut self) -> Result<Option<ElementType>> {
        let tag = read_u8(&mut self.root_deserializer.bytes)?;
        *self.length_remaining -= 1;
        if tag == 0 {
            if *self.length_remaining != 0 {
                return Err(Error::custom(format!(
                    "got null byte but still have length {} remaining",
                    self.length_remaining
                )));
            }
            return Ok(None);
        }

        let tag = ElementType::from(tag)
            .ok_or_else(|| Error::custom(format!("invalid element type: {}", tag)))?;

        self.root_deserializer.current_type = tag;
        Ok(Some(tag))
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

        if *self.length_remaining <= 0 {
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
        if self.read_next_tag()?.is_none() {
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
        if self.read_next_tag()?.is_none() {
            return Ok(None);
        }
        let _index = self.read(|s| s.root_deserializer.bytes.read_cstr())?;
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
        if self.read_next_tag()?.is_none() {
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
        let s = &mut self.root_deserializer.bytes.read_cstr()?;
        visitor.visit_borrowed_str(s)
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

struct ObjectIdAccess {
    oid: ObjectId,
    visited: bool,
}

impl ObjectIdAccess {
    fn new(oid: ObjectId) -> Self {
        Self {
            oid,
            visited: false,
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
        seed.deserialize(ObjectIdDeserializer(self.oid))
    }
}

struct ObjectIdDeserializer(ObjectId);

impl<'de> serde::de::Deserializer<'de> for ObjectIdDeserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_string(self.0.to_hex())
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
        seed.deserialize(Decimal128Deserializer(self.decimal.clone()))
    }
}

struct Decimal128Deserializer(Decimal128);

impl<'de> serde::de::Deserializer<'de> for Decimal128Deserializer {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        #[cfg(not(feature = "decimal128"))]
        {
            visitor.visit_bytes(&self.0.bytes)
        }

        #[cfg(feature = "decimal128")]
        {
            visitor.visit_bytes(&self.0.to_raw_bytes_le())
        }
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
}

impl DateTimeDeserializer {
    fn new(dt: DateTime) -> Self {
        Self {
            dt,
            stage: DateTimeDeserializationStage::TopLevel,
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
            DateTimeDeserializationStage::TopLevel => {
                self.stage = DateTimeDeserializationStage::NumberLong;
                visitor.visit_map(DateTimeAccess {
                    deserializer: &mut self,
                })
            }
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

struct BinaryAccess<'d> {
    deserializer: &'d mut BinaryDeserializer,
}

impl<'de, 'd> serde::de::MapAccess<'de> for BinaryAccess<'d> {
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

struct BinaryDeserializer {
    binary: Binary,
    stage: BinaryDeserializationStage,
}

impl BinaryDeserializer {
    fn new(binary: Binary) -> Self {
        Self {
            binary,
            stage: BinaryDeserializationStage::TopLevel,
        }
    }
}

impl<'de, 'a> serde::de::Deserializer<'de> for &'a mut BinaryDeserializer {
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
                visitor.visit_string(hex::encode([u8::from(self.binary.subtype)]))
            }
            BinaryDeserializationStage::Bytes => {
                self.stage = BinaryDeserializationStage::Done;
                visitor.visit_string(base64::encode(self.binary.bytes.as_slice()))
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

/// Struct wrapping a slice of BSON bytes.
struct BsonBuf<'a> {
    bytes: &'a [u8],
    index: usize,
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
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, index: 0 }
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

    fn read_cstr(&mut self) -> Result<&'a str> {
        let start = self.index;
        while self.index < self.bytes.len() && self.bytes[self.index] != 0 {
            self.index += 1
        }

        self.index_check()?;

        let s = std::str::from_utf8(&self.bytes[start..self.index]).map_err(Error::custom);
        // consume the null byte
        self.index += 1;
        s
    }

    fn read_slice(&mut self, length: usize) -> Result<&'a [u8]> {
        let start = self.index;
        self.index += length;
        self.index_check()?;
        Ok(&self.bytes[start..self.index])
    }
}
