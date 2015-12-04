use std::fmt;
use std::io;
use std::vec;
use std::error;
use std::string::FromUtf8Error;

use bson::{self, Bson};
use ordered::OrderedDocumentIntoIterator;
use serde::{de, ser};

/// The errors that can arise while parsing a JSON stream.
#[derive(Clone, PartialEq)]
pub enum ErrorCode {
    /// EOF while parsing a list.
    EOFWhileParsingList,

    /// EOF while parsing an object.
    EOFWhileParsingObject,

    /// EOF while parsing a string.
    EOFWhileParsingString,

    /// EOF while parsing a JSON value.
    EOFWhileParsingValue,

    /// Expected this character to be a `':'`.
    ExpectedColon,

    /// Expected this character to be either a `','` or a `]`.
    ExpectedListCommaOrEnd,

    /// Expected this character to be either a `','` or a `}`.
    ExpectedObjectCommaOrEnd,

    /// Expected to parse either a `true`, `false`, or a `null`.
    ExpectedSomeIdent,

    /// Expected this character to start a JSON value.
    ExpectedSomeValue,

    /// Invalid hex escape code.
    InvalidEscape,

    /// Invalid number.
    InvalidNumber,

    /// Invalid unicode code point.
    InvalidUnicodeCodePoint,

    /// Object key is not a string.
    KeyMustBeAString,

    /// Lone leading surrogate in hex escape.
    LoneLeadingSurrogateInHexEscape,

    /// Unknown field in struct.
    UnknownField(String),

    /// Struct is missing a field.
    MissingField(&'static str),

    /// JSON has non-whitespace trailing characters after the value.
    TrailingCharacters,

    /// Unexpected end of hex excape.
    UnexpectedEndOfHexEscape,
}

impl fmt::Debug for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::fmt::Debug;

        match *self {
            ErrorCode::EOFWhileParsingList => "EOF while parsing a list".fmt(f),
            ErrorCode::EOFWhileParsingObject => "EOF while parsing an object".fmt(f),
            ErrorCode::EOFWhileParsingString => "EOF while parsing a string".fmt(f),
            ErrorCode::EOFWhileParsingValue => "EOF while parsing a value".fmt(f),
            ErrorCode::ExpectedColon => "expected `:`".fmt(f),
            ErrorCode::ExpectedListCommaOrEnd => "expected `,` or `]`".fmt(f),
            ErrorCode::ExpectedObjectCommaOrEnd => "expected `,` or `}`".fmt(f),
            ErrorCode::ExpectedSomeIdent => "expected ident".fmt(f),
            ErrorCode::ExpectedSomeValue => "expected value".fmt(f),
            ErrorCode::InvalidEscape => "invalid escape".fmt(f),
            ErrorCode::InvalidNumber => "invalid number".fmt(f),
            ErrorCode::InvalidUnicodeCodePoint => "invalid unicode code point".fmt(f),
            ErrorCode::KeyMustBeAString => "key must be a string".fmt(f),
            ErrorCode::LoneLeadingSurrogateInHexEscape => "lone leading surrogate in hex escape".fmt(f),
            ErrorCode::UnknownField(ref field) => write!(f, "unknown field \"{}\"", field),
            ErrorCode::MissingField(ref field) => write!(f, "missing field \"{}\"", field),
            ErrorCode::TrailingCharacters => "trailing characters".fmt(f),
            ErrorCode::UnexpectedEndOfHexEscape => "unexpected end of hex escape".fmt(f),
        }
    }
}

/// This type represents all possible errors that can occur when serializing or deserializing a
/// value into JSON.
#[derive(Debug)]
pub enum Error {
    /// The JSON value had some syntatic error.
    SyntaxError(ErrorCode, usize, usize),

    /// Some IO error occurred when serializing or deserializing a value.
    IoError(io::Error),

