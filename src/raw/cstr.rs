use core::str;

use crate::error::{Error, Result};

#[allow(rustdoc::invalid_rust_codeblocks)]
/// A borrowed BSON-spec cstring: Zero or more UTF-8 encoded characters, excluding the nul byte.
/// Can be constructed at compile-time via the [`cstr!`](crate::raw::cstr) macro or at run-time from
/// a [`prim@str`] via [`TryFrom`].
///
/// Unlike [`std::ffi::CStr`], this is required to be valid UTF-8, and does not include the nul
/// terminator in the buffer:
/// ```
/// // std::ffi::CStr accepts invalid UTF-8:
/// let invalid: &std::ffi::CStr = c"\xc3\x28";
/// ```
/// ```compile_fail
/// # use bson::raw::cstr;
/// // bson::raw::CStr does not:
/// let invalid: &bson::raw::CStr = cstr!("\xc3\x28");  // will not compile
/// ```
/// ```
/// // &str accepts embedded nil characters:
/// let invalid: &str = "foo\0bar";
/// ```
/// ```compile_fail
/// # use bson::raw::cstr;
/// // bson::raw::CStr does not:
/// let invalid: &bson::raw::CStr = cstr!("foo\0bar");  // will not compile
/// ```
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
    // Convenience shorthand for making the types of TryFrom line up
    #[cfg(feature = "serde")]
    pub(crate) fn from_str(value: &str) -> Result<&CStr> {
        value.try_into()
    }

    const fn from_str_unchecked(value: &str) -> &Self {
        // Safety: the conversion is safe because CStr is repr(transparent), and the deref is safe
        // because the pointer came from a safe reference.
        unsafe { &*(value.as_bytes() as *const [u8] as *const CStr) }
    }

    /// View the buffer as a Rust `&str`.
    pub fn as_str(&self) -> &str {
        // Safety: the only way to constrct a CStr is from a valid &str.
        unsafe { str::from_utf8_unchecked(&self.data) }
    }

    /// The length in bytes of the buffer.
    pub fn len(&self) -> usize {
        self.as_str().len()
    }

    /// Whether the buffer contains zero bytes.
    pub fn is_empty(&self) -> bool {
        self.as_str().is_empty()
    }

    pub(crate) fn append_to(&self, buf: &mut Vec<u8>) {
        buf.extend(&self.data);
        buf.push(0);
    }
}

impl PartialEq<&CStr> for &CStr {
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

impl AsRef<CStr> for CStr {
    fn as_ref(&self) -> &CStr {
        self
    }
}

impl AsRef<str> for CStr {
    fn as_ref(&self) -> &str {
        self.as_str()
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

#[doc(hidden)]
#[diagnostic::on_unimplemented(message = "the string literal contains a zero byte")]
pub trait ValidCStr {}
#[doc(hidden)]
pub struct IsValidCStr<const VALID: bool>;
#[doc(hidden)]
impl ValidCStr for IsValidCStr<true> {}

#[doc(hidden)]
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
#[doc(hidden)]
pub const fn assert_valid_cstr<T: ValidCStr>() {}

#[allow(rustdoc::invalid_rust_codeblocks)]
/// Construct a `'static &CStr`.  The validitiy will be verified at compile-time.
/// ```
/// # use bson::raw::{CStr, cstr};
/// // A valid literal:
/// let key: &CStr = cstr!("hello");
/// ```
/// ```compile_fail
/// # use bson::raw::{CStr, cstr};
/// // A literal with invalid UTF-8 will not compile:
/// let key: &CStr = cstr!("\xc3\x28");
/// ```
/// ```compile_fail
/// # use bson::raw::{CStr, cstr};
/// // A literal with an embedded nil will not compile:
/// let key: &CStr = cstr!("hel\0lo");
/// ```
#[macro_export]
macro_rules! cstr {
    ($text:literal) => {{
        const VALIDATED: Option<&$crate::raw::CStr> = $crate::raw::validate_cstr($text);
        const VALID: bool = VALIDATED.is_some();
        $crate::raw::assert_valid_cstr::<$crate::raw::IsValidCStr<VALID>>();
        VALIDATED.unwrap()
    }};
}
pub use cstr;

/// An owned BSON-spec cstring: Zero or more UTF-8 encoded characters, excluding the nul byte.
/// `CString` is to `CStr` as [`String`] is to [`prim@str`].  Can be constructed from a [`CStr`] via
/// [`ToOwned`]/[`Into`] or from a [`String`] or [`prim@str`] via [`TryFrom`].
///
/// Like `CStr`, this differs from [`std::ffi::CString`] in that it is required to be valid UTF-8,
/// and does not include the nul terminator in the buffer.
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
    pub(crate) fn from_string_unchecked(data: String) -> Self {
        Self { data }
    }

    /// Consume `self` to return the underlying `String`.
    pub fn into_string(self) -> String {
        self.data
    }

    /// View the buffer as a Rust `&str`.
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

#[cfg(feature = "serde")]
impl serde::Serialize for CString {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.data.serialize(serializer)
    }
}
