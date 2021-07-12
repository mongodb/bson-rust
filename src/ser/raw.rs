use std::{borrow::Borrow, io::Write, ops::Index};

use serde::{
    ser::{Error as SerdeError, Impossible, SerializeMap},
    Serialize,
    Serializer as SerdeSerializer,
};

use super::{write_cstring, write_f64, write_i32, write_i64, write_string, write_u8};
use crate::{
    ser::{Error, Result},
    spec::{BinarySubtype, ElementType},
};

pub(crate) struct Serializer {
    bytes: Vec<u8>,
    type_index: usize,
}

impl Serializer {
    pub(crate) fn new() -> Self {
        Self {
            bytes: Vec::new(),
            type_index: 0,
        }
    }

    pub(crate) fn into_vec(self) -> Vec<u8> {
        self.bytes
    }

    fn update_element_type(&mut self, t: ElementType) -> Result<()> {
        if self.type_index == 0 {
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
}

impl<'a> serde::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = DocumentSerializer<'a>;
    type SerializeTuple = DocumentSerializer<'a>;
    type SerializeTupleStruct = DocumentSerializer<'a>;
    type SerializeTupleVariant = VariantSerializer<'a>;
    type SerializeMap = DocumentSerializer<'a>;
    type SerializeStruct = DocumentSerializer<'a>;
    type SerializeStructVariant = VariantSerializer<'a>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Boolean)?;
        self.bytes.push(if v { 1 } else { 0 });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Int32)?;
        write_i32(&mut self.bytes, v.into())?;
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Int32)?;
        write_i32(&mut self.bytes, v.into())?;
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Int32)?;
        write_i32(&mut self.bytes, v)?;
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Int64)?;
        write_i64(&mut self.bytes, v)?;
        Ok(())
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Double)?;
        write_f64(&mut self.bytes, v.into())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Double)?;
        write_f64(&mut self.bytes, v.into())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.update_element_type(ElementType::String)?;
        write_string(&mut self.bytes, v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Binary)?;
        let len = v.len() as i32;
        write_i32(&mut self.bytes, len)?;
        write_u8(&mut self.bytes, BinarySubtype::Generic.into())?;
        self.bytes.write_all(v)?;
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Null)?;
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok> {
        self.serialize_none()
    }

    #[inline]
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok> {
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

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        self.update_element_type(ElementType::EmbeddedDocument)?;
        let mut d = DocumentSerializer::start(&mut *self)?;
        d.serialize_entry(variant, value)?;
        d.end_doc()
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.update_element_type(ElementType::Array)?;
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
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.update_element_type(ElementType::EmbeddedDocument)?;
        VariantSerializer::start(&mut *self, variant, VariantInnerType::Tuple)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        self.update_element_type(ElementType::EmbeddedDocument)?;
        DocumentSerializer::start(&mut *self)
    }

    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        self.update_element_type(ElementType::EmbeddedDocument)?;
        DocumentSerializer::start(&mut *self)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        VariantSerializer::start(&mut *self, variant, VariantInnerType::Struct)
    }
}

pub(crate) struct DocumentSerializer<'a> {
    root_serializer: &'a mut Serializer,
    num_keys_serialized: usize,
    start: usize,
}

impl<'a> DocumentSerializer<'a> {
    fn start(rs: &'a mut Serializer) -> crate::ser::Result<Self> {
        let start = rs.bytes.len();
        write_i32(&mut rs.bytes, 0)?;
        Ok(Self {
            root_serializer: rs,
            num_keys_serialized: 0,
            start,
        })
    }

    fn serialize_doc_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        // push a dummy element type for now, will update this once we serialize the value
        self.root_serializer.type_index = self.root_serializer.bytes.len();
        self.root_serializer.bytes.push(0);
        key.serialize(KeySerializer {
            root_serializer: &mut *self.root_serializer,
        })?;

        self.num_keys_serialized += 1;
        Ok(())
    }

    fn end_doc(self) -> crate::ser::Result<()> {
        self.root_serializer.bytes.push(0);
        let length = (self.root_serializer.bytes.len() - self.start) as i32;
        self.root_serializer.bytes.splice(
            self.start..self.start + 4,
            length.to_le_bytes().iter().cloned(),
        );
        Ok(())
    }
}

impl<'a> serde::ser::SerializeSeq for DocumentSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.serialize_doc_key(&self.num_keys_serialized.to_string())?;
        value.serialize(&mut *self.root_serializer)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_doc()
    }
}

impl<'a> serde::ser::SerializeMap for DocumentSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.serialize_doc_key(key)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut *self.root_serializer)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_doc()
    }
}

impl<'a> serde::ser::SerializeStruct for DocumentSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.serialize_doc_key(key)?;
        value.serialize(&mut *self.root_serializer)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_doc()
    }
}