    /// Some UTF8 error occurred while serializing or deserializing a value.
    FromUtf8Error(FromUtf8Error),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::SyntaxError(..) => "syntax error",
            Error::IoError(ref error) => error::Error::description(error),
            Error::FromUtf8Error(ref error) => error.description(),
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::IoError(ref error) => Some(error),
            Error::FromUtf8Error(ref error) => Some(error),
            _ => None,
        }
    }

}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::SyntaxError(ref code, line, col) => {
                write!(fmt, "{:?} at line {} column {}", code, line, col)
            }
            Error::IoError(ref error) => fmt::Display::fmt(error, fmt),
            Error::FromUtf8Error(ref error) => fmt::Display::fmt(error, fmt),
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Error {
        Error::IoError(error)
    }
}

impl From<FromUtf8Error> for Error {
    fn from(error: FromUtf8Error) -> Error {
        Error::FromUtf8Error(error)
    }
}

impl From<de::value::Error> for Error {
    fn from(error: de::value::Error) -> Error {
        match error {
            de::value::Error::SyntaxError => {
                Error::SyntaxError(ErrorCode::ExpectedSomeValue, 0, 0)
            }
            de::value::Error::EndOfStreamError => {
                de::Error::end_of_stream()
            }
            de::value::Error::UnknownFieldError(field) => {
                Error::SyntaxError(ErrorCode::UnknownField(field), 0, 0)
            }
            de::value::Error::MissingFieldError(field) => {
                Error::SyntaxError(ErrorCode::MissingField(field), 0, 0)
            }
        }
    }
}

impl de::Error for Error {
    fn syntax(_: &str) -> Error {
        Error::SyntaxError(ErrorCode::ExpectedSomeValue, 0, 0)
    }

    fn end_of_stream() -> Error {
        Error::SyntaxError(ErrorCode::EOFWhileParsingValue, 0, 0)
    }

    fn unknown_field(field: &str) -> Error {
        Error::SyntaxError(ErrorCode::UnknownField(String::from(field)), 0, 0)
    }

    fn missing_field(field: &'static str) -> Error {
        Error::SyntaxError(ErrorCode::MissingField(field), 0, 0)
    }
}

pub struct BsonVisitor;

impl de::Visitor for BsonVisitor {
    type Value = Bson;
    
    #[inline]
    fn visit_bool<E>(&mut self, value: bool) -> Result<Bson, E> {
        Ok(Bson::Boolean(value))
    }
    
    #[inline]
    fn visit_i64<E>(&mut self, value: i64) -> Result<Bson, E> {
        Ok(Bson::I64(value))
    }
    
    #[inline]
    fn visit_u64<E>(&mut self, value: u64) -> Result<Bson, E> {
        Ok(Bson::FloatingPoint(value as f64))
    }
    
    #[inline]
    fn visit_f64<E>(&mut self, value: f64) -> Result<Bson, E> {
        Ok(Bson::FloatingPoint(value))
    }
    
    #[inline]
    fn visit_str<E>(&mut self, value: &str) -> Result<Bson, E>
        where E: de::Error,
    {
        self.visit_string(String::from(value))
    }
    
    #[inline]
    fn visit_string<E>(&mut self, value: String) -> Result<Bson, E> {
        Ok(Bson::String(value))
    }
    
    #[inline]
    fn visit_none<E>(&mut self) -> Result<Bson, E> {
        Ok(Bson::Null)
    }
    
    #[inline]
    fn visit_some<D>(&mut self, deserializer: &mut D) -> Result<Bson, D::Error>
        where D: de::Deserializer,
    {
        de::Deserialize::deserialize(deserializer)
    }
    
    #[inline]
    fn visit_unit<E>(&mut self) -> Result<Bson, E> {
        Ok(Bson::Null)
    }
    
    #[inline]
    fn visit_seq<V>(&mut self, visitor: V) -> Result<Bson, V::Error>
        where V: de::SeqVisitor,
    {
        let values = try!(de::impls::VecVisitor::new().visit_seq(visitor));
        Ok(Bson::Array(values))
    }
    
