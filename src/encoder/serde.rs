use serde::ser::{Serialize, Serializer, SeqVisitor, MapVisitor};
use serde::ser::impls::MapIteratorVisitor;

use bson::{self, Bson, Document};
use oid::ObjectId;

use super::to_bson;

impl Serialize for ObjectId {
    #[inline]
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer,
    {
        let mut doc = Document::new();
        doc.insert("$oid".to_owned(), self.to_string());
        serializer.visit_map(MapIteratorVisitor::new(doc.iter(), Some(doc.len())))
    }
}

impl Serialize for Document {
    #[inline]
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer,
    {
        serializer.visit_map(MapIteratorVisitor::new(self.iter(), Some(self.len())))
    }
}

impl Serialize for Bson {
    #[inline]
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: Serializer,
    {
        match *self {
            Bson::FloatingPoint(v) => serializer.visit_f64(v),
            Bson::String(ref v) => serializer.visit_str(v),
            Bson::Array(ref v) => v.serialize(serializer),
            Bson::Document(ref v) => v.serialize(serializer),
            Bson::Boolean(v) => serializer.visit_bool(v),
            Bson::Null => serializer.visit_unit(),
            Bson::I32(v) => serializer.visit_i32(v),
            Bson::I64(v) => serializer.visit_i64(v),                        
            _ => {
                let doc = self.to_extended_document();
                doc.serialize(serializer)
            }
        }
    }
}

#[derive(Debug)]
enum State {
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
    pub fn unwrap(mut self) -> Bson {
        match self.state.pop() {
            Some(State::Bson(value)) => value,
            Some(state) => panic!("expected value, found {:?}", state),
            None => panic!("expected value, found no state"),
        }
    }
}

impl Serializer for Encoder {
    type Error = ();

    #[inline]
    fn visit_bool(&mut self, value: bool) -> Result<(), ()> {
        self.state.push(State::Bson(Bson::Boolean(value)));
        Ok(())
    }

    #[inline]
    fn visit_i8(&mut self, value: i8) -> Result<(), ()> {
        self.visit_i32(value as i32)
    }    

    #[inline]
    fn visit_i16(&mut self, value: i16) -> Result<(), ()> {
        self.visit_i32(value as i32)
    }    

    #[inline]
    fn visit_i32(&mut self, value: i32) -> Result<(), ()> {
        self.state.push(State::Bson(Bson::I32(value)));
        Ok(())
    }

    #[inline]
    fn visit_i64(&mut self, value: i64) -> Result<(), ()> {
        self.state.push(State::Bson(Bson::I64(value)));
        Ok(())
    }

    #[inline]
    fn visit_u64(&mut self, value: u64) -> Result<(), ()> {
        self.state.push(State::Bson(Bson::FloatingPoint(value as f64)));
        Ok(())
    }

    #[inline]
    fn visit_f64(&mut self, value: f64) -> Result<(), ()> {
        self.state.push(State::Bson(Bson::FloatingPoint(value as f64)));
        Ok(())
    }

    #[inline]
    fn visit_char(&mut self, value: char) -> Result<(), ()> {
        let mut s = String::new();
        s.push(value);
        self.visit_str(&s)
    }

    #[inline]
    fn visit_str(&mut self, value: &str) -> Result<(), ()> {
        self.state.push(State::Bson(Bson::String(String::from(value))));
        Ok(())
    }

    #[inline]
    fn visit_none(&mut self) -> Result<(), ()> {
        self.visit_unit()
    }

    #[inline]
    fn visit_some<V>(&mut self, value: V) -> Result<(), ()>
        where V: Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn visit_unit(&mut self) -> Result<(), ()> {
        self.state.push(State::Bson(Bson::Null));
        Ok(())
    }

    #[inline]
    fn visit_unit_variant(&mut self,
                          _name: &str,
                          _variant_index: usize,
                          variant: &str) -> Result<(), ()> {
        let mut values = bson::Document::new();
        values.insert(String::from(variant), Bson::Array(vec![]));

        self.state.push(State::Bson(Bson::Document(values)));

        Ok(())
    }

