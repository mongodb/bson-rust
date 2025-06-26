use crate::{raw::write_cstring, RawBsonRef};

pub(super) struct RawWriter<'a> {
    data: &'a mut Vec<u8>,
}

impl<'a> RawWriter<'a> {
    pub(super) fn new(data: &'a mut Vec<u8>) -> Self {
        Self { data }
    }

    pub(super) fn append(&mut self, key: &str, value: RawBsonRef) -> crate::error::Result<()> {
        let original_len = self.data.len();
        self.data[original_len - 1] = value.element_type() as u8;

        write_cstring(self.data, key)?;
        value.append_to(self.data)?;

        // append trailing null byte
        self.data.push(0);
        // update length
        let new_len = (self.data.len() as i32).to_le_bytes();
        self.data[0..4].copy_from_slice(&new_len);

        Ok(())
    }
}