    #[inline]
    fn visit_map<V>(&mut self, visitor: V) -> Result<Bson, V::Error>
        where V: de::MapVisitor,
    {
        let values = try!(de::impls::BTreeMapVisitor::new().visit_map(visitor));
        let values2 = values.clone();
        println!("{:?}", Bson::from_extended_document(values2.into()));
        Ok(Bson::from_extended_document(values.into()).unwrap())
    }
}


#[derive(Debug)]
enum State {
    Bson(Bson),
    Array(Vec<Bson>),
    Document(bson::Document),
}

pub struct Serializer {
    state: Vec<State>,
}

impl Serializer {
    /// Construct a new `Serializer`.
    pub fn new() -> Serializer {
        Serializer {
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

impl ser::Serializer for Serializer {
    type Error = ();

    #[inline]
    fn visit_bool(&mut self, value: bool) -> Result<(), ()> {
        self.state.push(State::Bson(Bson::Boolean(value)));
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
        where V: ser::Serialize,
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
        where T: ser::Serialize,
    {
        let mut values = bson::Document::new();
        values.insert(String::from(variant), to_bson(&value));

        self.state.push(State::Bson(Bson::Document(values)));

        Ok(())
    }

    #[inline]
    fn visit_seq<V>(&mut self, mut visitor: V) -> Result<(), ()>
        where V: ser::SeqVisitor,
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
        where V: ser::SeqVisitor,
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
        where T: ser::Serialize,
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
        where V: ser::MapVisitor,
    {
        let values = bson::Document::new();

        self.state.push(State::Document(values));

        while let Some(()) = try!(visitor.visit(self)) { }

        let values = match self.state.pop().unwrap() {
            State::Document(values) => values,
            state => panic!("expected object, found {:?}", state),
        };

        println!("VALUES FROM VISIT_MAP: {:?} // {:?}", &values, values.get("$oid"));
        let bson = Bson::from_extended_document(values).unwrap();
        
        self.state.push(State::Bson(bson));
        Ok(())
    }

    #[inline]
    fn visit_struct_variant<V>(&mut self,
                               _name: &str,
                               _variant_index: usize,
                               variant: &str,
                               visitor: V) -> Result<(), ()>
        where V: ser::MapVisitor,
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
        where K: ser::Serialize,
              V: ser::Serialize,
    {
        try!(key.serialize(self));

        let key = match self.state.pop().unwrap() {
            State::Bson(Bson::String(value)) => value,
            state => panic!("expected key, found {:?}", state),
        };

        println!("SERIALIZED KEY: {:?}", &key);

        try!(value.serialize(self));

        let value = match self.state.pop().unwrap() {
            State::Bson(value) => value,
            state => panic!("expected value, found {:?}", state),
        };

        println!("SERIALIZED VALUE: {:?}", &value);

        match *self.state.last_mut().unwrap() {
            State::Document(ref mut values) => { values.insert(key, value); }
            ref state => panic!("expected object, found {:?}", state),
        }

        Ok(())
    }

    #[inline]
    fn format() -> &'static str {
        "bson"
    }
}

/// Creates a `serde::Deserializer` from a `json::Value` object.
pub struct Deserializer {
    value: Option<Bson>,
}

impl Deserializer {
    /// Creates a new deserializer instance for deserializing the specified JSON value.
    pub fn new(value: Bson) -> Deserializer {
        Deserializer {
            value: Some(value),
        }
    }
}

impl de::Deserializer for Deserializer {
    type Error = Error;

    #[inline]
    fn visit<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
        where V: de::Visitor,
    {
        let value = match self.value.take() {
            Some(value) => value,
            None => { return Err(de::Error::end_of_stream()); }
        };

        match value {
            Bson::FloatingPoint(v) => visitor.visit_f64(v),
            Bson::String(v) => visitor.visit_string(v),
            Bson::Array(v) => {
                let len = v.len();
                visitor.visit_seq(SeqDeserializer {
                    de: self,
                    iter: v.into_iter(),
                    len: len,
                })
            }
            Bson::Document(v) => {
                let len = v.len();
                visitor.visit_map(MapDeserializer {
                    de: self,
                    iter: v.into_iter(),
                    value: None,
                    len: len,
                })
            }
            Bson::Boolean(v) => visitor.visit_bool(v),
            Bson::Null => visitor.visit_unit(),
            Bson::I32(v) => visitor.visit_i32(v),
            Bson::I64(v) => visitor.visit_i64(v),
            _ => {
                let doc = value.to_extended_document();
                let len = doc.len();
                println!("{:?}", doc);
                visitor.visit_map(MapDeserializer {
                    de: self,
                    iter: doc.into_iter(),
                    value: None,
                    len: len,
                })
            }
        }
    }

    #[inline]
    fn visit_option<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
        where V: de::Visitor,
    {
        match self.value {
            Some(Bson::Null) => visitor.visit_none(),
            Some(_) => visitor.visit_some(self),
            None => Err(de::Error::end_of_stream()),
        }
    }

    #[inline]
    fn visit_enum<V>(&mut self,
                     _name: &str,
                     _variants: &'static [&'static str],
                     mut visitor: V) -> Result<V::Value, Error>
        where V: de::EnumVisitor,
    {
        let value = match self.value.take() {
            Some(Bson::Document(value)) => value,
            Some(_) => { return Err(de::Error::syntax("expected an enum")); }
            None => { return Err(de::Error::end_of_stream()); }
        };

        let mut iter = value.into_iter();

        let (variant, value) = match iter.next() {
            Some(v) => v,
            None => return Err(de::Error::syntax("expected a variant name")),
        };

        // enums are encoded in json as maps with a single key:value pair
        match iter.next() {
            Some(_) => Err(de::Error::syntax("expected map")),
            None => visitor.visit(VariantDeserializer {
                de: self,
                val: Some(value),
                variant: Some(Bson::String(variant)),
            }),
        }
    }

    #[inline]
    fn visit_newtype_struct<V>(&mut self,
                               _name: &'static str,
                               mut visitor: V) -> Result<V::Value, Self::Error>
        where V: de::Visitor,
    {
        visitor.visit_newtype_struct(self)
    }

    #[inline]
    fn format() -> &'static str {
        "json"
    }
}

struct VariantDeserializer<'a> {
    de: &'a mut Deserializer,
    val: Option<Bson>,
    variant: Option<Bson>,
}

