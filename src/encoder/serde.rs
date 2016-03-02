use serde::ser::{Serialize, Serializer, SeqVisitor, MapVisitor};
use serde::ser::impls::MapIteratorVisitor;

use bson::{self, Bson, Document};
use oid::ObjectId;

use super::{to_bson, EncoderError, EncoderResult};

impl Serialize for ObjectId {
    #[inline]
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer,
    {
        let mut doc = Document::new();
        doc.insert("$oid".to_owned(), self.to_string());
        serializer.serialize_map(MapIteratorVisitor::new(doc.iter(), Some(doc.len())))
    }
}

impl Serialize for Document {
    #[inline]
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer,
    {
        serializer.serialize_map(MapIteratorVisitor::new(self.iter(), Some(self.len())))
    }
}

impl Serialize for Bson {
    #[inline]
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer,
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

#[derive(Debug, Clone, PartialEq)]
pub enum State {
    Bson(Bson),
    Array(Vec<Bson>),
    Document(bson::Document),
}

pub struct Encoder {
    state: Vec<State>,
}

impl Encoder {
    /// Construct a new `Serializer`.
    pub fn new() -> Encoder {
        Encoder {
            state: Vec::with_capacity(4),
        }
    }

    /// Unwrap the `Serializer` and return the `Value`.
    pub fn bson(mut self) -> EncoderResult<Bson> {
        match self.state.pop() {
            Some(State::Bson(value)) => Ok(value),
            Some(state) => Err(EncoderError::InvalidState(state)),
            None => Err(EncoderError::EmptyState),
        }
    }

    fn pop(&mut self) -> EncoderResult<State> {
        match self.state.pop() {
            Some(state) => Ok(state),
            None => Err(EncoderError::EmptyState),
        }
    }
}

impl Serializer for Encoder {
    type Error = EncoderError;

    #[inline]
    fn serialize_bool(&mut self, value: bool) -> EncoderResult<()> {
        self.state.push(State::Bson(Bson::Boolean(value)));
        Ok(())
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
        self.state.push(State::Bson(Bson::I32(value)));
        Ok(())
    }

    #[inline]
    fn serialize_i64(&mut self, value: i64) -> EncoderResult<()> {
        self.state.push(State::Bson(Bson::I64(value)));
        Ok(())
    }

    #[inline]
    fn serialize_u64(&mut self, value: u64) -> EncoderResult<()> {
        self.state.push(State::Bson(Bson::FloatingPoint(value as f64)));
        Ok(())
    }

    #[inline]
    fn serialize_f64(&mut self, value: f64) -> EncoderResult<()> {
        self.state.push(State::Bson(Bson::FloatingPoint(value as f64)));
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
        self.state.push(State::Bson(Bson::String(String::from(value))));
        Ok(())
    }

    #[inline]
    fn serialize_none(&mut self) -> EncoderResult<()> {
        self.serialize_unit()
    }

    #[inline]
    fn serialize_some<V>(&mut self, value: V) -> EncoderResult<()>
        where V: Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(&mut self) -> EncoderResult<()> {
        self.state.push(State::Bson(Bson::Null));
        Ok(())
    }

    #[inline]
    fn serialize_unit_variant(&mut self,
                          _name: &str,
                          _variant_index: usize,
                          variant: &str) -> EncoderResult<()> {

        let mut values = bson::Document::new();
        values.insert(String::from(variant), Bson::Array(vec![]));

        self.state.push(State::Bson(Bson::Document(values)));
        Ok(())
    }

    #[inline]
    fn serialize_newtype_variant<T>(&mut self,
                                _name: &str,
                                _variant_index: usize,
                                variant: &str,
                                value: T) -> EncoderResult<()>
        where T: Serialize,
    {
        let mut values = bson::Document::new();
        values.insert(String::from(variant), try!(to_bson(&value)));

        self.state.push(State::Bson(Bson::Document(values)));
        Ok(())
    }

    #[inline]
    fn serialize_seq<V>(&mut self, mut visitor: V) -> EncoderResult<()>
        where V: SeqVisitor,
    {
        let len = visitor.len().unwrap_or(0);
        let values = Vec::with_capacity(len);
        self.state.push(State::Array(values));

        while let Some(()) = try!(visitor.visit(self)) { }

        let values = match try!(self.pop()) {
            State::Array(values) => values,
            state => return Err(EncoderError::InvalidState(state)),
        };

        self.state.push(State::Bson(Bson::Array(values)));
        Ok(())
    }

    #[inline]
    fn serialize_tuple_variant<V>(&mut self,
                              _name: &str,
                              _variant_index: usize,
                              variant: &str,
                              visitor: V) -> EncoderResult<()>
        where V: SeqVisitor,
    {
        try!(self.serialize_seq(visitor));

        let value = match try!(self.pop()) {
            State::Bson(value) => value,
            state => return Err(EncoderError::InvalidState(state)),
        };

        let mut object = bson::Document::new();
        object.insert(String::from(variant), value);

        self.state.push(State::Bson(Bson::Document(object)));
        Ok(())
    }

    #[inline]
    fn serialize_seq_elt<T>(&mut self, value: T) -> EncoderResult<()>
        where T: Serialize,
    {
        try!(value.serialize(self));

        let value = match try!(self.pop()) {
            State::Bson(value) => value,
            state => return Err(EncoderError::InvalidState(state)),
        };

        match self.state.last_mut() {
            Some(&mut State::Array(ref mut values)) => values.push(value),
            Some(state) => return Err(EncoderError::InvalidState(state.clone())),
            None => return Err(EncoderError::EmptyState),
        }

        Ok(())
    }

    #[inline]
    fn serialize_map<V>(&mut self, mut visitor: V) -> EncoderResult<()>
        where V: MapVisitor,
    {
        let values = bson::Document::new();
        self.state.push(State::Document(values));

        while let Some(()) = try!(visitor.visit(self)) { }

        let values = match try!(self.pop()) {
            State::Document(values) => values,
            state => return return Err(EncoderError::InvalidState(state)),
        };

        let bson = Bson::from_extended_document(values);
        self.state.push(State::Bson(bson));
        Ok(())
    }

    #[inline]
    fn serialize_struct_variant<V>(&mut self,
                               _name: &str,
                               _variant_index: usize,
                               variant: &str,
                               visitor: V) -> EncoderResult<()>
        where V: MapVisitor,
    {
        try!(self.serialize_map(visitor));

        let value = match try!(self.pop()) {
            State::Bson(value) => value,
            state => return Err(EncoderError::InvalidState(state)),
        };

        let mut object = bson::Document::new();
        object.insert(String::from(variant), value);

        self.state.push(State::Bson(Bson::Document(object)));
        Ok(())
    }

    #[inline]
    fn serialize_map_elt<K, V>(&mut self, key: K, value: V) -> EncoderResult<()>
        where K: Serialize,
              V: Serialize,
    {
        try!(key.serialize(self));

        let key = match try!(self.pop()) {
            State::Bson(Bson::String(value)) => value,
            state => return Err(EncoderError::InvalidMapKeyType(state)),
        };

        try!(value.serialize(self));

        let value = match try!(self.pop()) {
            State::Bson(value) => value,
            state => return Err(EncoderError::InvalidState(state)),
        };

        if Bson::Null != value {
            match self.state.last_mut() {
                Some(&mut State::Document(ref mut values)) => { values.insert(key, value); }
                Some(state) => return Err(EncoderError::InvalidState(state.clone())),
                None => return Err(EncoderError::EmptyState),
            }
        }

        Ok(())
    }
}
