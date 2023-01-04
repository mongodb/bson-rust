use std::{convert::TryFrom, io::Write};

use serde::{
    ser::{Error as SerdeError, Impossible, SerializeMap, SerializeStruct},
    Serialize,
};

use crate::{
    oid::ObjectId,
    raw::RAW_DOCUMENT_NEWTYPE,
    ser::{write_binary, write_cstring, write_i32, write_i64, write_string, Error, Result},
    spec::{BinarySubtype, ElementType},
    RawDocument,
    RawJavaScriptCodeWithScopeRef,
};

use super::{document_serializer::DocumentSerializer, Serializer};

/// A serializer used specifically for serializing the serde-data-model form of a BSON type (e.g.
/// [`Binary`]) to raw bytes.
pub(crate) struct ValueSerializer<'a> {
    root_serializer: &'a mut Serializer,
    state: SerializationStep,
}

/// State machine used to track which step in the serialization of a given type the serializer is
/// currently on.
#[derive(Debug)]
enum SerializationStep {
    Oid,

    DateTime,
    DateTimeNumberLong,

    Binary,
    /// This step can either transition to the raw or base64 steps depending
    /// on whether a string or bytes are serialized.
    BinaryBytes,
    BinarySubType {
        base64: String,
    },
    RawBinarySubType {
        bytes: Vec<u8>,
    },

    Symbol,

    RegEx,
    RegExPattern,
    RegExOptions,

    Timestamp,
    TimestampTime,
    TimestampIncrement {
        time: i64,
    },

    DbPointer,
    DbPointerRef,
    DbPointerId,

    Code,

    CodeWithScopeCode,
    CodeWithScopeScope {
        code: String,
        raw: bool,
    },

    MinKey,

    MaxKey,

    Undefined,

    Decimal128,
    Decimal128Value,

    Done,
}

/// Enum of BSON "value" types that this serializer can serialize.
#[derive(Debug, Clone, Copy)]
pub(super) enum ValueType {
    DateTime,
    Binary,
    ObjectId,
    Symbol,
    RegularExpression,
    Timestamp,
    DbPointer,
    JavaScriptCode,
    JavaScriptCodeWithScope,
    MinKey,
    MaxKey,
    Decimal128,
    Undefined,
}

impl From<ValueType> for ElementType {
    fn from(vt: ValueType) -> Self {
        match vt {
            ValueType::Binary => ElementType::Binary,
            ValueType::DateTime => ElementType::DateTime,
            ValueType::DbPointer => ElementType::DbPointer,
            ValueType::Decimal128 => ElementType::Decimal128,
            ValueType::Symbol => ElementType::Symbol,
            ValueType::RegularExpression => ElementType::RegularExpression,
            ValueType::Timestamp => ElementType::Timestamp,
            ValueType::JavaScriptCode => ElementType::JavaScriptCode,
            ValueType::JavaScriptCodeWithScope => ElementType::JavaScriptCodeWithScope,
            ValueType::MaxKey => ElementType::MaxKey,
            ValueType::MinKey => ElementType::MinKey,
            ValueType::Undefined => ElementType::Undefined,
            ValueType::ObjectId => ElementType::ObjectId,
        }
    }
}

impl<'a> ValueSerializer<'a> {
    pub(super) fn new(rs: &'a mut Serializer, value_type: ValueType) -> Self {
        let state = match value_type {
            ValueType::DateTime => SerializationStep::DateTime,
            ValueType::Binary => SerializationStep::Binary,
            ValueType::ObjectId => SerializationStep::Oid,
            ValueType::Symbol => SerializationStep::Symbol,
            ValueType::RegularExpression => SerializationStep::RegEx,
            ValueType::Timestamp => SerializationStep::Timestamp,
            ValueType::DbPointer => SerializationStep::DbPointer,
            ValueType::JavaScriptCode => SerializationStep::Code,
            ValueType::JavaScriptCodeWithScope => SerializationStep::CodeWithScopeCode,
            ValueType::MinKey => SerializationStep::MinKey,
            ValueType::MaxKey => SerializationStep::MaxKey,
            ValueType::Decimal128 => SerializationStep::Decimal128,
            ValueType::Undefined => SerializationStep::Undefined,
        };
        Self {
            root_serializer: rs,
            state,
        }
    }

    fn invalid_step(&self, primitive_type: &'static str) -> Error {
        Error::custom(format!(
            "cannot serialize {} at step {:?}",
            primitive_type, self.state
        ))
    }
}

