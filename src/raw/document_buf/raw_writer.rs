use crate::{
    raw::{write_cstring, write_string},
    RawBsonRef,
};

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

        match value {
            RawBsonRef::String(s) => {
                write_string(self.data, s);
            }
            RawBsonRef::Document(d) => {
                self.data.extend(d.as_bytes());
            }
            RawBsonRef::Array(a) => {
                self.data.extend(a.as_bytes());
            }
            RawBsonRef::Boolean(b) => {
                self.data.push(b as u8);
            }
            RawBsonRef::DateTime(dt) => {
                self.data.extend(dt.timestamp_millis().to_le_bytes());
            }
            RawBsonRef::DbPointer(dbp) => {
                write_string(self.data, dbp.namespace);
                self.data.extend(dbp.id.bytes());
            }
            RawBsonRef::Decimal128(d) => {
                self.data.extend(d.bytes());
            }
            RawBsonRef::RegularExpression(re) => {
                write_cstring(self.data, re.pattern)?;
                write_cstring(self.data, re.options)?;
            }
            RawBsonRef::JavaScriptCode(js) => {
                write_string(self.data, js);
            }
            RawBsonRef::JavaScriptCodeWithScope(code_w_scope) => {
                let len = code_w_scope.len();
                self.data.extend(len.to_le_bytes());
                write_string(self.data, code_w_scope.code);
                self.data.extend(code_w_scope.scope.as_bytes());
            }
            RawBsonRef::Timestamp(ts) => {
                self.data.extend(ts.to_le_bytes());
            }
            RawBsonRef::ObjectId(oid) => {
                self.data.extend(oid.bytes());
            }
            RawBsonRef::Symbol(s) => {
                write_string(self.data, s);
            }
            RawBsonRef::Null | RawBsonRef::Undefined | RawBsonRef::MinKey | RawBsonRef::MaxKey => {}
            value => value.append_to(self.data)?,
        }

        // append trailing null byte
        self.data.push(0);
        // update length
        let new_len = (self.data.len() as i32).to_le_bytes();
        self.data[0..4].copy_from_slice(&new_len);

        Ok(())
    }
}
