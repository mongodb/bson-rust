use core::str;

use crate::error::{Error, Result};

// A BSON-spec cstring: Zero or more UTF-8 encoded characters, excluding the null byte.
#[derive(Debug)]
#[repr(transparent)]
pub struct CStr {
    data: [u8],
}

impl<'a> TryFrom<&'a str> for &'a CStr {
    type Error = Error;

    fn try_from(value: &str) -> Result<&CStr> {
        match validate_cstr(value) {
            Some(cs) => Ok(cs),
            None => Err(Error::malformed_bytes(format!(
                "cstring with interior null: {:?}",
                value,
            ))),
        }
    }
}

impl CStr {
    const fn from_str_unchecked(value: &str) -> &Self {
        // Safety: the conversion is safe because CStr is repr(transparent), and the deref is safe
        // because the pointer came from a safe reference.
        unsafe { &*(value.as_bytes() as *const [u8] as *const CStr) }
    }

    pub fn as_str(&self) -> &str {
        // Safety: the only way to constrct a CStr is from a valid &str.
        unsafe { str::from_utf8_unchecked(&self.data) }
    }

    pub fn len(&self) -> usize {
        self.as_str().len()
    }

    pub(crate) fn append_to(&self, buf: &mut Vec<u8>) {
        buf.extend(&self.data);
        buf.push(0);
    }
}

impl<'a, 'b> PartialEq<&'b CStr> for &'a CStr {
    fn eq(&self, other: &&CStr) -> bool {
        self.as_str() == other.as_str()
    }
}

impl std::borrow::ToOwned for CStr {
    type Owned = CString;

    fn to_owned(&self) -> Self::Owned {
        self.into()
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for &CStr {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_str().serialize(serializer)
    }
}

#[diagnostic::on_unimplemented(message = "the string literal contains a zero byte")]
pub trait ValidCStr {}
pub struct IsValidCStr<const VALID: bool>;
impl ValidCStr for IsValidCStr<true> {}

pub const fn validate_cstr(text: &str) -> Option<&CStr> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0 {
            return None;
        }
        i += 1;
    }
    Some(CStr::from_str_unchecked(text))
}
pub const fn assert_valid_cstr<T: ValidCStr>() {}

#[macro_export]
macro_rules! cstr {
    ($text:expr) => {{
        const VALIDATED: Option<&$crate::raw::CStr> = $crate::raw::validate_cstr($text);
        const VALID: bool = VALIDATED.is_some();
        $crate::raw::assert_valid_cstr::<$crate::raw::IsValidCStr<VALID>>();
        VALIDATED.unwrap()
    }};
}
pub use cstr;

#[derive(Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct CString {
    data: String,
}

impl TryFrom<String> for CString {
    type Error = Error;

    fn try_from(data: String) -> Result<Self> {
        let _: &CStr = data.as_str().try_into()?;
        Ok(Self { data })
    }
}

impl TryFrom<&str> for CString {
    type Error = Error;

    fn try_from(data: &str) -> Result<Self> {
        let cs: &CStr = data.try_into()?;
        Ok(cs.into())
    }
}

impl CString {
    pub(crate) fn from_unchecked(data: String) -> Self {
        Self { data }
    }

    pub fn into_string(self) -> String {
        self.data
    }

    pub fn as_str(&self) -> &str {
        self.as_ref().as_str()
    }
}

impl From<&CStr> for CString {
    fn from(value: &CStr) -> Self {
        Self {
            data: value.as_str().into(),
        }
    }
}

impl AsRef<CStr> for CString {
    fn as_ref(&self) -> &CStr {
        CStr::from_str_unchecked(self.data.as_str())
    }
}

impl std::fmt::Debug for CString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.data.fmt(f)
    }
}

impl std::fmt::Display for CString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.data.fmt(f)
    }
}

impl std::borrow::Borrow<CStr> for CString {
    fn borrow(&self) -> &CStr {
        self.as_ref()
    }
}
