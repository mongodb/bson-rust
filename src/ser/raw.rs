use std::io::Write;

use serde::{
    ser::{Error as SerdeError, Impossible, SerializeMap, SerializeStruct},
    Serialize, Serializer as SerdeSerializer,
};

use super::{write_cstring, write_f64, write_i32, write_i64, write_string, write_u8};
use crate::{Document, oid::ObjectId, ser::{write_binary, Error, Result}, spec::{BinarySubtype, ElementType}};

pub(crate) struct Serializer {
    bytes: Vec<u8>,
    type_index: usize,
}

impl Serializer {
    pub(crate) fn new() -> Self {
        Self {
            bytes: Vec::new(),
            type_index: 0,
        }
    }

    pub(crate) fn into_vec(self) -> Vec<u8> {
        self.bytes
    }

    fn update_element_type(&mut self, t: ElementType) -> Result<()> {
        if self.type_index == 0 {
            if matches!(t, ElementType::EmbeddedDocument) {
                // don't need to set the element type for the top level document
                return Ok(());
            } else {
                return Err(Error::custom(format!(
                    "attempted to encode a non-document type at the top level: {:?}",
                    t
                )));
            }
        }

        self.bytes[self.type_index] = t as u8;
        Ok(())
    }

    fn replace_i32(&mut self, at: usize, with: i32) {
        self.bytes.splice(
            at..at + 4,
            with.to_le_bytes().iter().cloned(),
        );
    }
}

impl<'a> serde::Serializer for &'a mut Serializer {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = DocumentSerializer<'a>;
    type SerializeTuple = DocumentSerializer<'a>;
    type SerializeTupleStruct = DocumentSerializer<'a>;
    type SerializeTupleVariant = VariantSerializer<'a>;
    type SerializeMap = DocumentSerializer<'a>;
    type SerializeStruct = StructSerializer<'a>;
    type SerializeStructVariant = VariantSerializer<'a>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Boolean)?;
        self.bytes.push(if v { 1 } else { 0 });
        Ok(())
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Int32)?;
        write_i32(&mut self.bytes, v.into())?;
        Ok(())
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Int32)?;
        write_i32(&mut self.bytes, v.into())?;
        Ok(())
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Int32)?;
        write_i32(&mut self.bytes, v)?;
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Int64)?;
        write_i64(&mut self.bytes, v)?;
        Ok(())
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
        self.update_element_type(ElementType::Double)?;
        write_f64(&mut self.bytes, v.into())
    }

    fn serialize_f64(self, v: f64) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Double)?;
        write_f64(&mut self.bytes, v.into())
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok> {
        self.update_element_type(ElementType::String)?;
        write_string(&mut self.bytes, v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Binary)?;
        let len = v.len() as i32;
        write_i32(&mut self.bytes, len)?;
        write_u8(&mut self.bytes, BinarySubtype::Generic.into())?;
        self.bytes.write_all(v)?;
        Ok(())
    }

    fn serialize_none(self) -> Result<Self::Ok> {
        self.update_element_type(ElementType::Null)?;
        Ok(())
    }

    fn serialize_some<T: ?Sized>(self, value: &T) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    #[inline]
    fn serialize_unit(self) -> Result<Self::Ok> {
        self.serialize_none()
    }

    #[inline]
    fn serialize_unit_struct(self, name: &'static str) -> Result<Self::Ok> {
        self.serialize_unit()
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

    fn serialize_newtype_struct<T: ?Sized>(self, _name: &'static str, value: &T) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T: ?Sized>(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok>
    where
        T: serde::Serialize,
    {
        self.update_element_type(ElementType::EmbeddedDocument)?;
        let mut d = DocumentSerializer::start(&mut *self)?;
        d.serialize_entry(variant, value)?;
        d.end_doc()?;
        Ok(())
    }

    #[inline]
    fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq> {
        self.update_element_type(ElementType::Array)?;
        DocumentSerializer::start(&mut *self)
    }

    #[inline]
    fn serialize_tuple(self, len: usize) -> Result<Self::SerializeTuple> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        self.serialize_seq(Some(len))
    }

    #[inline]
    fn serialize_tuple_variant(
        self,
        name: &'static str,
        variant_index: u32,
        variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.update_element_type(ElementType::EmbeddedDocument)?;
        VariantSerializer::start(&mut *self, variant, VariantInnerType::Tuple)
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        self.update_element_type(ElementType::EmbeddedDocument)?;
        DocumentSerializer::start(&mut *self)
    }

    fn serialize_struct(self, name: &'static str, _len: usize) -> Result<Self::SerializeStruct> {
        let element_type = match name {
            "$oid" => ElementType::ObjectId,
            "$date" => ElementType::DateTime,
            "$binary" => ElementType::Binary,
            _ => ElementType::EmbeddedDocument,
        };

        self.update_element_type(element_type)?;
        StructSerializer::new(&mut *self, element_type)
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        _variant_index: u32,
        variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        VariantSerializer::start(&mut *self, variant, VariantInnerType::Struct)
    }
}

