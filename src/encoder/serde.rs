use serde::ser::{Serialize, Serializer, SerializeSeq, SerializeTuple, SerializeTupleStruct,
                 SerializeTupleVariant, SerializeMap, SerializeStruct, SerializeStructVariant};

use bson::{Array, Bson, Document, UtcDateTime};
use oid::ObjectId;

use super::{to_bson, EncoderError, EncoderResult};

impl Serialize for ObjectId {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut ser = serializer.serialize_map(Some(1))?;
        ser.serialize_key("$oid")?;
        ser.serialize_value(&self.to_string())?;
        ser.end()
    }
}

impl Serialize for Document {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut state = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            state.serialize_key(k)?;
            state.serialize_value(v)?;
        }
        state.end()
    }
}

impl Serialize for Bson {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        match *self {
            Bson::FloatingPoint(v) => serializer.serialize_f64(v),
            Bson::String(ref v) => serializer.serialize_str(v),
            Bson::Array(ref v) => v.serialize(serializer),
            Bson::Document(ref v) => v.serialize(serializer),
            Bson::Boolean(v) => serializer.serialize_bool(v),
            Bson::Null => serializer.serialize_unit(),
            Bson::I32(v) => serializer.serialize_i32(v),
            Bson::I64(v) => serializer.serialize_i64(v),
            _ => {
                let doc = self.to_extended_document();
                doc.serialize(serializer)
            }
        }
    }
}

/// Serde Encoder
pub struct Encoder;

impl Encoder {
    /// Construct a new `Serializer`.
    pub fn new() -> Encoder {
        Encoder
    }

    #[cfg(feature = "unsigned_conversion")]
    fn visit_unsigned_int<
        T: Copy + ::std::convert::TryInto<i32> + Into<u64>
    >(self, value: T) -> EncoderResult<Bson> {
        value.try_into()
            .map(|val| self.serialize_i32(val))
            .map_err(|_| EncoderError::OutOfRangeUnsignedType(value.into()))?
    }

    #[cfg(not(feature = "unsigned_conversion"))]
    fn visit_unsigned_int<T: Into<u64>>(self, _: T) -> EncoderResult<Bson> {
        Err(EncoderError::UnsupportedUnsignedType)
    }
}

impl Serializer for Encoder {
    type Ok = Bson;
    type Error = EncoderError;

    type SerializeSeq = ArraySerializer;
    type SerializeTuple = TupleSerializer;
    type SerializeTupleStruct = TupleStructSerializer;
    type SerializeTupleVariant = TupleVariantSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = StructSerializer;
    type SerializeStructVariant = StructVariantSerializer;