impl<'a, 'b> serde::Serializer for &'b mut ValueSerializer<'a> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = CodeWithScopeSerializer<'b>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Impossible<(), Error>;

    #[inline]
    fn serialize_bool(self, _v: bool) -> Result<Self::Ok> {
        Err(self.invalid_step("bool"))
    }

    #[inline]
    fn serialize_i8(self, _v: i8) -> Result<Self::Ok> {
        Err(self.invalid_step("i8"))
    }

    #[inline]
    fn serialize_i16(self, _v: i16) -> Result<Self::Ok> {
        Err(self.invalid_step("i16"))
    }

    #[inline]
    fn serialize_i32(self, _v: i32) -> Result<Self::Ok> {
        Err(self.invalid_step("i32"))
    }

    #[inline]
    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        match self.state {
            SerializationStep::TimestampTime => {
                self.state = SerializationStep::TimestampIncrement { time: v };
                Ok(())
            }
            SerializationStep::TimestampIncrement { time } => {
                let t = u32::try_from(time).map_err(Error::custom)?;
                let i = u32::try_from(v).map_err(Error::custom)?;

                write_i32(&mut self.root_serializer.bytes, i as i32)?;
                write_i32(&mut self.root_serializer.bytes, t as i32)?;
                Ok(())
            }
            _ => Err(self.invalid_step("i64")),
        }
    }

    #[inline]
    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        match self.state {
            SerializationStep::RawBinarySubType { ref bytes } => {
                write_binary(&mut self.root_serializer.bytes, bytes.as_slice(), v.into())?;
                self.state = SerializationStep::Done;
                Ok(())
            }
            _ => Err(self.invalid_step("u8")),
        }
    }

    #[inline]
    fn serialize_u16(self, _v: u16) -> Result<Self::Ok> {
        Err(self.invalid_step("u16"))
    }

    #[inline]
    fn serialize_u32(self, _v: u32) -> Result<Self::Ok> {
        Err(self.invalid_step("u32"))
    }

    #[inline]
    fn serialize_u64(self, _v: u64) -> Result<Self::Ok> {
        Err(self.invalid_step("u64"))
    }

    #[inline]
    fn serialize_f32(self, _v: f32) -> Result<Self::Ok> {
        Err(self.invalid_step("f32"))
    }

    #[inline]
    fn serialize_f64(self, _v: f64) -> Result<Self::Ok> {
        Err(self.invalid_step("f64"))
    }

    #[inline]
    fn serialize_char(self, _v: char) -> Result<Self::Ok> {
        Err(self.invalid_step("char"))
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        match &self.state {
            SerializationStep::DateTimeNumberLong => {
                let millis: i64 = v.parse().map_err(Error::custom)?;
                write_i64(&mut self.root_serializer.bytes, millis)?;
            }
            SerializationStep::Oid => {
                let oid = ObjectId::parse_str(v).map_err(Error::custom)?;
                self.root_serializer.bytes.write_all(&oid.bytes())?;
            }
            SerializationStep::BinaryBytes => {
                self.state = SerializationStep::BinarySubType {
                    base64: v.to_string(),
                };
            }
            SerializationStep::BinarySubType { base64 } => {
                let subtype_byte = hex::decode(v).map_err(Error::custom)?;
                let subtype: BinarySubtype = subtype_byte[0].into();

                let bytes = base64::decode(base64.as_str()).map_err(Error::custom)?;

                write_binary(&mut self.root_serializer.bytes, bytes.as_slice(), subtype)?;
            }
            SerializationStep::Symbol | SerializationStep::DbPointerRef => {
                write_string(&mut self.root_serializer.bytes, v)?;
            }
            SerializationStep::RegExPattern => {
                write_cstring(&mut self.root_serializer.bytes, v)?;
            }
            SerializationStep::RegExOptions => {
                let mut chars: Vec<_> = v.chars().collect();
                chars.sort_unstable();

                let sorted = chars.into_iter().collect::<String>();
                write_cstring(&mut self.root_serializer.bytes, sorted.as_str())?;
            }
            SerializationStep::Code => {
                write_string(&mut self.root_serializer.bytes, v)?;
            }
            SerializationStep::CodeWithScopeCode => {
                self.state = SerializationStep::CodeWithScopeScope {
                    code: v.to_string(),
                    raw: false,
                };
            }
            s => {
                return Err(Error::custom(format!(
                    "can't serialize string for step {:?}",
                    s
                )))
            }
        }
        Ok(())
    }

    #[inline]
    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        match self.state {
            SerializationStep::Decimal128Value => {
                self.root_serializer.bytes.write_all(v)?;
                Ok(())
            }
            SerializationStep::BinaryBytes => {
                self.state = SerializationStep::RawBinarySubType { bytes: v.to_vec() };
                Ok(())
            }
            SerializationStep::CodeWithScopeScope { ref code, raw } if raw => {
                let raw = RawJavaScriptCodeWithScopeRef {
                    code,
                    scope: RawDocument::from_bytes(v).map_err(Error::custom)?,
                };
                write_i32(&mut self.root_serializer.bytes, raw.len())?;
                write_string(&mut self.root_serializer.bytes, code)?;
                self.root_serializer.bytes.write_all(v)?;
                self.state = SerializationStep::Done;
                Ok(())
            }
            _ => Err(self.invalid_step("&[u8]")),
        }
    }

    #[inline]
    fn serialize_none(self) -> Result<Self::Ok> {
        Err(self.invalid_step("none"))
    }

    #[inline]
    fn serialize_some<T: ?Sized>(self, _value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        Err(self.invalid_step("some"))
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok> {
        Err(self.invalid_step("unit"))
    }

    #[inline]
    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok> {
        Err(self.invalid_step("unit_struct"))
    }

    #[inline]
    fn serialize_unit_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok> {
        Err(self.invalid_step("unit_variant"))
    }

    #[inline]
    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        match (&mut self.state, name) {
            (
                SerializationStep::CodeWithScopeScope {
                    code: _,
                    ref mut raw,
                },
                RAW_DOCUMENT_NEWTYPE,
            ) => {
                *raw = true;
                value.serialize(self)
            }
            _ => Err(self.invalid_step("newtype_struct")),
        }
    }

    #[inline]
    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _value: &T,
    ) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        Err(self.invalid_step("newtype_variant"))
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        Err(self.invalid_step("seq"))
    }

    #[inline]
    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Err(self.invalid_step("newtype_tuple"))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Err(self.invalid_step("tuple_struct"))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        Err(self.invalid_step("tuple_variant"))
    }

    #[inline]
    fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap> {
        match self.state {
            SerializationStep::CodeWithScopeScope { ref code, raw } if !raw => {
                CodeWithScopeSerializer::start(code.as_str(), self.root_serializer)
            }
            _ => Err(self.invalid_step("map")),
        }
    }

    #[inline]
    fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        Ok(self)
    }

    #[inline]
    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        Err(self.invalid_step("struct_variant"))
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

