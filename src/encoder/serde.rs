use serde::ser::{Serialize, Serializer};

use bson::{Array, Bson, Document};
use oid::ObjectId;

use super::{to_bson, EncoderError, EncoderResult};

impl Serialize for ObjectId {
    #[inline]
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer
    {
        let mut state = try!(serializer.serialize_map(Some(1)));
        try!(serializer.serialize_map_key(&mut state, "$oid"));
        try!(serializer.serialize_map_value(&mut state, self.to_string()));
        serializer.serialize_map_end(state)
    }
}

impl Serialize for Document {
    #[inline]
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer
    {
        let mut state = try!(serializer.serialize_map(Some(self.len())));
        for (k, v) in self {
            try!(serializer.serialize_map_key(&mut state, k));
            try!(serializer.serialize_map_value(&mut state, v));
        }
        serializer.serialize_map_end(state)
    }
}

impl Serialize for Bson {
    #[inline]
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
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

pub struct Encoder {
    value: Bson,
}

impl Encoder {
    /// Construct a new `Serializer`.
    pub fn new() -> Encoder {
        Encoder { value: Bson::Null }
    }

    /// Unwrap the `Encoder` and return the `Bson` value.
    pub fn bson(self) -> EncoderResult<Bson> {
        Ok(self.value)
    }
}

#[doc(hidden)]
pub struct TupleVariantState {
    name: &'static str,
    array: Array,
}

#[doc(hidden)]
pub struct StructVariantState {
    name: &'static str,
    map: MapState,
}

#[doc(hidden)]
pub struct MapState {
    document: Document,
    next_key: Option<String>,
}

impl Serializer for Encoder {
    type Error = EncoderError;
    type SeqState = Array;
    type TupleState = Array;
    type TupleStructState = Array;
    type TupleVariantState = TupleVariantState;
    type MapState = MapState;
    type StructState = MapState;
    type StructVariantState = StructVariantState;

    #[inline]
    fn serialize_bool(&mut self, value: bool) -> EncoderResult<()> {
        self.value = Bson::Boolean(value);
        Ok(())
    }

    #[inline]
    fn serialize_isize(&mut self, value: isize) -> EncoderResult<()> {
        self.serialize_i64(value as i64)
    }

    #[inline]
    fn serialize_i8(&mut self, value: i8) -> EncoderResult<()> {
        self.serialize_i32(value as i32)
    }

    #[inline]
    fn serialize_i16(&mut self, value: i16) -> EncoderResult<()> {
        self.serialize_i32(value as i32)
    }

    #[inline]
    fn serialize_i32(&mut self, value: i32) -> EncoderResult<()> {
        self.value = Bson::I32(value);
        Ok(())
    }

    #[inline]
    fn serialize_i64(&mut self, value: i64) -> EncoderResult<()> {
        self.value = Bson::I64(value);
        Ok(())
    }

    #[inline]
    fn serialize_usize(&mut self, value: usize) -> EncoderResult<()> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u8(&mut self, value: u8) -> EncoderResult<()> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u16(&mut self, value: u16) -> EncoderResult<()> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u32(&mut self, value: u32) -> EncoderResult<()> {
        self.serialize_u64(value as u64)
    }

    #[inline]
    fn serialize_u64(&mut self, value: u64) -> EncoderResult<()> {
        self.value = Bson::FloatingPoint(value as f64);
        Ok(())
    }

    #[inline]
    fn serialize_f32(&mut self, value: f32) -> EncoderResult<()> {
        self.serialize_f64(value as f64)
    }

    #[inline]
    fn serialize_f64(&mut self, value: f64) -> EncoderResult<()> {
        self.value = Bson::FloatingPoint(value);
        Ok(())
    }

    #[inline]
    fn serialize_char(&mut self, value: char) -> EncoderResult<()> {
        let mut s = String::new();
        s.push(value);
        self.serialize_str(&s)
    }

    #[inline]
    fn serialize_str(&mut self, value: &str) -> EncoderResult<()> {
        self.value = Bson::String(value.to_string());
        Ok(())
    }

    fn serialize_bytes(&mut self, value: &[u8]) -> EncoderResult<()> {
        let mut state = try!(self.serialize_seq(Some(value.len())));
        for byte in value {
            try!(self.serialize_seq_elt(&mut state, byte));
        }
        self.serialize_seq_end(state)
    }

    #[inline]
    fn serialize_none(&mut self) -> EncoderResult<()> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<V>(&mut self, value: V) -> EncoderResult<()>
        where V: Serialize
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(&mut self) -> EncoderResult<()> {
        Ok(())
    }

