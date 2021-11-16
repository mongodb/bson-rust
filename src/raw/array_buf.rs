use std::{
    borrow::{Borrow, Cow},
    fmt::Debug,
    iter::FromIterator,
};

use crate::{raw::Result, RawArray, RawBson, RawDocumentBuf};

use super::RawArrayIter;

#[derive(Clone)]
pub struct RawArrayBuf {
    inner: RawDocumentBuf,
    len: usize,
}

impl RawArrayBuf {
    /// Construct a new, empty `RawArrayBuf`.
    pub fn new() -> RawArrayBuf {
        Self {
            inner: RawDocumentBuf::empty(),
            len: 0,
        }
    }

    pub fn from_vec(bytes: Vec<u8>) -> Result<Self> {
        let doc = RawDocumentBuf::new(bytes)?;
        let len = doc.iter().count();

        Ok(Self { inner: doc, len })
    }

    pub fn append<'a>(&mut self, value: impl Into<RawBson<'a>>) {
        self.inner.append(self.len.to_string(), value);
        self.len += 1;
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.inner.as_bytes()
    }
}

impl Debug for RawArrayBuf {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawArrayBuf")
            .field("data", &hex::encode(self.as_bytes()))
            .field("len", &self.len)
            .finish()
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

impl<'a> IntoIterator for &'a RawArrayBuf {
    type IntoIter = RawArrayIter<'a>;
    type Item = super::Result<RawBson<'a>>;

    fn into_iter(self) -> RawArrayIter<'a> {
        self.deref().into_iter()
    }
}

impl<'a> From<RawArrayBuf> for Cow<'a, RawArray> {
    fn from(rd: RawArrayBuf) -> Self {
        Cow::Owned(rd)
    }
}

impl<'a> From<&'a RawArrayBuf> for Cow<'a, RawArray> {
    fn from(rd: &'a RawArrayBuf) -> Self {
        Cow::Borrowed(rd.as_ref())
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
