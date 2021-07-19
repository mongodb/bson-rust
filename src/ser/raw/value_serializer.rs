use std::convert::TryFrom;
use std::io::Write;

use serde::ser::{Error as SerdeError, Impossible, SerializeMap, SerializeStruct};
use serde::Serialize;

use crate::oid::ObjectId;
use crate::ser::Result;
use crate::ser::{write_binary, write_cstring, write_i32, write_i64, write_string, Error};
use crate::spec::{BinarySubtype, ElementType};

use super::document_serializer::DocumentSerializer;
use super::Serializer;

/// A serializer used specifically for serializing the serde-data-model form of a BSON type (e.g. `Binary`) to
/// raw bytes.
pub(crate) struct ValueSerializer<'a> {
    root_serializer: &'a mut Serializer,
    state: SerializationStep,
}

/// State machine used to track which step in the serialization of a given type the serializer is currently on.
#[derive(Debug)]
enum SerializationStep {
    Oid,

    DateTime,
    DateTimeNumberLong,

    Binary,
    BinaryBase64,
    BinarySubType { base64: String },

    Symbol,

    RegEx,
    RegExPattern,
    RegExOptions,

    Timestamp,
    TimestampTime,
    TimestampIncrement { time: i64 },

    DbPointer,
    DbPointerRef,
    DbPointerId,

    Code,

    CodeWithScopeCode,
    CodeWithScopeScope { code: String },

    MinKey,

    MaxKey,

    Undefined,

    Decimal128,
    Decimal128Value,

    Done,
}

impl<'a> ValueSerializer<'a> {
    pub(super) fn new(rs: &'a mut Serializer, element_type: ElementType) -> Self {
        let state = match element_type {
            ElementType::DateTime => SerializationStep::DateTime,
            ElementType::Binary => SerializationStep::Binary,
            ElementType::ObjectId => SerializationStep::Oid,
            ElementType::Symbol => SerializationStep::Symbol,
            ElementType::RegularExpression => SerializationStep::RegEx,
            ElementType::Timestamp => SerializationStep::Timestamp,
            ElementType::DbPointer => SerializationStep::DbPointer,
            ElementType::JavaScriptCode => SerializationStep::Code,
            ElementType::JavaScriptCodeWithScope => SerializationStep::CodeWithScopeCode,
            ElementType::MinKey => SerializationStep::MinKey,
            ElementType::MaxKey => SerializationStep::MaxKey,
            ElementType::Decimal128 => SerializationStep::Decimal128,
            ElementType::Undefined => SerializationStep::Undefined,

            _ => todo!(),
        };
        Self {
            root_serializer: rs,
            state,
        }
    }

    fn invalid_step(&self, primitive_type: &'static str) -> Error {
        Error::custom(format!("cannot serialize {} at step {:?}", primitive_type, self.state))
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

    fn serialize_bool(self, _v: bool) -> Result<Self::Ok> {
        Err(self.invalid_step("bool"))
    }

    fn serialize_i8(self, _v: i8) -> Result<Self::Ok> {
        Err(self.invalid_step("i8"))
    }

    fn serialize_i16(self, _v: i16) -> Result<Self::Ok> {
        Err(self.invalid_step("i16"))
    }

    fn serialize_i32(self, _v: i32) -> Result<Self::Ok> {
        Err(self.invalid_step("i32"))
    }

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

    fn serialize_u8(self, v: u8) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_f32(self, v: f32) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        todo!()
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
            SerializationStep::BinaryBase64 => {
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
            SerializationStep::RegExPattern | SerializationStep::RegExOptions => {
                write_cstring(&mut self.root_serializer.bytes, v)?;
            }
            SerializationStep::Code => {
                write_string(&mut self.root_serializer.bytes, v)?;
            }
            SerializationStep::CodeWithScopeCode => {
                self.state = SerializationStep::CodeWithScopeScope {
                    code: v.to_string(),
                };
            }
            #[cfg(feature = "decimal128")]
            SerializationStep::Decimal128Value => {
                let d = Decimal128::from_str(v);
                self.root_serializer.write_all(d.to_raw_bytes_le())?;
            }
            s => return Err(Error::custom(format!("can't serialize string for step {:?}", s))),
        }
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        match self.state {
            SerializationStep::Decimal128Value => {
                self.root_serializer.bytes.write_all(v)?;
                Ok(())
            }
            _ => todo!(),
        }
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_unit(self) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_unit_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
    ) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_newtype_struct<T: ?Sized>(self, name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: Serialize,
    {
        todo!()
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        todo!()
    }

    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        todo!()
    }

    fn serialize_tuple_struct(
        self,
        name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        todo!()
    }

    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        todo!()
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        match self.state {
            SerializationStep::CodeWithScopeScope { ref code } => {
                CodeWithScopeSerializer::start(code.as_str(), self.root_serializer)
            }
            _ => todo!(),
        }
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        Ok(self)
    }

    fn serialize_struct_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        todo!()
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
                self.state = SerializationStep::BinaryBase64;
                value.serialize(&mut **self)?;
            }
            (SerializationStep::BinaryBase64, "base64") => {
                // state is updated in serialize
                value.serialize(&mut **self)?;
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
                    state,
                    k
                )));
            },
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

pub(crate) struct CodeWithScopeSerializer<'a> {
    start: usize,
    doc: DocumentSerializer<'a>,
}

impl<'a> CodeWithScopeSerializer<'a> {
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

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.doc.serialize_key(key)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.doc.serialize_value(value)
    }

    fn end(self) -> Result<Self::Ok> {
        let result = self.doc.end_doc()?;

        let total_len = (result.root_serializer.bytes.len() - self.start) as i32;
        result.root_serializer.replace_i32(self.start, total_len);
        Ok(())
    }
}
