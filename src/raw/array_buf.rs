use std::{borrow::Borrow, iter::FromIterator};

use crate::{RawArray, RawBson, RawDocumentBuf};

#[derive(Clone)]
pub struct RawArrayBuf {
    inner: RawDocumentBuf,
    len: usize,
}

impl RawArrayBuf {
    pub fn new() -> RawArrayBuf {
        Self {
            inner: RawDocumentBuf::empty(),
            len: 0,
        }
    }

    pub fn append<'a>(&mut self, value: impl Into<RawBson<'a>>) {
        self.inner.append(self.len.to_string(), value);
        self.len += 1;
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }
}

impl std::ops::Deref for RawArrayBuf {
    type Target = RawArray;

    fn deref(&self) -> &Self::Target {
        RawArray::from_doc(&self.inner)
    }
}

impl AsRef<RawArray> for RawArrayBuf {
    fn as_ref(&self) -> &RawArray {
        RawArray::from_doc(&self.inner)
    }
}

impl Borrow<RawArray> for RawArrayBuf {
    fn borrow(&self) -> &RawArray {
        self.as_ref()
    }
}

impl<'a, T: Into<RawBson<'a>>> FromIterator<T> for RawArrayBuf {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        let mut array_buf = RawArrayBuf::new();
        for item in iter {
            array_buf.append(item);
        }
        array_buf
    }
}