impl<'a> de::VariantVisitor for VariantDeserializer<'a> {
    type Error = Error;

    fn visit_variant<V>(&mut self) -> Result<V, Error>
        where V: de::Deserialize,
    {
        de::Deserialize::deserialize(&mut Deserializer::new(self.variant.take().unwrap()))
    }

    fn visit_unit(&mut self) -> Result<(), Error> {
        de::Deserialize::deserialize(&mut Deserializer::new(self.val.take().unwrap()))
    }

    fn visit_newtype<T>(&mut self) -> Result<T, Error>
        where T: de::Deserialize,
    {
        de::Deserialize::deserialize(&mut Deserializer::new(self.val.take().unwrap()))
    }

    fn visit_tuple<V>(&mut self,
                      _len: usize,
                      visitor: V) -> Result<V::Value, Error>
        where V: de::Visitor,
    {
        if let Bson::Array(fields) = self.val.take().unwrap() {
            de::Deserializer::visit(
                &mut SeqDeserializer {
                    de: self.de,
                    len: fields.len(),
                    iter: fields.into_iter(),
                },
                visitor,
            )
        } else {
            Err(de::Error::syntax("expected a tuple"))
        }
    }

    fn visit_struct<V>(&mut self,
                       _fields: &'static[&'static str],
                       visitor: V) -> Result<V::Value, Error>
        where V: de::Visitor,
    {
        if let Bson::Document(fields) = self.val.take().unwrap() {
            de::Deserializer::visit(
                &mut MapDeserializer {
                    de: self.de,
                    len: fields.len(),
                    iter: fields.into_iter(),
                    value: None,
                },
                visitor,
            )
        } else {
            Err(de::Error::syntax("expected a struct"))
        }
    }
}