    #[inline]
    fn serialize_bool(self, value: bool) -> EncoderResult<Bson> {
        Ok(Bson::Boolean(value))
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> EncoderResult<Bson> {
        self.serialize_i32(value as i32)
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> EncoderResult<Bson> {
        self.visit_unsigned_int(value)
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> EncoderResult<Bson> {
        self.serialize_i32(value as i32)
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> EncoderResult<Bson> {
        self.visit_unsigned_int(value)
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> EncoderResult<Bson> {
        Ok(Bson::I32(value))
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> EncoderResult<Bson> {
        self.visit_unsigned_int(value)
    }

    #[inline]
    fn serialize_i64(self, value: i64) -> EncoderResult<Bson> {
        Ok(Bson::I64(value))
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> EncoderResult<Bson> {
        self.visit_unsigned_int(value)
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> EncoderResult<Bson> {
        self.serialize_f64(value as f64)
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> EncoderResult<Bson> {
        Ok(Bson::FloatingPoint(value))
    }

    #[inline]
    fn serialize_char(self, value: char) -> EncoderResult<Bson> {
        let mut s = String::new();
        s.push(value);
        self.serialize_str(&s)
    }

    #[inline]
    fn serialize_str(self, value: &str) -> EncoderResult<Bson> {
        Ok(Bson::String(value.to_string()))
    }

    fn serialize_bytes(self, value: &[u8]) -> EncoderResult<Bson> {
        let mut state = self.serialize_seq(Some(value.len()))?;
        for byte in value {
            state.serialize_element(byte)?;
        }
        state.end()
    }

    #[inline]
    fn serialize_none(self) -> EncoderResult<Bson> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<V: ?Sized>(self, value: &V) -> EncoderResult<Bson>
        where V: Serialize
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> EncoderResult<Bson> {
        Ok(Bson::Null)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> EncoderResult<Bson> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(self,
                              _name: &'static str,
                              _variant_index: u32,
                              variant: &'static str)
                              -> EncoderResult<Bson> {
        Ok(Bson::String(variant.to_string()))
    }

    #[inline]
    fn serialize_newtype_struct<T: ?Sized>(self,
                                           _name: &'static str,
                                           value: &T)
                                           -> EncoderResult<Bson>
        where T: Serialize
    {
        let mut ser = TupleStructSerializer { inner: Array::new() };
        ser.serialize_field(value)?;
        ser.end()
    }

    #[inline]
    fn serialize_newtype_variant<T: ?Sized>(self,
                                            _name: &'static str,
                                            _variant_index: u32,
                                            variant: &'static str,
                                            value: &T)
                                            -> EncoderResult<Bson>
        where T: Serialize
    {
        let mut ser = TupleVariantSerializer {
            inner: Array::new(),
            name: variant,
        };
        ser.serialize_field(value)?;
        ser.end()
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> EncoderResult<Self::SerializeSeq> {
        Ok(ArraySerializer { inner: Array::with_capacity(len.unwrap_or(0)) })
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> EncoderResult<Self::SerializeTuple> {
        Ok(TupleSerializer { inner: Array::with_capacity(len) })
    }

    #[inline]
    fn serialize_tuple_struct(self,
                              _name: &'static str,
                              len: usize)
                              -> EncoderResult<Self::SerializeTupleStruct> {
        Ok(TupleStructSerializer { inner: Array::with_capacity(len) })
    }

    #[inline]
    fn serialize_tuple_variant(self,
                               _name: &'static str,
                               _variant_index: u32,
                               variant: &'static str,
                               len: usize)
                               -> EncoderResult<Self::SerializeTupleVariant> {
        Ok(TupleVariantSerializer {
               inner: Array::with_capacity(len),
               name: variant,
           })
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> EncoderResult<Self::SerializeMap> {
        Ok(MapSerializer {
               inner: Document::new(),
               next_key: None,
           })
    }

    #[inline]
    fn serialize_struct(self,
                        _name: &'static str,
                        _len: usize)
                        -> EncoderResult<Self::SerializeStruct> {
        Ok(StructSerializer { inner: Document::new() })
    }

    #[inline]
    fn serialize_struct_variant(self,
                                _name: &'static str,
                                _variant_index: u32,
                                variant: &'static str,
                                _len: usize)
                                -> EncoderResult<Self::SerializeStructVariant> {
        Ok(StructVariantSerializer {
               name: variant,
               inner: Document::new(),
           })
    }
}

#[doc(hidden)]
pub struct ArraySerializer {
    inner: Array,
}

impl SerializeSeq for ArraySerializer {
    type Ok = Bson;
    type Error = EncoderError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> EncoderResult<()> {
        self.inner.push(to_bson(value)?);
        Ok(())
    }

    fn end(self) -> EncoderResult<Bson> {
        Ok(Bson::Array(self.inner))
    }
}

#[doc(hidden)]
pub struct TupleSerializer {
    inner: Array,
}

impl SerializeTuple for TupleSerializer {
    type Ok = Bson;
    type Error = EncoderError;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> EncoderResult<()> {
        self.inner.push(to_bson(value)?);
        Ok(())
    }

    fn end(self) -> EncoderResult<Bson> {
        Ok(Bson::Array(self.inner))
    }
}

#[doc(hidden)]
pub struct TupleStructSerializer {
    inner: Array,
}

impl SerializeTupleStruct for TupleStructSerializer {
    type Ok = Bson;
    type Error = EncoderError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> EncoderResult<()> {
        self.inner.push(to_bson(value)?);
        Ok(())
    }

    fn end(self) -> EncoderResult<Bson> {
        Ok(Bson::Array(self.inner))
    }
}

#[doc(hidden)]
pub struct TupleVariantSerializer {
    inner: Array,
    name: &'static str,
}

impl SerializeTupleVariant for TupleVariantSerializer {
    type Ok = Bson;
    type Error = EncoderError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> EncoderResult<()> {
        self.inner.push(to_bson(value)?);
        Ok(())
    }

    fn end(self) -> EncoderResult<Bson> {
        let mut tuple_variant = Document::new();
        if self.inner.len() == 1 {
            tuple_variant.insert(self.name, self.inner.into_iter().next().unwrap());
        } else {
            tuple_variant.insert(self.name, Bson::Array(self.inner));
        }

        Ok(Bson::Document(tuple_variant))
    }
}

#[doc(hidden)]
pub struct MapSerializer {
    inner: Document,
    next_key: Option<String>,
}

impl SerializeMap for MapSerializer {
    type Ok = Bson;
    type Error = EncoderError;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> EncoderResult<()> {
        self.next_key = match to_bson(&key)? {
            Bson::String(s) => Some(s),
            other => return Err(EncoderError::InvalidMapKeyType(other)),
        };
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> EncoderResult<()> {
        let key = self.next_key.take().unwrap_or_else(|| "".to_string());
        self.inner.insert(key, to_bson(&value)?);
        Ok(())
    }

    fn end(self) -> EncoderResult<Bson> {
        Ok(Bson::from_extended_document(self.inner))
    }
}

#[doc(hidden)]
pub struct StructSerializer {
    inner: Document,
}

impl SerializeStruct for StructSerializer {
    type Ok = Bson;
    type Error = EncoderError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self,
                                              key: &'static str,
                                              value: &T)
                                              -> EncoderResult<()> {
        self.inner.insert(key, to_bson(value)?);
        Ok(())
    }

    fn end(self) -> EncoderResult<Bson> {
        Ok(Bson::from_extended_document(self.inner))
    }
}

#[doc(hidden)]
pub struct StructVariantSerializer {
    inner: Document,
    name: &'static str,
}

impl SerializeStructVariant for StructVariantSerializer {
    type Ok = Bson;
    type Error = EncoderError;

    fn serialize_field<T: ?Sized + Serialize>(&mut self,
                                              key: &'static str,
                                              value: &T)
                                              -> EncoderResult<()> {
        self.inner.insert(key, to_bson(value)?);
        Ok(())
    }

    fn end(self) -> EncoderResult<Bson> {
        let var = Bson::from_extended_document(self.inner);

        let mut struct_variant = Document::new();
        struct_variant.insert(self.name, var);

        Ok(Bson::Document(struct_variant))
    }
}

impl Serialize for UtcDateTime {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        // Cloning a `DateTime` is extremely cheap
        let doc = Bson::UtcDatetime(self.0);
        doc.serialize(serializer)
    }
}