    #[inline]
    fn visit_newtype_variant<T>(&mut self,
                                _name: &str,
                                _variant_index: usize,
                                variant: &str,
                                value: T) -> Result<(), ()>
        where T: Serialize,
    {
        let mut values = bson::Document::new();
        values.insert(String::from(variant), to_bson(&value));

        self.state.push(State::Bson(Bson::Document(values)));

        Ok(())
    }

    #[inline]
    fn visit_seq<V>(&mut self, mut visitor: V) -> Result<(), ()>
        where V: SeqVisitor,
    {
        let len = visitor.len().unwrap_or(0);
        let values = Vec::with_capacity(len);

        self.state.push(State::Array(values));

        while let Some(()) = try!(visitor.visit(self)) { }

        let values = match self.state.pop().unwrap() {
            State::Array(values) => values,
            state => panic!("Expected array, found {:?}", state),
        };

        self.state.push(State::Bson(Bson::Array(values)));

        Ok(())
    }

    #[inline]
    fn visit_tuple_variant<V>(&mut self,
                              _name: &str,
                              _variant_index: usize,
                              variant: &str,
                              visitor: V) -> Result<(), ()>
        where V: SeqVisitor,
    {
        try!(self.visit_seq(visitor));

        let value = match self.state.pop().unwrap() {
            State::Bson(value) => value,
            state => panic!("expected value, found {:?}", state),
        };

        let mut object = bson::Document::new();

        object.insert(String::from(variant), value);

        self.state.push(State::Bson(Bson::Document(object)));

        Ok(())
    }

    #[inline]
    fn visit_seq_elt<T>(&mut self, value: T) -> Result<(), ()>
        where T: Serialize,
    {
        try!(value.serialize(self));

        let value = match self.state.pop().unwrap() {
            State::Bson(value) => value,
            state => panic!("expected value, found {:?}", state),
        };

        match *self.state.last_mut().unwrap() {
            State::Array(ref mut values) => { values.push(value); }
            ref state => panic!("expected array, found {:?}", state),
        }

        Ok(())
    }

    #[inline]
    fn visit_map<V>(&mut self, mut visitor: V) -> Result<(), ()>
        where V: MapVisitor,
    {
        let values = bson::Document::new();

        self.state.push(State::Document(values));

        while let Some(()) = try!(visitor.visit(self)) { }

        let values = match self.state.pop().unwrap() {
            State::Document(values) => values,
            state => panic!("expected object, found {:?}", state),
        };

        let bson = Bson::from_extended_document(values);
        
        self.state.push(State::Bson(bson));
        Ok(())
    }

    #[inline]
    fn visit_struct_variant<V>(&mut self,
                               _name: &str,
                               _variant_index: usize,
                               variant: &str,
                               visitor: V) -> Result<(), ()>
        where V: MapVisitor,
    {
        try!(self.visit_map(visitor));

        let value = match self.state.pop().unwrap() {
            State::Bson(value) => value,
            state => panic!("expected value, found {:?}", state),
        };

        let mut object = bson::Document::new();

        object.insert(String::from(variant), value);

        self.state.push(State::Bson(Bson::Document(object)));

        Ok(())
    }

    #[inline]
    fn visit_map_elt<K, V>(&mut self, key: K, value: V) -> Result<(), ()>
        where K: Serialize,
              V: Serialize,
    {
        try!(key.serialize(self));

        let key = match self.state.pop().unwrap() {
            State::Bson(Bson::String(value)) => value,
            state => panic!("expected key, found {:?}", state),
        };

        try!(value.serialize(self));

        let value = match self.state.pop().unwrap() {
            State::Bson(value) => value,
            state => panic!("expected value, found {:?}", state),
        };

        if Bson::Null != value {
            match *self.state.last_mut().unwrap() {
                State::Document(ref mut values) => { values.insert(key, value); }
                ref state => panic!("expected object, found {:?}", state),
            }
        }

        Ok(())
    }

    #[inline]
    fn format() -> &'static str {
        "bson"
    }
}