struct DocumentSerializationResult<'a> {
    length: i32,
    root_serializer: &'a mut Serializer,
}

pub(crate) struct DocumentSerializer<'a> {
    root_serializer: &'a mut Serializer,
    num_keys_serialized: usize,
    start: usize,
}

impl<'a> DocumentSerializer<'a> {
    fn start(rs: &'a mut Serializer) -> crate::ser::Result<Self> {
        let start = rs.bytes.len();
        write_i32(&mut rs.bytes, 0)?;
        Ok(Self {
            root_serializer: rs,
            num_keys_serialized: 0,
            start,
        })
    }

    fn serialize_doc_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: serde::Serialize + ?Sized,
    {
        // push a dummy element type for now, will update this once we serialize the value
        self.root_serializer.type_index = self.root_serializer.bytes.len();
        self.root_serializer.bytes.push(0);
        key.serialize(KeySerializer {
            root_serializer: &mut *self.root_serializer,
        })?;

        self.num_keys_serialized += 1;
        Ok(())
    }

    fn end_doc(self) -> crate::ser::Result<DocumentSerializationResult<'a>> {
        self.root_serializer.bytes.push(0);
        let length = (self.root_serializer.bytes.len() - self.start) as i32;
        self.root_serializer.replace_i32(self.start, length);
        Ok(DocumentSerializationResult {
            length,
            root_serializer: self.root_serializer
        })
    }
}

impl<'a> serde::ser::SerializeSeq for DocumentSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.serialize_doc_key(&self.num_keys_serialized.to_string())?;
        value.serialize(&mut *self.root_serializer)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

impl<'a> serde::ser::SerializeMap for DocumentSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_key<T: ?Sized>(&mut self, key: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.serialize_doc_key(key)
    }

    fn serialize_value<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        value.serialize(&mut *self.root_serializer)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

impl<'a> serde::ser::SerializeStruct for DocumentSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.serialize_doc_key(key)?;
        value.serialize(&mut *self.root_serializer)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

impl<'a> serde::ser::SerializeTuple for DocumentSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_element<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.serialize_doc_key(&self.num_keys_serialized.to_string())?;
        value.serialize(&mut *self.root_serializer)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

impl<'a> serde::ser::SerializeTupleStruct for DocumentSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: serde::Serialize,
    {
        self.serialize_doc_key(&self.num_keys_serialized.to_string())?;
        value.serialize(&mut *self.root_serializer)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_doc().map(|_| ())
    }
}

pub(crate) struct BsonTypeSerializer<'a> {
    root_serializer: &'a mut Serializer,
    state: SerializationStep,
}

impl<'a> BsonTypeSerializer<'a> {
    fn new(rs: &'a mut Serializer, element_type: ElementType) -> Self {
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

            _ => todo!(),
        };
        Self {
            root_serializer: rs,
            state,
        }
    }
}

impl<'a, 'b, 'c: 'a + 'b> serde::Serializer for &'b mut BsonTypeSerializer<'a> {
    type Ok = ();
    type Error = Error;

    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = CodeWithScopeSerializer<'b>;
    type SerializeStruct = Self;
    type SerializeStructVariant = Impossible<(), Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        match self.state {
            SerializationStep::TimestampTime | SerializationStep::TimestampIncrement => {
                write_i32(&mut self.root_serializer.bytes, v)?;
            }
            _ => todo!(),
        }
        Ok(())
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        todo!()
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
        // match self.bson_type {
        //     ElementType::ObjectId => {
        //         let oid = ObjectId::parse_str(v).map_err(Error::custom)?;
        //         self.root_serializer.bytes.write_all(&oid.bytes())?;
        //     }
        //     _ => todo!(),
        // }

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
            _ => todo!(),
        }
        Ok(())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        todo!()
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

impl<'a, 'b> SerializeStruct for &'b mut BsonTypeSerializer<'a> {
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
                value.serialize(&mut **self)?;
                self.state = SerializationStep::TimestampIncrement;
            }
            (SerializationStep::TimestampIncrement, "i") => {
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
                value.serialize(&mut **self)?;
            }
            (SerializationStep::CodeWithScopeScope { .. }, "$scope") => {
                value.serialize(&mut **self)?;
                self.state = SerializationStep::Done;
            }
            (SerializationStep::MinKey { .. }, "$minKey") => {
                self.state = SerializationStep::Done;
            }
            (SerializationStep::MaxKey { .. }, "$maxKey") => {
                self.state = SerializationStep::Done;
            }
            (state, k) => panic!("bad combo: {:?} + {:?}", state, k),
        }

        Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        Ok(())
    }
}

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
    TimestampIncrement,

    DbPointer,
    DbPointerRef,
    DbPointerId,

    Code,

    CodeWithScopeCode,
    CodeWithScopeScope { code: String },

    MinKey,

    MaxKey,

    Done,
}

