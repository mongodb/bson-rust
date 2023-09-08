use crate::{raw::serde::CowStr, spec::ElementType, Document, RawDocumentBuf};
use serde::{
    de::{DeserializeSeed, Error, MapAccess, Visitor},
    Deserializer,
};
use std::{borrow::Cow, fmt::Formatter};

struct ExtendDocument<'a> {
    buffer: &'a mut Vec<u8>,
    embedded: bool,
    next_key: CowStr<'a>,
}

impl<'a> ExtendDocument<'a> {
    #[cfg(test)]
    fn new(buffer: &'a mut Vec<u8>) -> Self {
        Self {
            buffer,
            embedded: false,
            // Temporary value; will not be used.
            next_key: CowStr(Cow::Borrowed("")),
        }
    }

    fn append_element_type(&mut self, element_type: ElementType) {
        self.buffer.push(element_type as u8);
    }

    fn append_key(&mut self) {
        let key = self.next_key.0.as_ref();
        if key.contains('\0') {
            panic!("key includes interior null byte: {}", key);
        }
        self.buffer.extend_from_slice(key.as_bytes());
        self.buffer.push(0);
    }

    fn append_string(&mut self, s: &str) {
        let bytes = s.as_bytes();
        // Add 1 to account for null byte.
        let length = ((bytes.len() + 1) as i32).to_le_bytes();
        self.buffer.extend_from_slice(&length);
        self.buffer.extend_from_slice(&bytes);
        self.buffer.push(0);
    }

    fn finish_document(&mut self, length_index: usize) {
        self.buffer.push(0);
        let document_length = ((self.buffer.len() - length_index) as i32).to_le_bytes();
        self.buffer[length_index..length_index + 4].copy_from_slice(&document_length);
    }
}

impl<'a, 'de: 'a> DeserializeSeed<'de> for ExtendDocument<'a> {
    type Value = Self;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        if !self.embedded {
            // The top-level value provided to the deserializer must be a map.
            deserializer.deserialize_map(ExtendDocumentVisitor { state: Some(self) })
        } else {
            // If the value is embedded, it can be of any type.
            deserializer.deserialize_any(ExtendDocumentVisitor { state: Some(self) })
        }
    }
}

struct ExtendDocumentVisitor<'a> {
    state: Option<ExtendDocument<'a>>,
}

impl<'a, 'de: 'a> Visitor<'de> for ExtendDocumentVisitor<'a> {
    type Value = ExtendDocument<'a>;

    fn expecting(&self, formatter: &mut Formatter) -> std::fmt::Result {
        formatter.write_str("map or string for now")
    }

    fn visit_map<A>(mut self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let state = self.state.as_mut().unwrap();

        if state.embedded {
            state.append_element_type(ElementType::EmbeddedDocument);
            state.append_key();
        } else {
            state.embedded = true;
        }

        let length_index = state.buffer.len();
        // Add padding for size of document. This will be overwritten once the document is appended
        // to the buffer.
        state.buffer.extend_from_slice(&[0; 4]);

        while let Some(key) = map.next_key::<CowStr>()? {
            let mut state = self.state.take().unwrap();
            state.next_key = key;
            let state = map.next_value_seed(state)?;
            self.state = Some(state);
        }

        let state = self.state.as_mut().unwrap();
        state.finish_document(length_index);

        Ok(self.state.unwrap())
    }

    fn visit_str<E>(mut self, s: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let state = self.state.as_mut().unwrap();

        state.append_element_type(ElementType::String);
        state.append_key();
        state.append_string(s);

        Ok(self.state.unwrap())
    }

    fn visit_string<E>(mut self, s: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let state = self.state.as_mut().unwrap();

        state.append_element_type(ElementType::String);
        state.append_key();
        state.append_string(&s);

        Ok(self.state.unwrap())
    }

    fn visit_borrowed_str<E>(mut self, s: &'de str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        let state = self.state.as_mut().unwrap();

        state.append_element_type(ElementType::String);
        state.append_key();
        state.append_string(s);

        Ok(self.state.unwrap())
    }
}

#[cfg(test)]
fn json_to_doc(json: &str, expected: Option<Document>) -> bool {
    let mut buffer = Vec::new();
    let extend_document = ExtendDocument::new(&mut buffer);

    match extend_document.deserialize(&mut serde_json::Deserializer::from_str(&json)) {
        Ok(_) => {
            let raw_document = RawDocumentBuf::from_bytes(buffer).unwrap();
            let document = raw_document.to_document().unwrap();
            Some(document) == expected
        }
        Err(_) => expected.is_none(),
    }
}

#[test]
fn basic_json() {
    assert!(json_to_doc("{\"a\": \"B\"}", Some(doc! { "a": "B" })));
    assert!(json_to_doc("{\"a\"}", None));
    assert!(json_to_doc(
        "{\"a\":{\"b\":\"c\"}}",
        Some(doc! { "a": { "b": "c" } })
    ));
    assert!(json_to_doc("a", None));
}
