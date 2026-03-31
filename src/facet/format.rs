use facet_format::ScalarValue;
use facet_reflect::ReflectError;

use crate::{
    error::{Error, Result},
    raw::CStr,
    spec::ElementType,
    RawBsonRef,
};

#[derive(Debug)]
struct Serializer {
    bytes: Vec<u8>,
    doc_size_pos: Vec<usize>,
    elem_kind_pos: Option<usize>,
}

impl Serializer {
    fn new() -> Self {
        Self {
            bytes: vec![],
            doc_size_pos: vec![],
            elem_kind_pos: None,
        }
    }

    fn write_bson_ref(&mut self, bv: RawBsonRef<'_>) -> Result<()> {
        let kind_pos = self
            .elem_kind_pos
            .take()
            .ok_or_else(|| Error::serialization("field value without field key"))?;
        self.bytes[kind_pos] = bv.element_type() as u8;
        bv.append_to(&mut self.bytes);
        Ok(())
    }
}

impl facet_format::FormatSerializer for Serializer {
    type Error = Error;

    fn begin_struct(&mut self) -> Result<()> {
        println!("begin_struct");
        if let Some(kind_pos) = self.elem_kind_pos.take() {
            self.bytes[kind_pos] = ElementType::EmbeddedDocument as u8;
        }
        self.doc_size_pos.push(self.bytes.len());
        self.bytes
            .extend_from_slice(crate::raw::MIN_BSON_DOCUMENT_SIZE.to_le_bytes().as_slice()); // placeholder
        Ok(())
    }

    fn end_struct(&mut self) -> Result<()> {
        println!("end_struct");
        let size_pos = self
            .doc_size_pos
            .pop()
            .ok_or_else(|| Error::serialization("mismatched begin_struct / end_struct"))?;
        self.bytes.push(0); // terminal null
        let size = (self.bytes.len() - size_pos) as i32;
        self.bytes[size_pos..size_pos + 4].copy_from_slice(&size.to_le_bytes());
        Ok(())
    }

    fn field_key(&mut self, key: &str) -> Result<()> {
        println!("field_key: {key:?}");
        if self.elem_kind_pos.is_some() {
            return Err(Error::serialization("unexpected field_key"));
        }
        self.elem_kind_pos = Some(self.bytes.len());
        self.bytes.push(0); // placeholder
        let key: &CStr = key.try_into()?;
        key.append_to(&mut self.bytes);
        Ok(())
    }

    fn scalar(&mut self, scalar: ScalarValue<'_>) -> Result<()> {
        println!("scalar: {scalar:?}");
        let bv = match scalar {
            ScalarValue::I64(i) => RawBsonRef::Int64(i),
            _ => todo!(),
        };
        self.write_bson_ref(bv)
    }

    fn serialize_opaque_scalar(
        &mut self,
        shape: &'static facet::Shape,
        value: facet_reflect::Peek<'_, '_>,
    ) -> Result<bool> {
        println!("serialize_opaque_scalar: {}", shape.type_name());
        if let Ok(v) = value.get::<i32>() {
            self.write_bson_ref(RawBsonRef::Int32(*v))?;
            return Ok(true);
        }
        Ok(false)
    }

    fn begin_seq(&mut self) -> Result<()> {
        println!("begin_seq");
        todo!()
    }

    fn end_seq(&mut self) -> Result<()> {
        println!("end_seq");
        todo!()
    }
}

impl From<ReflectError> for Error {
    fn from(value: ReflectError) -> Self {
        Error::serialization(format!("{value}"))
    }
}

#[cfg(test)]
mod test {
    use crate::Document;

    use super::*;

    use facet::Facet;

    #[test]
    fn simple_serialize() {
        #[derive(Facet, Debug)]
        struct Inner {
            value: i32,
        }

        #[derive(Facet, Debug)]
        struct Outer {
            inner: Inner,
            other: i32,
        }

        let mut s = Serializer::new();
        let v = Outer {
            inner: Inner { value: 42 },
            other: 13,
        };
        facet_format::serialize_root(&mut s, facet_reflect::Peek::new(&v)).unwrap();
        let doc = Document::from_reader(std::io::Cursor::new(s.bytes)).unwrap();
        assert_eq!(doc, doc! { "inner": { "value": 42 }, "other": 13 });
    }
}
