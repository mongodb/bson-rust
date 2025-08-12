use serde::{ser::Impossible, Serialize};

use crate::{
    error::{Error, Result},
    RawBsonRef,
};

use super::Serializer;

pub(crate) struct DocumentSerializationResult<'a, 'b> {
    pub(crate) root_serializer: &'a mut Serializer<'b>,
}

/// Serializer used to serialize document or array bodies.
pub(crate) struct DocumentSerializer<'a, 'b> {
    root_serializer: &'a mut Serializer<'b>,
    num_keys_serialized: usize,
    start: usize,
}

impl<'a, 'b> DocumentSerializer<'a, 'b> {
    pub(crate) fn start(rs: &'a mut Serializer<'b>) -> Self {
        let start = rs.bytes.len();
        RawBsonRef::Int32(0).append_to(rs.bytes);
        Self {
            root_serializer: rs,
            num_keys_serialized: 0,
            start,
        }
    }

    /// Serialize a document key using the provided closure.
    fn serialize_doc_key_custom<F: FnOnce(&mut Serializer<'b>) -> Result<()>>(
        &mut self,
        f: F,
    ) -> Result<()> {
        // push a dummy element type for now, will update this once we serialize the value
        self.root_serializer.reserve_element_type();
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

    pub(crate) fn end_doc(self) -> crate::ser::Result<DocumentSerializationResult<'a, 'b>> {
        self.root_serializer.bytes.push(0);
        let length = (self.root_serializer.bytes.len() - self.start) as i32;
        self.root_serializer.replace_i32(self.start, length);
        Ok(DocumentSerializationResult {
            root_serializer: self.root_serializer,
        })
    }
}

impl serde::ser::SerializeSeq for DocumentSerializer<'_, '_> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        let index = self.num_keys_serialized;
        self.serialize_doc_key_custom(|rs| {
            use std::io::Write;
            write!(&mut rs.bytes, "{}", index)?;
            rs.bytes.push(0);
            Ok(())
        })?;
        value.serialize(&mut *self.root_serializer)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

impl serde::ser::SerializeMap for DocumentSerializer<'_, '_> {
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

impl serde::ser::SerializeStruct for DocumentSerializer<'_, '_> {
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

impl serde::ser::SerializeTuple for DocumentSerializer<'_, '_> {
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

impl serde::ser::SerializeTupleStruct for DocumentSerializer<'_, '_> {
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
struct KeySerializer<'a, 'b> {
    root_serializer: &'a mut Serializer<'b>,
}

impl serde::Serializer for KeySerializer<'_, '_> {
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
    fn serialize_bool(self, _v: bool) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("bool"))
    }

    #[inline]
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("i8"))
    }

    #[inline]
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("i16"))
    }

    #[inline]
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("i32"))
    }

    #[inline]
    fn serialize_i64(self, _v: i64) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("i64"))
    }

    #[inline]
    fn serialize_u8(self, _v: u8) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("u8"))
    }

    #[inline]
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("u16"))
    }

    #[inline]
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("u32"))
    }

    #[inline]
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("u64"))
    }

    #[inline]
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("f32"))
    }

    #[inline]
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("f64"))
    }

    #[inline]
    fn serialize_char(self, _v: char) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("char"))
    }

    #[inline]
    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        crate::raw::CStr::from_str(v)?.append_to(self.root_serializer.bytes);
        Ok(())
    }

    #[inline]
    fn serialize_bytes(self, _v: &[u8]) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("bytes"))
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok> {
        Err(Error::invalid_key_type("none"))
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
        Err(Error::invalid_key_type("unit"))
    }

    #[inline]
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok> {
        Err(Error::invalid_key_type(name))
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
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok>
    where
        T: Serialize + ?Sized,
    {
        Err(Error::invalid_key_type(format!("{}::{}", name, variant)))
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(Error::invalid_key_type("sequence"))
    }

    #[inline]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(Error::invalid_key_type("tuple"))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(Error::invalid_key_type(name))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(Error::invalid_key_type(format!("{}::{}", name, variant)))
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        Err(Error::invalid_key_type("map"))
    }

    #[inline]
    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Err(Error::invalid_key_type(name))
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(Error::invalid_key_type(format!("{}::{}", name, variant)))
    }
}
