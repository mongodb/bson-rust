use serde::ser::{
    self,
    Serialize,
    SerializeMap,
    SerializeSeq,
    SerializeStruct,
    SerializeStructVariant,
    SerializeTuple,
    SerializeTupleStruct,
    SerializeTupleVariant,
};
use serde_bytes::Bytes;

use crate::{
    bson::{Array, Binary, Bson, DbPointer, Document, JavaScriptCodeWithScope, Regex, Timestamp},
    datetime::DateTime,
    extjson,
    oid::ObjectId,
    spec::BinarySubtype,
    Decimal128,
};

use super::{to_bson, Error};

impl Serialize for ObjectId {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::ser::Serializer,
    {
        let mut ser = serializer.serialize_struct("$oid", 1)?;
        ser.serialize_field("$oid", &self.to_string())?;
        ser.end()
    }
}

impl Serialize for Document {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            state.serialize_entry(k, v)?;
        }
        state.end()
    }
}

impl Serialize for Bson {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        match self {
            Bson::Double(v) => serializer.serialize_f64(*v),
            Bson::String(v) => serializer.serialize_str(v),
            Bson::Array(v) => v.serialize(serializer),
            Bson::Document(v) => v.serialize(serializer),
            Bson::Boolean(v) => serializer.serialize_bool(*v),
            Bson::Null => serializer.serialize_unit(),
            Bson::Int32(v) => serializer.serialize_i32(*v),
            Bson::Int64(v) => serializer.serialize_i64(*v),
            Bson::ObjectId(oid) => oid.serialize(serializer),
            Bson::DateTime(dt) => dt.serialize(serializer),
            Bson::Binary(b) => b.serialize(serializer),
            Bson::JavaScriptCode(c) => {
                let mut state = serializer.serialize_struct("$code", 1)?;
                state.serialize_field("$code", c)?;
                state.end()
            }
            Bson::JavaScriptCodeWithScope(code_w_scope) => code_w_scope.serialize(serializer),
            Bson::DbPointer(dbp) => dbp.serialize(serializer),
            Bson::Symbol(s) => {
                let mut state = serializer.serialize_struct("$symbol", 1)?;
                state.serialize_field("$symbol", s)?;
                state.end()
            }
            Bson::RegularExpression(re) => re.serialize(serializer),
            Bson::Timestamp(t) => t.serialize(serializer),
            Bson::Decimal128(d) => {
                let mut state = serializer.serialize_struct("$numberDecimal", 1)?;
                state.serialize_field("$numberDecimalBytes", Bytes::new(&d.bytes))?;
                state.end()
            }
            Bson::Undefined => {
                let mut state = serializer.serialize_struct("$undefined", 1)?;
                state.serialize_field("$undefined", &true)?;
                state.end()
            }
            Bson::MaxKey => {
                let mut state = serializer.serialize_struct("$maxKey", 1)?;
                state.serialize_field("$maxKey", &1)?;
                state.end()
            }
            Bson::MinKey => {
                let mut state = serializer.serialize_struct("$minKey", 1)?;
                state.serialize_field("$minKey", &1)?;
                state.end()
            }
        }
    }
}

/// Serde Serializer
#[non_exhaustive]
pub struct Serializer;

impl Serializer {
    /// Construct a new `Serializer`.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Serializer {
        Serializer
    }
}

impl ser::Serializer for Serializer {
    type Ok = Bson;
    type Error = Error;

    type SerializeSeq = ArraySerializer;
    type SerializeTuple = TupleSerializer;
    type SerializeTupleStruct = TupleStructSerializer;
    type SerializeTupleVariant = TupleVariantSerializer;
    type SerializeMap = MapSerializer;
    type SerializeStruct = StructSerializer;
    type SerializeStructVariant = StructVariantSerializer;

    #[inline]
    fn serialize_bool(self, value: bool) -> crate::ser::Result<Bson> {
        Ok(Bson::Boolean(value))
    }

    #[inline]
    fn serialize_i8(self, value: i8) -> crate::ser::Result<Bson> {
        self.serialize_i32(value as i32)
    }

    #[inline]
    fn serialize_u8(self, value: u8) -> crate::ser::Result<Bson> {
        Ok(Bson::Int32(value as i32))
    }

    #[inline]
    fn serialize_i16(self, value: i16) -> crate::ser::Result<Bson> {
        self.serialize_i32(value as i32)
    }

