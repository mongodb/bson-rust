use crate::{raw::CStr, spec::ElementType};

pub(crate) struct DocWriter<'a> {
    data: &'a mut Vec<u8>,
    start: usize,
}

impl<'a> DocWriter<'a> {
    pub(crate) fn open(data: &'a mut Vec<u8>) -> Self {
        let start = data.len();
        data.extend(crate::raw::MIN_BSON_DOCUMENT_SIZE.to_le_bytes());
        Self { data, start }
    }

    pub(crate) fn resume(data: &'a mut Vec<u8>, start: usize) -> Self {
        Self { data, start }
    }

    pub(crate) fn append_key(&mut self, element_type: ElementType, name: &CStr) {
        self.data.push(element_type as u8);
        name.append_to(self.data);
    }

    pub(crate) fn buffer(&mut self) -> &mut Vec<u8> {
        self.data
    }
}

impl<'a> Drop for DocWriter<'a> {
    fn drop(&mut self) {
        self.data.push(0);
        let new_len = ((self.data.len() - self.start) as i32).to_le_bytes();
        self.data[self.start..self.start + 4].copy_from_slice(&new_len);
    }
}
