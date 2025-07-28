use std::ops::{Deref, DerefMut};

// One could imagine passthrough Borrow impls; however, it turns out that can't be made to work
// because of the existing base library impl of Borrow<T> for T will conflict despite that not
// actually being possible to construct (https://github.com/rust-lang/rust/issues/50237).  So,
// sadly, Borrow impls for HumanReadable are deliberately omitted :(

/// Wrapper type for lossily decoding embedded strings with invalid UTF-8 sequences.
///
/// A [`RawDocument`](crate::RawDocument) or [`RawDocumentBuf`](crate::RawDocumentBuf) can be
/// converted into a `Utf8Lossy<Document>` via `TryFrom`; any invalid UTF-8 sequences contained in
/// strings in the source buffer will be replaced with the Unicode replacement character.
///
/// If the `serde` feature is enabled, this type will also cause the same lossy decoding to apply
/// to any strings contained in a wrapped deserializable type when deserializing from BSON bytes.
/// This wrapper has no effect on serialization behavior.
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Hash, Default)]
#[repr(transparent)]
pub struct Utf8Lossy<T>(pub T);

impl<T: std::fmt::Display> std::fmt::Display for Utf8Lossy<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<T> From<T> for Utf8Lossy<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for Utf8Lossy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for Utf8Lossy<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T, R> AsRef<R> for Utf8Lossy<T>
where
    R: ?Sized,
    <Utf8Lossy<T> as Deref>::Target: AsRef<R>,
{
    fn as_ref(&self) -> &R {
        self.deref().as_ref()
    }
}

impl<T, R: ?Sized> AsMut<R> for Utf8Lossy<T>
where
    <Utf8Lossy<T> as Deref>::Target: AsMut<R>,
{
    fn as_mut(&mut self) -> &mut R {
        self.deref_mut().as_mut()
    }
}

#[cfg(feature = "serde")]
pub(crate) const UTF8_LOSSY_NEWTYPE: &str = "$__bson_private_utf8_lossy";

#[cfg(feature = "serde")]
impl<T: serde::Serialize> serde::Serialize for Utf8Lossy<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Utf8Lossy<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct V<T>(std::marker::PhantomData<fn() -> T>);
        impl<'de, T: serde::Deserialize<'de>> serde::de::Visitor<'de> for V<T> {
            type Value = Utf8Lossy<T>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("Utf8Lossy wrapper")
            }
            fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                T::deserialize(deserializer).map(Utf8Lossy)
            }
        }
        deserializer.deserialize_newtype_struct(UTF8_LOSSY_NEWTYPE, V(std::marker::PhantomData))
    }
}