    #[inline]
    fn serialize_u16(self, value: u16) -> crate::ser::Result<Bson> {
        Ok(Bson::Int32(value as i32))
    }

    #[inline]
    fn serialize_i32(self, value: i32) -> crate::ser::Result<Bson> {
        Ok(Bson::Int32(value))
    }

    #[inline]
    fn serialize_u32(self, value: u32) -> crate::ser::Result<Bson> {
        Ok(Bson::Int64(value as i64))
    }

    #[inline]
    fn serialize_i64(self, value: i64) -> crate::ser::Result<Bson> {
        Ok(Bson::Int64(value))
    }

    #[inline]
    fn serialize_u64(self, value: u64) -> crate::ser::Result<Bson> {
        use std::convert::TryFrom;

        match i64::try_from(value) {
            Ok(ivalue) => Ok(Bson::Int64(ivalue)),
            Err(_) => Err(Error::UnsignedIntegerExceededRange(value)),
        }
    }

    #[inline]
    fn serialize_f32(self, value: f32) -> crate::ser::Result<Bson> {
        self.serialize_f64(value as f64)
    }

    #[inline]
    fn serialize_f64(self, value: f64) -> crate::ser::Result<Bson> {
        Ok(Bson::Double(value))
    }

    #[inline]
    fn serialize_char(self, value: char) -> crate::ser::Result<Bson> {
        let mut s = String::new();
        s.push(value);
        self.serialize_str(&s)
    }

    #[inline]
    fn serialize_str(self, value: &str) -> crate::ser::Result<Bson> {
        Ok(Bson::String(value.to_string()))
    }

    fn serialize_bytes(self, value: &[u8]) -> crate::ser::Result<Bson> {
        // let mut state = self.serialize_seq(Some(value.len()))?;
        // for byte in value {
        //     state.serialize_element(byte)?;
        // }
        // state.end()
        Ok(Bson::Binary(Binary {
            subtype: BinarySubtype::Generic,
            bytes: value.to_vec(),
        }))
    }

    #[inline]
    fn serialize_none(self) -> crate::ser::Result<Bson> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<V: ?Sized>(self, value: &V) -> crate::ser::Result<Bson>
    where
        V: Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> crate::ser::Result<Bson> {
        Ok(Bson::Null)
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> crate::ser::Result<Bson> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
    ) -> crate::ser::Result<Bson> {
        Ok(Bson::String(variant.to_string()))
    }

    #[inline]
    fn serialize_newtype_struct<T: ?Sized>(
        self,
        _name: &'static str,
        value: &T,
    ) -> crate::ser::Result<Bson>
    where
        T: Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> crate::ser::Result<Bson>
    where
        T: Serialize,
    {
        let mut newtype_variant = Document::new();
        newtype_variant.insert(variant, to_bson(value)?);
        Ok(newtype_variant.into())
    }

