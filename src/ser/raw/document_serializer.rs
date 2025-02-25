use serde::{ser::Impossible, Serialize};

use crate::{
    ser::{write_cstring, Error, Result},
    to_bson, Bson,
};

use super::Serializer;

/// Serializer used to serialize document or array bodies.
pub(crate) struct DocumentSerializer<'a> {
    root_serializer: &'a mut Serializer,
    num_keys_serialized: usize,
}

impl<'a> DocumentSerializer<'a> {
    pub(crate) fn start(rs: &'a mut Serializer) -> crate::ser::Result<Self> {
        rs.write_next_len()?;
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

    pub(crate) fn end_doc(self) -> crate::ser::Result<()> {
        self.root_serializer.bytes.push(0);
        Ok(())
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
        write_cstring(&mut self.root_serializer.bytes, v)
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
