use crate::ser::write_i32;
use crate::ser::Result;
use crate::ser::Error;

use super::Serializer;

pub(super) struct DocumentSerializationResult<'a> {
    pub(super) length: i32,
    pub(super) root_serializer: &'a mut Serializer,
}

pub(super) struct DocumentSerializer<'a> {
    root_serializer: &'a mut Serializer,
    num_keys_serialized: usize,
    start: usize,
}

impl<'a> DocumentSerializer<'a> {
    pub(super) fn start(rs: &'a mut Serializer) -> crate::ser::Result<Self> {
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

    pub(super) fn end_doc(self) -> crate::ser::Result<DocumentSerializationResult<'a>> {
        self.root_serializer.bytes.push(0);
        let length = (self.root_serializer.bytes.len() - self.start) as i32;
        self.root_serializer.replace_i32(self.start, length);
        Ok(DocumentSerializationResult {
            length,
            root_serializer: self.root_serializer,
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