    #[inline]
    fn serialize_seq(self, len: Option<usize>) -> crate::ser::Result<Self::SerializeSeq> {
        Ok(ArraySerializer {
            inner: Array::with_capacity(len.unwrap_or(0)),
        })
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> crate::ser::Result<Self::SerializeTuple> {
        Ok(TupleSerializer {
            inner: Array::with_capacity(len),
        })
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> crate::ser::Result<Self::SerializeTupleStruct> {
        Ok(TupleStructSerializer {
            inner: Array::with_capacity(len),
        })
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> crate::ser::Result<Self::SerializeTupleVariant> {
        Ok(TupleVariantSerializer {
            inner: Array::with_capacity(len),
            name: variant,
        })
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> crate::ser::Result<Self::SerializeMap> {
        Ok(MapSerializer {
            inner: Document::new(),
            next_key: None,
        })
    }

    #[inline]
    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> crate::ser::Result<Self::SerializeStruct> {
        Ok(StructSerializer {
            inner: Document::new(),
        })
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> crate::ser::Result<Self::SerializeStructVariant> {
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
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> crate::ser::Result<()> {
        self.inner.push(to_bson(value)?);
        Ok(())
    }

    fn end(self) -> crate::ser::Result<Bson> {
        Ok(Bson::Array(self.inner))
    }
}

#[doc(hidden)]
pub struct TupleSerializer {
    inner: Array,
}

impl SerializeTuple for TupleSerializer {
    type Ok = Bson;
    type Error = Error;

    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> crate::ser::Result<()> {
        self.inner.push(to_bson(value)?);
        Ok(())
    }

    fn end(self) -> crate::ser::Result<Bson> {
        Ok(Bson::Array(self.inner))
    }
}

#[doc(hidden)]
pub struct TupleStructSerializer {
    inner: Array,
}

impl SerializeTupleStruct for TupleStructSerializer {
    type Ok = Bson;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> crate::ser::Result<()> {
        self.inner.push(to_bson(value)?);
        Ok(())
    }

    fn end(self) -> crate::ser::Result<Bson> {
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
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> crate::ser::Result<()> {
        self.inner.push(to_bson(value)?);
        Ok(())
    }

    fn end(self) -> crate::ser::Result<Bson> {
        let mut tuple_variant = Document::new();
        tuple_variant.insert(self.name, self.inner);
        Ok(tuple_variant.into())
    }
}

#[doc(hidden)]
pub struct MapSerializer {
    inner: Document,
    next_key: Option<String>,
}

impl SerializeMap for MapSerializer {
    type Ok = Bson;
    type Error = Error;

    fn serialize_key<T: ?Sized + Serialize>(&mut self, key: &T) -> crate::ser::Result<()> {
        self.next_key = match to_bson(&key)? {
            Bson::String(s) => Some(s),
            other => return Err(Error::InvalidDocumentKey(other)),
        };
        Ok(())
    }

    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> crate::ser::Result<()> {
        let key = self.next_key.take().unwrap_or_default();
        self.inner.insert(key, to_bson(&value)?);
        Ok(())
    }

    fn end(self) -> crate::ser::Result<Bson> {
        Ok(Bson::from_extended_document(self.inner))
    }
}

#[doc(hidden)]
pub struct StructSerializer {
    inner: Document,
}

impl SerializeStruct for StructSerializer {
    type Ok = Bson;
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> crate::ser::Result<()> {
        self.inner.insert(key, to_bson(value)?);
        Ok(())
    }

    fn end(self) -> crate::ser::Result<Bson> {
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
    type Error = Error;

    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        key: &'static str,
        value: &T,
    ) -> crate::ser::Result<()> {
        self.inner.insert(key, to_bson(value)?);
        Ok(())
    }

    fn end(self) -> crate::ser::Result<Bson> {
        let var = Bson::from_extended_document(self.inner);

        let mut struct_variant = Document::new();
        struct_variant.insert(self.name, var);

        Ok(Bson::Document(struct_variant))
    }
}

impl Serialize for Timestamp {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("$timestamp", 1)?;
        let body = extjson::models::TimestampBody {
            t: self.time,
            i: self.increment,
        };
        state.serialize_field("$timestamp", &body)?;
        state.end()
    }
}

impl Serialize for Regex {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("$regularExpression", 1)?;
        let body = extjson::models::RegexBody {
            pattern: self.pattern.clone(),
            options: self.options.clone(),
        };
        state.serialize_field("$regularExpression", &body)?;
        state.end()
    }
}

impl Serialize for JavaScriptCodeWithScope {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("$codeWithScope", 2)?;
        state.serialize_field("$code", &self.code)?;
        state.serialize_field("$scope", &self.scope)?;
        state.end()
    }
}

impl Serialize for Binary {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        if let BinarySubtype::Generic = self.subtype {
            serializer.serialize_bytes(self.bytes.as_slice())
        } else {
            let mut state = serializer.serialize_struct("$binary", 1)?;
            let body = extjson::models::BinaryBody {
                base64: base64::encode(self.bytes.as_slice()),
                subtype: hex::encode([self.subtype.into()]),
            };
            state.serialize_field("$binary", &body)?;
            state.end()
        }
    }
}

impl Serialize for Decimal128 {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("$numberDecimal", 1)?;
        state.serialize_field("$numberDecimalBytes", serde_bytes::Bytes::new(&self.bytes))?;
        state.end()
    }
}

impl Serialize for DateTime {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("$date", 1)?;
        let body = extjson::models::DateTimeBody::from_millis(self.timestamp_millis());
        state.serialize_field("$date", &body)?;
        state.end()
    }
}

impl Serialize for DbPointer {
    #[inline]
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        let mut state = serializer.serialize_struct("$dbPointer", 1)?;
        let body = extjson::models::DbPointerBody {
            ref_ns: self.namespace.clone(),
            id: self.id.into(),
        };
        state.serialize_field("$dbPointer", &body)?;
        state.end()
    }
}