    #[inline]
    fn serialize_unit_struct(&mut self, _name: &'static str) -> EncoderResult<()> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_unit_variant(&mut self,
                              _name: &'static str,
                              _variant_index: usize,
                              variant: &'static str)
                              -> EncoderResult<()> {
        let mut unit_variant = Document::new();
        unit_variant.insert(variant.to_string(), Bson::Array(vec![]));

        self.value = Bson::Document(unit_variant);
        Ok(())
    }

    #[inline]
    fn serialize_newtype_struct<T>(&mut self, name: &'static str, value: T) -> EncoderResult<()>
        where T: Serialize
    {
        let mut state = try!(self.serialize_tuple_struct(name, 1));
        try!(self.serialize_tuple_struct_elt(&mut state, value));
        self.serialize_tuple_struct_end(state)
    }

    #[inline]
    fn serialize_newtype_variant<T>(&mut self,
                                    name: &'static str,
                                    variant_index: usize,
                                    variant: &'static str,
                                    value: T)
                                    -> EncoderResult<()>
        where T: Serialize
    {
        let mut state = try!(self.serialize_tuple_variant(name, variant_index, variant, 1));
        try!(self.serialize_tuple_variant_elt(&mut state, value));
        self.serialize_tuple_variant_end(state)
    }

    #[inline]
    fn serialize_seq(&mut self, len: Option<usize>) -> EncoderResult<Array> {
        Ok(Array::with_capacity(len.unwrap_or(0)))
    }

    #[inline]
    fn serialize_seq_elt<T>(&mut self, state: &mut Array, value: T) -> EncoderResult<()>
        where T: Serialize
    {
        state.push(try!(to_bson(&value)));
        Ok(())
    }

    #[inline]
    fn serialize_seq_end(&mut self, state: Array) -> EncoderResult<()> {
        self.value = Bson::Array(state);
        Ok(())
    }

    #[inline]
    fn serialize_seq_fixed_size(&mut self, len: usize) -> EncoderResult<Array> {
        Ok(Array::with_capacity(len))
    }

    #[inline]
    fn serialize_tuple(&mut self, len: usize) -> EncoderResult<Array> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_elt<T>(&mut self, state: &mut Array, value: T) -> EncoderResult<()>
        where T: Serialize
    {
        self.serialize_seq_elt(state, value)
    }

    #[inline]
    fn serialize_tuple_end(&mut self, state: Array) -> EncoderResult<()> {
        self.serialize_seq_end(state)
    }

    #[inline]
    fn serialize_tuple_struct(&mut self, _name: &'static str, len: usize) -> EncoderResult<Array> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct_elt<T>(&mut self, state: &mut Array, value: T) -> EncoderResult<()>
        where T: Serialize
    {
        self.serialize_seq_elt(state, value)
    }

    #[inline]
    fn serialize_tuple_struct_end(&mut self, state: Array) -> EncoderResult<()> {
        self.serialize_seq_end(state)
    }

    #[inline]
    fn serialize_tuple_variant(&mut self,
                               _name: &'static str,
                               _variant_index: usize,
                               variant: &'static str,
                               len: usize)
                               -> EncoderResult<TupleVariantState> {
        Ok(TupleVariantState {
            name: variant,
            array: try!(self.serialize_seq(Some(len))),
        })
    }

    #[inline]
    fn serialize_tuple_variant_elt<T>(&mut self,
                                      state: &mut TupleVariantState,
                                      value: T)
                                      -> EncoderResult<()>
        where T: Serialize
    {
        self.serialize_seq_elt(&mut state.array, value)
    }

    #[inline]
    fn serialize_tuple_variant_end(&mut self, state: TupleVariantState) -> EncoderResult<()> {
        let mut tuple_variant = Document::new();
        tuple_variant.insert(state.name.to_string(), Bson::Array(state.array));

        self.value = Bson::Document(tuple_variant);
        Ok(())
    }

    #[inline]
    fn serialize_map(&mut self, _len: Option<usize>) -> EncoderResult<MapState> {
        Ok(MapState {
            document: Document::new(),
            next_key: None,
        })
    }

    #[inline]
    fn serialize_map_key<T>(&mut self, state: &mut MapState, key: T) -> EncoderResult<()>
        where T: Serialize
    {
        state.next_key = match try!(to_bson(&key)) {
            Bson::String(s) => Some(s),
            other => return Err(EncoderError::InvalidMapKeyType(other)),
        };
        Ok(())
    }

    #[inline]
    fn serialize_map_value<T>(&mut self, state: &mut MapState, value: T) -> EncoderResult<()>
        where T: Serialize
    {
        let key = state.next_key.take().unwrap_or_else(|| "".to_string());
        state.document.insert(key, try!(to_bson(&value)));
        Ok(())
    }

    #[inline]
    fn serialize_map_end(&mut self, state: MapState) -> EncoderResult<()> {
        self.value = Bson::from_extended_document(state.document);
        Ok(())
    }

    #[inline]
    fn serialize_struct(&mut self, _name: &'static str, len: usize) -> EncoderResult<MapState> {
        self.serialize_map(Some(len))
    }

    #[inline]
    fn serialize_struct_elt<V: Serialize>(&mut self,
                                          state: &mut MapState,
                                          key: &'static str,
                                          value: V)
                                          -> EncoderResult<()> {
        try!(self.serialize_map_key(state, key));
        self.serialize_map_value(state, value)
    }

    #[inline]
    fn serialize_struct_end(&mut self, state: MapState) -> EncoderResult<()> {
        self.serialize_map_end(state)
    }

    #[inline]
    fn serialize_struct_variant(&mut self,
                                name: &'static str,
                                _variant_index: usize,
                                variant: &'static str,
                                len: usize)
                                -> EncoderResult<StructVariantState> {
        Ok(StructVariantState {
            name: variant,
            map: try!(self.serialize_struct(name, len)),
        })
    }

    #[inline]
    fn serialize_struct_variant_elt<V>(&mut self,
                                       state: &mut StructVariantState,
                                       key: &'static str,
                                       value: V)
                                       -> EncoderResult<()>
        where V: Serialize
    {
        self.serialize_struct_elt(&mut state.map, key, value)
    }

    #[inline]
    fn serialize_struct_variant_end(&mut self, state: StructVariantState) -> EncoderResult<()> {
        let value = Bson::from_extended_document(state.map.document);

        let mut struct_variant = Document::new();
        struct_variant.insert(state.name.to_string(), value);

        self.value = Bson::Document(struct_variant);
        Ok(())
    }
}