impl<'a, 'b> SerializeStruct for &'b mut ValueSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        match (&self.state, key) {
            (SerializationStep::DateTime, "$date") => {
                self.state = SerializationStep::DateTimeNumberLong;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::DateTimeNumberLong, "$numberLong") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Oid, "$oid") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Binary, "$binary") => {
                self.state = SerializationStep::BinaryBytes;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::BinaryBytes, key) if key == "bytes" || key == "base64" => {
                // state is updated in serialize
                value.serialize(&mut **self)?;
            }
            (SerializationStep::RawBinarySubType { .. }, "subType") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::BinarySubType { .. }, "subType") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Symbol, "$symbol") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::RegEx, "$regularExpression") => {
                self.state = SerializationStep::RegExPattern;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::RegExPattern, "pattern") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::RegExOptions;
            }
            (SerializationStep::RegExOptions, "options") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Timestamp, "$timestamp") => {
                self.state = SerializationStep::TimestampTime;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::TimestampTime, "t") => {
                // state is updated in serialize
                value.serialize(&mut **self)?;
            }
            (SerializationStep::TimestampIncrement { .. }, "i") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::DbPointer, "$dbPointer") => {
                self.state = SerializationStep::DbPointerRef;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::DbPointerRef, "$ref") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::DbPointerId;
            }
            (SerializationStep::DbPointerId, "$id") => {
                self.state = SerializationStep::Oid;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::Code, "$code") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::CodeWithScopeCode, "$code") => {
                // state is updated in serialize
                value.serialize(&mut **self)?;
            }
            (SerializationStep::CodeWithScopeScope { .. }, "$scope") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::MinKey, "$minKey") => {
                self.state = SerializationStep::Done;
            }
            (SerializationStep::MaxKey, "$maxKey") => {
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Undefined, "$undefined") => {
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Decimal128, "$numberDecimal")
            | (SerializationStep::Decimal128, "$numberDecimalBytes") => {
                self.state = SerializationStep::Decimal128Value;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::Decimal128Value, "$numberDecimal") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::Done, k) => {
                return Err(Error::custom(format!(
                    "expected to end serialization of type, got extra key \"{}\"",
                    k
                )));
            }
            (state, k) => {
                return Err(Error::custom(format!(
                    "mismatched serialization step and next key: {:?} + \"{}\"",
                    state, k
                )));
            }
        }

        Ok(())
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

pub(crate) struct CodeWithScopeSerializer<'a> {
    start: usize,
    doc: DocumentSerializer<'a>,
}

impl<'a> CodeWithScopeSerializer<'a> {
    #[inline]
    fn start(code: &str, rs: &'a mut Serializer) -> Result<Self> {
        let start = rs.bytes.len();
        write_i32(&mut rs.bytes, 0)?; // placeholder length
        write_string(&mut rs.bytes, code)?;

        let doc = DocumentSerializer::start(rs)?;
        Ok(Self { start, doc })
    }
}

impl<'a> SerializeMap for CodeWithScopeSerializer<'a> {
    type Ok = ();
    type Error = Error;

    #[inline]
    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.doc.serialize_key(key)
    }

    #[inline]
    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.doc.serialize_value(value)
    }

    #[inline]
    fn end(self) -> Result<Self::Ok> {
        let result = self.doc.end_doc()?;

        let total_len = (result.root_serializer.bytes.len() - self.start) as i32;
        result.root_serializer.replace_i32(self.start, total_len);
        Ok(())
    }
}