// pub(crate) struct StructSerializer<'a> {
//     root_serializer: &'a mut Serializer,
//     bson_type: ElementType
// }

pub(crate) enum StructSerializer<'a> {
    Value(BsonTypeSerializer<'a>),
    Document(DocumentSerializer<'a>),
}

impl<'a> StructSerializer<'a> {
    fn new(rs: &'a mut Serializer, element_type: ElementType) -> Result<Self> {
        if let ElementType::EmbeddedDocument = element_type {
            Ok(Self::Document(DocumentSerializer::start(rs)?))
        } else {
            Ok(Self::Value(BsonTypeSerializer::new(rs, element_type)))
        }
    }
}

impl<'a> SerializeStruct for StructSerializer<'a> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        // println!("got field: {}", key);
        match self {
            // StructSerializer::Value {
            //     element_type,
            //     root_serializer,
            // } => {
            //     match element_type {
            //         ElementType::ObjectId => {
            //             assert_eq!(key, "$oid");
            //         }
            //         ElementType::DateTime => {
            //             assert_eq!(key, "$date");
            //         }
            //         _ => todo!(),
            //     }
            //     let mut s = BsonTypeSerializer::new(&mut *root_serializer, *element_type);
            //     value.serialize(&mut s)
            // }
            StructSerializer::Value(ref mut v) => (&mut *v).serialize_field(key, value),
            StructSerializer::Document(d) => d.serialize_field(key, value),
        }
        // Ok(())
    }

    fn end(self) -> Result<Self::Ok> {
        match self {
            StructSerializer::Document(d) => SerializeStruct::end(d),
            _ => Ok(()),
        }
    }
}

enum VariantInnerType {
    Tuple,
    Struct,
}

pub(crate) struct VariantSerializer<'a> {
    root_serializer: &'a mut Serializer,
    doc_start: usize,
    inner_start: usize,
    num_elements_serialized: usize,
}

impl<'a> VariantSerializer<'a> {
    fn start(
        rs: &'a mut Serializer,
        variant: &'static str,
        inner_type: VariantInnerType,
    ) -> Result<Self> {
        rs.update_element_type(ElementType::EmbeddedDocument)?;
        let doc_start = rs.bytes.len();
        write_i32(&mut rs.bytes, 0)?;

        let inner = match inner_type {
            VariantInnerType::Struct => ElementType::EmbeddedDocument,
            VariantInnerType::Tuple => ElementType::Array,
        };
        rs.bytes.push(inner as u8);
        write_cstring(&mut rs.bytes, variant)?;
        let inner_start = rs.bytes.len();

        Ok(Self {
            root_serializer: rs,
            num_elements_serialized: 0,
            doc_start,
            inner_start,
        })
    }

    fn serialize_element<T>(&mut self, k: &str, v: &T) -> Result<()>
    where
        T: Serialize + ?Sized,
    {
        self.root_serializer.bytes.push(0);
        write_cstring(&mut self.root_serializer.bytes, k)?;
        v.serialize(&mut *self.root_serializer)?;

        self.num_elements_serialized += 1;
        Ok(())
    }

    fn end_both(self) -> Result<()> {
        // null byte for the inner
        self.root_serializer.bytes.push(0);
        let arr_length = (self.root_serializer.bytes.len() - self.inner_start) as i32;
        self.root_serializer.replace_i32(self.inner_start, arr_length);

        // null byte for document
        self.root_serializer.bytes.push(0);
        let doc_length = (self.root_serializer.bytes.len() - self.doc_start) as i32;
        self.root_serializer.replace_i32(self.doc_start, doc_length);
        Ok(())
    }
}

impl<'a> serde::ser::SerializeTupleVariant for VariantSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.serialize_element(format!("{}", self.num_elements_serialized).as_str(), value)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_both()
    }
}

impl<'a> serde::ser::SerializeStructVariant for VariantSerializer<'a> {
    type Ok = ();

    type Error = Error;

    fn serialize_field<T: ?Sized>(&mut self, key: &'static str, value: &T) -> Result<()>
    where
        T: Serialize,
    {
        self.serialize_element(key, value)
    }

    fn end(self) -> Result<Self::Ok> {
        self.end_both()
    }
}

struct KeySerializer<'a> {
    root_serializer: &'a mut Serializer,
}

impl<'a> serde::Serializer for KeySerializer<'a> {
    type Ok = ();

    type Error = Error;