struct SeqDeserializer<'a> {
    de: &'a mut Deserializer,
    iter: vec::IntoIter<Bson>,
    len: usize,
}

impl<'a> de::Deserializer for SeqDeserializer<'a> {
    type Error = Error;

    #[inline]
    fn visit<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
        where V: de::Visitor,
    {
        if self.len == 0 {
            visitor.visit_unit()
        } else {
            visitor.visit_seq(self)
        }
    }
}

impl<'a> de::SeqVisitor for SeqDeserializer<'a> {
    type Error = Error;

    fn visit<T>(&mut self) -> Result<Option<T>, Error>
        where T: de::Deserialize
    {
        match self.iter.next() {
            Some(value) => {
                self.len -= 1;
                self.de.value = Some(value);
                Ok(Some(try!(de::Deserialize::deserialize(self.de))))
            }
            None => Ok(None),
        }
    }

    fn end(&mut self) -> Result<(), Error> {
        if self.len == 0 {
            Ok(())
        } else {
            Err(de::Error::length_mismatch(self.len))
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

struct MapDeserializer<'a> {
    de: &'a mut Deserializer,
    iter: OrderedDocumentIntoIterator,
    value: Option<Bson>,
    len: usize,
}

impl<'a> de::MapVisitor for MapDeserializer<'a> {
    type Error = Error;

    fn visit_key<T>(&mut self) -> Result<Option<T>, Error>
        where T: de::Deserialize
    {
        match self.iter.next() {
            Some((key, value)) => {
                self.len -= 1;
                self.value = Some(value);
                self.de.value = Some(Bson::String(key));
                Ok(Some(try!(de::Deserialize::deserialize(self.de))))
            }
            None => Ok(None),
        }
    }

    fn visit_value<T>(&mut self) -> Result<T, Error>
        where T: de::Deserialize
    {
        let value = self.value.take().unwrap();
        self.de.value = Some(value);
        Ok(try!(de::Deserialize::deserialize(self.de)))
    }

    fn end(&mut self) -> Result<(), Error> {
        if self.len == 0 {
            Ok(())
        } else {
            Err(de::Error::length_mismatch(self.len))
        }
    }

    fn missing_field<V>(&mut self, _field: &'static str) -> Result<V, Error>
        where V: de::Deserialize,
    {
        // See if the type can deserialize from a unit.
        struct UnitDeserializer;

        impl de::Deserializer for UnitDeserializer {
            type Error = Error;

            fn visit<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
                where V: de::Visitor,
            {
                visitor.visit_unit()
            }

            fn visit_option<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
                where V: de::Visitor,
            {
                visitor.visit_none()
            }
        }

        Ok(try!(de::Deserialize::deserialize(&mut UnitDeserializer)))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a> de::Deserializer for MapDeserializer<'a> {
    type Error = Error;

    #[inline]
    fn visit<V>(&mut self, mut visitor: V) -> Result<V::Value, Error>
        where V: de::Visitor,
    {
        visitor.visit_map(self)
    }
}

impl ser::Serialize for Bson {
    #[inline]
    fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: ser::Serializer,
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

impl de::Deserialize for Bson {
    #[inline]
    fn deserialize<D>(deserializer: &mut D) -> Result<Bson, D::Error>
        where D: de::Deserializer,
    {
        deserializer.visit(BsonVisitor)
    }
}

/// Encode a `T` Serializable into a BSON `Value`.
pub fn to_bson<T>(value: &T) -> Bson
    where T: ser::Serialize
{
    let mut ser = Serializer::new();
    value.serialize(&mut ser).unwrap();
    ser.unwrap()
}

/// Decode a BSON `Value` into a `T` Deserializable.
pub fn from_bson<T>(bson: Bson) -> Result<T, Error>
    where T: de::Deserialize
{
    let mut de = Deserializer::new(bson);
    de::Deserialize::deserialize(&mut de)
}
