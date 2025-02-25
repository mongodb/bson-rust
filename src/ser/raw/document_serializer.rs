use bytes::BufMut;
use serde::{ser::Impossible, Serialize};

use crate::{
    ser::{Error, Result},
    to_bson, Bson,
};

use super::{Key, Serializer};

/// Serializer used to serialize document or array bodies.
pub(crate) struct DocumentSerializer<'a, B> {
    root_serializer: &'a mut Serializer<B>,
    num_keys_serialized: usize,
}

impl<'a, B: BufMut> DocumentSerializer<'a, B> {
    pub(crate) fn start(rs: &'a mut Serializer<B>) -> crate::ser::Result<Self> {
        rs.write_next_len()?;
        Ok(Self {
            root_serializer: rs,
            num_keys_serialized: 0,
        })
    }

    /// Serialize a document key to string using [`KeySerializer`].
    fn serialize_doc_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        key.serialize(KeySerializer {
            root_serializer: &mut self.root_serializer,
        })?;
        self.num_keys_serialized += 1;
        Ok(())
    }

    pub(crate) fn end_doc(self) -> crate::ser::Result<()> {
        self.root_serializer.buf.put_u8(0);
        Ok(())
    }
}

impl<B: BufMut> serde::ser::SerializeSeq for DocumentSerializer<'_, B> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        self.root_serializer
            .set_next_key(Key::Index(self.num_keys_serialized));
        self.num_keys_serialized += 1;
        value.serialize(&mut *self.root_serializer)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

impl<B: BufMut> serde::ser::SerializeMap for DocumentSerializer<'_, B> {
    type Ok = ();

    type Error = Error;

    #[inline]
    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        self.serialize_doc_key(key) // XXX this may result in a new copy.
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

impl<B: BufMut> serde::ser::SerializeStruct for DocumentSerializer<'_, B> {
    type Ok = ();

    type Error = Error;

    #[inline]
    fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        self.root_serializer.set_next_key(Key::Static(key));
        value.serialize(&mut *self.root_serializer)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

impl<B: BufMut> serde::ser::SerializeTuple for DocumentSerializer<'_, B> {
    type Ok = ();

    type Error = Error;

    #[inline]
    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        self.root_serializer
            .set_next_key(Key::Index(self.num_keys_serialized));
        self.num_keys_serialized += 1;
        value.serialize(&mut *self.root_serializer)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

impl<B: BufMut> serde::ser::SerializeTupleStruct for DocumentSerializer<'_, B> {
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
struct KeySerializer<'a, B> {
    root_serializer: &'a mut Serializer<B>,
}

impl<B> KeySerializer<'_, B> {
    fn invalid_key<T: Serialize>(v: T) -> Error {
        Error::InvalidDocumentKey(to_bson(&v).unwrap_or(Bson::Null))
    }
}

impl<B: BufMut> serde::Serializer for KeySerializer<'_, B> {
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
        self.root_serializer.set_next_key(Key::Owned(v.to_owned()));
        Ok(())
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