    type SerializeSeq = Impossible<(), Error>;
    type SerializeTuple = Impossible<(), Error>;
    type SerializeTupleStruct = Impossible<(), Error>;
    type SerializeTupleVariant = Impossible<(), Error>;
    type SerializeMap = Impossible<(), Error>;
    type SerializeStruct = Impossible<(), Error>;
    type SerializeStructVariant = Impossible<(), Error>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i8(self, v: i8) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok> {
        todo!()
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok> {
        todo!()
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
        write_cstring(&mut self.root_serializer.bytes, v)
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok> {
        todo!()
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
        todo!()
    }

    fn serialize_struct(self, name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        todo!()
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

pub(crate) struct CodeWithScopeSerializer<'a> {
    code_length: usize,
    start: usize,
    doc: DocumentSerializer<'a>,
}

impl<'a> CodeWithScopeSerializer<'a> {
    fn start(code: &str, rs: &'a mut Serializer) -> Result<Self> {
        let start = rs.bytes.len();
        write_i32(&mut rs.bytes, 0)?; // placeholder length
        write_string(&mut rs.bytes, code)?;

        let doc = DocumentSerializer::start(rs)?;
        Ok(Self {
            code_length: code.len(),
            start,
            doc,
        })
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

        let total_len = result.length + self.code_length as i32;
        result.root_serializer.replace_i32(self.start, total_len);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{doc, Binary, DateTime, JavaScriptCodeWithScope};
    use serde::Serialize;

    #[test]
    fn raw_serialize() {
        let binary = Binary {
            subtype: crate::spec::BinarySubtype::BinaryOld,
            bytes: Vec::new(),
        };
        let doc = doc! {
            // "a": JavaScriptCodeWithScope {
            //     code: "".to_string(),
            //     scope: doc! {}
            // }
            "o": ObjectId::new(),
            "d": DateTime::now(),
            "b": binary,
            // "x": { "y": "ok" },
            // "a": true,
            // "b": 1i32,
            // "c": 2i64,
            // "d": 5.5,
            // "e": [ true, "aaa", { "ok": 1.0 } ]
        };
        println!("{}", doc);
        // let mut v = Vec::new();
        // doc.to_writer(&mut v).unwrap();

        let raw_v = crate::ser::to_vec(&doc).unwrap();
        // assert_eq!(raw_v, v);
        let d = Document::from_reader(raw_v.as_slice()).unwrap();
        println!("{:#?}", d);
    }
    use std::time::Instant;

    use serde::Deserialize;

    use crate::{oid::ObjectId, Document};

    #[derive(Debug, Deserialize)]
    struct D {
        x: i32,
        y: i32,
        i: I,
        // oid: ObjectId,
        null: Option<i32>,
        b: bool,
        d: f32,
    }

    #[derive(Debug, Deserialize)]
    struct I {
        a: i32,
        b: i32,
    }

    #[derive(Debug, Serialize)]
    struct Code {
        c: JavaScriptCodeWithScope,
    }

    // #[test]
    // fn raw_serialize() {
    //     let c = Code {
    //         c: JavaScriptCodeWithScope {
    //             code: "".to_string(),
    //             scope: doc! {},
    //         }
    //     };

    //     let v = crate::ser::to_vec(&c).unwrap();

    //     let doc = crate::to_document(&c).unwrap();
    //     let mut v2 = Vec::new();
    //     doc.to_writer(&mut v2).unwrap();

    //     assert_eq!(v, v2);
    // }

    #[test]
    fn raw_bench() {
        let binary = Binary {
            subtype: crate::spec::BinarySubtype::Generic,
            bytes: vec![1, 2, 3, 4, 5],
        };
        let doc = doc! {
            "ok": 1,
            "x": 1,
            "y": 2,
            "i": { "a": 300, "b": 12345 },
            // "oid": ObjectId::new(),
            "null": crate::Bson::Null,
            "b": true,
            "dt": DateTime::now(),
            "d": 12.5,
            "b": binary,
        };

        let raw_start = Instant::now();
        for _ in 0..10_000 {
            let _b = crate::ser::to_vec(&doc).unwrap();
        }
        let raw_time = raw_start.elapsed();
        println!("raw time: {}", raw_time.as_secs_f32());

        let normal_start = Instant::now();
        for _ in 0..10_000 {
            let d: Document = crate::to_document(&doc).unwrap();
            let mut v = Vec::new();
            d.to_writer(&mut v).unwrap();
        }
        let normal_time = normal_start.elapsed();
        println!("normal time: {}", normal_time.as_secs_f32());

        let normal_start = Instant::now();
        for _ in 0..10_000 {
            let mut v = Vec::new();
            doc.to_writer(&mut v).unwrap();
        }
        let normal_time = normal_start.elapsed();
        println!("decode time: {}", normal_time.as_secs_f32());
    }
}
