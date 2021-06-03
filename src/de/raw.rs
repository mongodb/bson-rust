use std::io::Read;

use serde::forward_to_deserialize_any;

use crate::{oid::ObjectId, spec::ElementType};

use super::{read_cstring, read_f64, read_i32, read_u8, Error};
use super::{read_i64, read_string, Result};

// hello

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

pub(crate) struct Deserializer<R> {
    reader: CountReader<R>,
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
            ElementType::Boolean => visitor.visit_bool(read_u8(&mut self.reader)? == 1),
            ElementType::Null => visitor.visit_none(),
            ElementType::ObjectId => {
                let oid = ObjectId::from_reader(&mut self.reader)?;
                visitor.visit_map(ObjectIdAccess::new(oid))
            }
            ElementType::EmbeddedDocument => {
                let length = read_i32(&mut self.reader)?;
                visitor.visit_map(MapAccess {
                    root_deserializer: &mut self,
                    length_remaining: length - 4,
                })
            }
            _ => todo!("unexecpted type {:?}", self.current_type),
        }
    }

    forward_to_deserialize_any! {
        bool char str bytes byte_buf option unit unit_struct
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

    // fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: serde::de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    // fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }

    // fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }

    // fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }

    // fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }

    // fn deserialize_unit_struct<V>(
    //     self,
    //     name: &'static str,
    //     visitor: V,
    // ) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }

    // fn deserialize_newtype_struct<V>(
    //     self,
    //     name: &'static str,
    //     visitor: V,
    // ) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }

    // fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }

    // fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }

    // fn deserialize_tuple_struct<V>(
    //     self,
    //     name: &'static str,
    //     len: usize,
    //     visitor: V,
    // ) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }

    // fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }

    // fn deserialize_struct<V>(
    //     self,
    //     name: &'static str,
    //     fields: &'static [&'static str],
    //     visitor: V,
    // ) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }

    // fn deserialize_enum<V>(
    //     self,
    //     name: &'static str,
    //     variants: &'static [&'static str],
    //     visitor: V,
    // ) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }

    // fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    // where
    //     V: serde::de::Visitor<'de> {
    //     todo!()
    // }
}

struct MapAccess<'d, T: 'd> {
    root_deserializer: &'d mut Deserializer<T>,
    length_remaining: i32,
}

impl<'de, 'd, R: Read> serde::de::MapAccess<'de> for MapAccess<'d, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        let tag = read_u8(&mut self.root_deserializer.reader)?;
        self.length_remaining -= 1;
        if tag == 0 {
            if self.length_remaining != 0 {
                panic!(
                    "got null byte but still have length {} remaining",
                    self.length_remaining
                )
            }
            return Ok(None);
        }
        // TODO: handle bad tags
        self.root_deserializer.current_type = ElementType::from(tag).unwrap();
        let start_bytes = self.root_deserializer.reader.bytes_read();
        let out = seed
            .deserialize(DocumentKeyDeserializer {
                root_deserializer: &mut *self.root_deserializer,
            })
            .map(Some);
        let bytes_read = self.root_deserializer.reader.bytes_read() - start_bytes;
        self.length_remaining -= bytes_read as i32;

        if self.length_remaining <= 0 {
            panic!("ran out of bytes!");
        }
        out
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let start_bytes = self.root_deserializer.reader.bytes_read();
        let out = seed.deserialize(&mut *self.root_deserializer);
        let bytes_read = self.root_deserializer.reader.bytes_read() - start_bytes;
        self.length_remaining -= bytes_read as i32;

        if self.length_remaining <= 0 {
            panic!("ran out of bytes!");
        }
        out
    }
}

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

#[cfg(test)]
mod test {
    use std::time::Instant;

    use serde::Deserialize;

    use crate::{oid::ObjectId, Document};

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

    #[test]
    fn raw() {
        let doc = doc! {
            "ok": 1,
            "x": 1,
            "y": 2,
            "i": { "a": 300, "b": 12345 },
            // "oid": ObjectId::new(),
            "null": crate::Bson::Null,
            "b": true,
            "d": 12.5,
        };
        let mut bson = vec![0u8; 0];
        doc.to_writer(&mut bson).unwrap();
        println!("byte len: {}", bson.len());

        let raw_start = Instant::now();
        for _ in 0..10_000 {
            let mut de = Deserializer::new(bson.as_slice());
            // let cr = CommandResponse::deserialize(&mut de).unwrap();
            let t = Document::deserialize(&mut de).unwrap();
        }
        let raw_time = raw_start.elapsed();
        println!("raw time: {}", raw_time.as_secs_f32());

        let normal_start = Instant::now();
        for _ in 0..10_000 {
            // let mut de = Deserializer::new(bson.as_slice());
            // // let cr = CommandResponse::deserialize(&mut de).unwrap();
            // let t = D::deserialize(&mut de).unwrap();
            let d = Document::from_reader(bson.as_slice()).unwrap();
            let t: Document = crate::from_document(d).unwrap();
        }
        let normal_time = normal_start.elapsed();
        println!("normal time: {}", normal_time.as_secs_f32());

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