impl<'a> serde::ser::SerializeTuple for DocumentSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.serialize_doc_key(&self.num_keys_serialized.to_string())?;
        value.serialize(&mut *self.root_serializer)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_doc()
    }
}

impl<'a> serde::ser::SerializeTupleStruct for DocumentSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.serialize_doc_key(&self.num_keys_serialized.to_string())?;
        value.serialize(&mut *self.root_serializer)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_doc()
    }
}

enum VariantInnerType {
    Tuple,
    Struct,
}

pub(crate) struct VariantSerializer<'a> {
    root_serializer: &'a mut Serializer,
    doc_start: usize,
    inner_start: usize,
    num_elements_serialized: usize,
}

impl<'a> VariantSerializer<'a> {
    fn start(
        rs: &'a mut Serializer,
        variant: &'static str,
        inner_type: VariantInnerType,
    ) -> Result<Self> {
        rs.update_element_type(ElementType::EmbeddedDocument)?;
        let doc_start = rs.bytes.len();
        write_i32(&mut rs.bytes, 0)?;

        let inner = match inner_type {
            VariantInnerType::Struct => ElementType::EmbeddedDocument,
            VariantInnerType::Tuple => ElementType::Array,
        };
        rs.bytes.push(inner as u8);
        write_cstring(&mut rs.bytes, variant)?;
        let inner_start = rs.bytes.len();

        Ok(Self {
            root_serializer: rs,
            num_elements_serialized: 0,
            doc_start,
            inner_start,
        })
    }

    fn serialize_element<T>(&mut self, k: &str, v: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.root_serializer.bytes.push(0);
        write_cstring(&mut self.root_serializer.bytes, k)?;
        v.serialize(&mut *self.root_serializer)?;

        self.num_elements_serialized += 1;
        Ok(())
    }

    fn end_both(self) -> Result<()> {
        // null byte for the inner
        self.root_serializer.bytes.push(0);
        let arr_length = (self.root_serializer.bytes.len() - self.inner_start) as i32;
        self.root_serializer.bytes.splice(
            self.inner_start..self.inner_start + 4,
            arr_length.to_le_bytes().iter().cloned(),
        );

        // null byte for document
        self.root_serializer.bytes.push(0);
        let doc_length = (self.root_serializer.bytes.len() - self.doc_start) as i32;
        self.root_serializer.bytes.splice(
            self.doc_start..self.doc_start + 4,
            doc_length.to_le_bytes().iter().cloned(),
        );
        Ok(())
    }
}

impl<'a> serde::ser::SerializeTupleVariant for VariantSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.serialize_element(format!("{}", self.num_elements_serialized).as_str(), value)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_both()
    }
}

impl<'a> serde::ser::SerializeStructVariant for VariantSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.serialize_element(key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_both()
    }
}

struct KeySerializer<'a> {
    root_serializer: &'a mut Serializer,
}

impl<'a> serde::Serializer for KeySerializer<'a> {
    type Ok = ();

    type Error = Error;

    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        write_cstring(&mut self.root_serializer.bytes, v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        todo!()
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        todo!()
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        todo!()
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        todo!()
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        todo!()
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        todo!()
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        todo!()
    }
}

#[cfg(test)]
mod test {
    use crate::doc;

    #[test]
    fn raw_serialize() {
        let doc = doc! {
            "x": { "y": "ok" },
            "a": true,
            "b": 1i32,
            "c": 2i64,
            "d": 5.5,
            "e": [ true, "aaa", { "ok": 1.0 } ]
        };
        println!("{}", doc);
        let mut v = Vec::new();
        doc.to_writer(&mut v).unwrap();

        let raw_v = crate::ser::to_vec(&doc).unwrap();
        assert_eq!(raw_v, v);
    }
    use std::time::Instant;

    use serde::Deserialize;

    use crate::{oid::ObjectId, Document};

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
    fn raw_bench() {
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

        let raw_start = Instant::now();
        for _ in 0..10_000 {
            let _b = crate::ser::to_vec(&doc).unwrap();
        }
        let raw_time = raw_start.elapsed();
        println!("raw time: {}", raw_time.as_secs_f32());

        let normal_start = Instant::now();
        for _ in 0..10_000 {
            let d: Document = crate::to_document(&doc).unwrap();
            let mut v = Vec::new();
            d.to_writer(&mut v).unwrap();
        }
        let normal_time = normal_start.elapsed();
        println!("normal time: {}", normal_time.as_secs_f32());

        let normal_start = Instant::now();
        for _ in 0..10_000 {
            let mut v = Vec::new();
            doc.to_writer(&mut v).unwrap();
        }
        let normal_time = normal_start.elapsed();
        println!("decode time: {}", normal_time.as_secs_f32());
    }
}
