use std::io::Read;

use serde::{de::Error as SerdeError, forward_to_deserialize_any};

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
    read_cstring,
    read_f128,
    read_f64,
    read_i32,
    read_i64,
    read_string,
    read_u8,
    Error,
    Result,
};
use crate::de::serde::MapDeserializer;

/// Deserializer used to parse and deserialize raw BSON bytes.
pub(crate) struct Deserializer<R> {
    reader: CountReader<R>,

    /// The type of the element currently being deserialized.
    ///
    /// When the Deserializer is initialized, this will be `ElementType::EmbeddedDocument`, as the
    /// only top level type is a document. The "embedded" portion is incorrect in this context,
    /// but given that there's no difference between deserializing an embedded document and a
    /// top level one, the distinction isn't necessary.
    current_type: ElementType,
}

impl<R> Deserializer<R>
where
    R: Read,
{
    pub(crate) fn new(reader: R) -> Self {
        Self {
            reader: CountReader::new(reader),
            current_type: ElementType::EmbeddedDocument,
        }
    }
}

impl<'de, 'a, R: Read> serde::de::Deserializer<'de> for &'a mut Deserializer<R> {
    type Error = Error;

    fn deserialize_any<V>(mut self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.current_type {
            ElementType::Int32 => visitor.visit_i32(read_i32(&mut self.reader)?),
            ElementType::Int64 => visitor.visit_i64(read_i64(&mut self.reader)?),
            ElementType::Double => visitor.visit_f64(read_f64(&mut self.reader)?),
            ElementType::String => visitor.visit_string(read_string(&mut self.reader, true)?),
            ElementType::Boolean => visitor.visit_bool(read_bool(&mut self.reader)?),
            ElementType::Null => visitor.visit_none(),
            ElementType::ObjectId => {
                let oid = ObjectId::from_reader(&mut self.reader)?;
                visitor.visit_map(ObjectIdAccess::new(oid))
            }
            ElementType::EmbeddedDocument => {
                let length = read_i32(&mut self.reader)?;
                visitor.visit_map(DocumentAccess {
                    root_deserializer: &mut self,
                    length_remaining: length - 4,
                })
            }
            ElementType::Array => {
                let length = read_i32(&mut self.reader)?;
                visitor.visit_seq(DocumentAccess {
                    root_deserializer: &mut self,
                    length_remaining: length - 4,
                })
            }
            ElementType::Binary => {
                let binary = Binary::from_reader(&mut self.reader)?;
                match binary.subtype {
                    BinarySubtype::Generic => visitor.visit_byte_buf(binary.bytes),
                    _ => {
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
                let dti = read_i64(&mut self.reader)?;
                let dt = DateTime::from_millis(dti);
                let mut d = DateTimeDeserializer::new(dt);
                visitor.visit_map(DateTimeAccess {
                    deserializer: &mut d,
                })
            }
            ElementType::RegularExpression => {
                let doc = Bson::RegularExpression(Regex::from_reader(&mut self.reader)?)
                    .into_extended_document();
                visitor.visit_map(MapDeserializer::new(doc))
            }
            ElementType::DbPointer => {
                let doc = Bson::DbPointer(DbPointer::from_reader(&mut self.reader)?)
                    .into_extended_document();
                visitor.visit_map(MapDeserializer::new(doc))
            }
            ElementType::JavaScriptCode => {
                let code = read_string(&mut self.reader, false)?;
                let doc = Bson::JavaScriptCode(code).into_extended_document();
                visitor.visit_map(MapDeserializer::new(doc))
            }
            ElementType::JavaScriptCodeWithScope => {
                let code_w_scope = JavaScriptCodeWithScope::from_reader(&mut self.reader)?;
                let doc = Bson::JavaScriptCodeWithScope(code_w_scope).into_extended_document();
                visitor.visit_map(MapDeserializer::new(doc))
            }
            ElementType::Symbol => {
                let symbol = read_string(&mut self.reader, false)?;
                let doc = Bson::Symbol(symbol).into_extended_document();
                visitor.visit_map(MapDeserializer::new(doc))
            }
            ElementType::Timestamp => {
                let ts = Timestamp::from_reader(&mut self.reader)?;
                let mut d = TimestampDeserializer::new(ts);
                visitor.visit_map(TimestampAccess {
                    deserializer: &mut d,
                })
            }
            ElementType::Decimal128 => {
                let d128 = read_f128(&mut self.reader)?;
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

    forward_to_deserialize_any! {
        bool char str bytes byte_buf option unit unit_struct string
            newtype_struct seq tuple tuple_struct struct map enum
            ignored_any i8 i16 i32 i64 u8 u16 u32 u64 f32 f64
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let s = read_cstring(&mut self.reader)?;
        visitor.visit_string(s)
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

/// A wrapper around a reader that records how many bytes have been read so far.
struct CountReader<R> {
    reader: R,
    bytes_read: usize,
}

impl<R: Read> CountReader<R> {
    /// Constructs a new CountReader that wraps `reader`.
    pub(super) fn new(reader: R) -> Self {
        CountReader {
            reader,
            bytes_read: 0,
        }
    }

    /// Gets the number of bytes read so far.
    pub(super) fn bytes_read(&self) -> usize {
        self.bytes_read
    }
}

impl<R: Read> Read for CountReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        let bytes = self.reader.read(buf)?;
        self.bytes_read += bytes;
        Ok(bytes)
    }
}

struct DocumentAccess<'d, T: 'd> {
    root_deserializer: &'d mut Deserializer<T>,
    length_remaining: i32,
}

impl<'d, T: Read> DocumentAccess<'d, T> {
    /// Read the next element type and update the root deserializer with it.
    ///
    /// Returns `Ok(None)` if the document has been fully read and has no more elements.
    fn read_next_tag(&mut self) -> Result<Option<ElementType>> {
        let tag = read_u8(&mut self.root_deserializer.reader)?;
        self.length_remaining -= 1;
        if tag == 0 {
            if self.length_remaining != 0 {
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

    /// Executes a closure that reads from the reader and ensures it did not spill over the
    /// length_remaining, returning an error if so.
    ///
    /// A mutable reference to this `DocumentAccess` is passed into the closure.
    fn read<F, O>(&mut self, f: F) -> Result<O>
    where
        F: FnOnce(&mut Self) -> Result<O>,
    {
        let start_bytes = self.root_deserializer.reader.bytes_read();
        let out = f(self);
        let bytes_read = self.root_deserializer.reader.bytes_read() - start_bytes;
        self.length_remaining -= bytes_read as i32;

        if self.length_remaining <= 0 {
            return Err(Error::custom("length of document too short"));
        }
        out
    }

    /// Read the next value from the document.
    fn read_next_value<'de, V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        self.read(|s| seed.deserialize(&mut *s.root_deserializer))
    }
}

impl<'de, 'd, R: Read> serde::de::MapAccess<'de> for DocumentAccess<'d, R> {
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

impl<'d, 'de, T: Read + 'd> serde::de::SeqAccess<'de> for DocumentAccess<'d, T> {
    type Error = Error;

    fn next_element_seed<S>(&mut self, seed: S) -> Result<Option<S::Value>>
    where
        S: serde::de::DeserializeSeed<'de>,
    {
        if self.read_next_tag()?.is_none() {
            return Ok(None);
        }

        let _index = self.read(|s| read_cstring(&mut s.root_deserializer.reader))?;
        self.read_next_value(seed).map(Some)
    }
}

/// Deserializer used specifically for deserializing a document's cstring keys.
struct DocumentKeyDeserializer<'d, R> {
    root_deserializer: &'d mut Deserializer<R>,
}

impl<'de, 'a, R: Read> serde::de::Deserializer<'de> for DocumentKeyDeserializer<'a, R> {
    type Error = Error;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        let s = read_cstring(&mut self.root_deserializer.reader)?;
        visitor.visit_string(s)
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

// TODO: update this to avoid having to go through hex
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
            TimestampDeserializationStage::Done => todo!(),
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
            DateTimeDeserializationStage::Done => todo!(),
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
                // visitor.visit_u8(self.binary.subtype.into())
            }
            BinaryDeserializationStage::Bytes => {
                // TODO: refactor this to save a copy
                self.stage = BinaryDeserializationStage::Done;
                // visitor.visit_byte_buf(self.binary.bytes)
                visitor.visit_string(base64::encode(self.binary.bytes.as_slice()))
                // visitor.visit_bytes(self.binary.bytes.as_slice())
                // println!("visiting binary");
                // visitor.visit_bytes(self.binary.bytes.as_slice())
            }
            BinaryDeserializationStage::Done => todo!(),
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

#[cfg(test)]
mod test {
    use std::time::Instant;

    use serde::Deserialize;

    use crate::{
        oid::ObjectId,
        tests::LOCK,
        Binary,
        Bson,
        DateTime,
        Decimal128,
        Document,
        Regex,
        Timestamp,
    };

    use super::Deserializer;

    #[derive(Debug, Deserialize)]
    struct D {
        x: i32,
        y: i32,
        i: I,
        // oid: ObjectId,
        null: Option<i32>,
        b: bool,
        d: f32,
    }

    #[derive(Debug, Deserialize)]
    struct I {
        a: i32,
        b: i32,
    }

    #[derive(Debug, Deserialize)]
    struct B {
        ok: i32,
        x: i32,
        y: i32,
        i: Ii,
        array: Vec<Bson>,
        oid: ObjectId,
        null: Option<()>,
        b: bool,
        d: f64,
        binary: Binary,
        date: DateTime,
        // regex: Regex,
        ts: Timestamp,
    }

    #[derive(Deserialize, Debug)]
    struct Ii {
        a: i32,
        b: i32,
    }

    #[test]
    fn raw() {
        let _guard = LOCK.run_concurrently();

        let doc = doc! {
            "ok": 1,
            "x": 1,
            "y": 2,
            "i": { "a": 300, "b": 12345 },
            "array": [ true, "oke", { "12": 24 } ],
            "oid": ObjectId::new(),
            "null": crate::Bson::Null,
            "b": true,
            "d": 12.5,
            "binary": crate::Binary { bytes: vec![36, 36, 36], subtype: crate::spec::BinarySubtype::Generic },
            "date": DateTime::now(),
            // "regex": Regex { pattern: "hello".to_string(), options: "x".to_string() },
            "ts": Timestamp { time: 123, increment: 456 },
            // "d128": Bson::Decimal128(Decimal128 { bytes: [0u8; 128 / 8] }),
        };
        let mut bson = vec![0u8; 0];
        doc.to_writer(&mut bson).unwrap();

        // let mut de = Deserializer::new(bson.as_slice());
        // // let cr = CommandResponse::deserialize(&mut de).unwrap();
        // let t = B::deserialize(&mut de).unwrap();
        // println!("doc: {:?}", t);

        // let d = Document::from_reader(bson.as_slice()).unwrap();
        // let t: Document = crate::from_document(d).unwrap();

        // let j: serde_json::Value = crate::from_document(doc.clone()).unwrap();
        // let j = serde_json::to_value(doc.clone()).unwrap();
        // println!("{:?}", j);
        let print = false;

        let binary_start = Instant::now();
        for i in 0..10_000 {
            let bytes =
                base64::encode("hello world this is a string wooohooo big string yeah".as_bytes());
            let _decoded = base64::decode(bytes).unwrap();
        }
        let binary_time = binary_start.elapsed();
        println!("binary time: {}", binary_time.as_secs_f32());

        let oid = ObjectId::new();
        let oid_start = Instant::now();
        for i in 0..10_000 {
            let hex = oid.to_hex();
            let _oid = ObjectId::parse_str(hex).unwrap();
            // let bytes = base64::encode(vec![36, 36, 36]);
            // let _decoded = base64::decode(bytes).unwrap();
        }
        let oid_time = binary_start.elapsed();
        println!("oid time: {}", oid_time.as_secs_f32());

        let raw_start = Instant::now();
        for i in 0..10_000 {
            let mut de = Deserializer::new(bson.as_slice());
            let t = B::deserialize(&mut de).unwrap();

            if i == 0 && print {
                println!("raw: {:#?}", t);
            }
        }
        let raw_time = raw_start.elapsed();
        println!("raw time: {}", raw_time.as_secs_f32());

        let raw_start = Instant::now();
        for i in 0..10_000 {
            let t: Document = crate::from_reader(bson.as_slice()).unwrap();

            if i == 0 {
                assert_eq!(t, doc);
                if print {
                    println!("raw: {:#?}", t);
                }
            }
        }
        let raw_time = raw_start.elapsed();
        println!("raw time doc: {}", raw_time.as_secs_f32());

        let normal_start = Instant::now();
        for i in 0..10_000 {
            let d = Document::from_reader(bson.as_slice()).unwrap();
            let t: B = crate::from_document(d).unwrap();
            if i == 0 && print {
                println!("normal: {:#?}", t);
            }
        }
        let normal_time = normal_start.elapsed();
        println!("normal time: {}", normal_time.as_secs_f32());

        let normal_start = Instant::now();
        for i in 0..10_000 {
            let d = Document::from_reader(bson.as_slice()).unwrap();
            let t: Document = crate::from_document(d).unwrap();
            if i == 0 {
                if print {
                    println!("normal: {:#?}", t);
                }
                assert_eq!(t, doc);
            }
        }
        let normal_time = normal_start.elapsed();
        println!("normal time doc: {}", normal_time.as_secs_f32());

        let normal_start = Instant::now();
        for _ in 0..10_000 {
            // let mut de = Deserializer::new(bson.as_slice());
            // // let cr = CommandResponse::deserialize(&mut de).unwrap();
            // let t = D::deserialize(&mut de).unwrap();
            // let d = Document::from_reader(bson.as_slice()).unwrap();
            // let t: Document = crate::from_document(doc.clone()).unwrap();
            let _d = Document::from_reader(bson.as_slice()).unwrap();
        }
        let normal_time = normal_start.elapsed();
        println!("decode time: {}", normal_time.as_secs_f32());
    }
}
