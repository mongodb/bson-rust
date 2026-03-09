//! Contains the error-related types for the `bson` crate.

mod decimal128;
mod oid;
mod uuid;
mod value_access;

use thiserror::Error;

pub use decimal128::Decimal128ErrorKind;
pub use oid::ObjectIdErrorKind;
pub use uuid::UuidErrorKind;
pub use value_access::ValueAccessErrorKind;

/// The result type for all methods that can return an error in the `bson` crate.
pub type Result<T> = std::result::Result<T, Error>;

/// An error that can occur in the `bson` crate.
#[derive(Clone, Debug, Error)]
#[non_exhaustive]
pub struct Error {
    /// The kind of error that occurred.
    pub kind: ErrorKind,

    /// An optional message describing the error.
    pub message: Option<String>,

    /// The document key associated with the error, if any.
    pub key: Option<String>,

    /// The array index associated with the error, if any.
    pub index: Option<usize>,

    /// The path to a deserialization error, if any.
    #[cfg(feature = "serde_path_to_error")]
    pub path: Option<serde_path_to_error::Path>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BSON error")?;

        if let Some(key) = self.key.as_deref() {
            write!(f, " at key \"{key}\"")?;
        } else if let Some(index) = self.index {
            write!(f, " at array index {index}")?;
        }

        write!(f, ". Kind: {}", self.kind)?;
        if let Some(ref message) = self.message {
            write!(f, ". Message: {}", message)?;
        }
        #[cfg(feature = "serde_path_to_error")]
        if let Some(ref path) = self.path {
            write!(f, ". Path: {}", path)?;
        }

        write!(f, ".")
    }
}

/// The types of errors that can occur in the `bson` crate.
#[derive(Clone, Debug, Error)]
#[non_exhaustive]
pub enum ErrorKind {
    /// An error related to the [`Binary`](crate::Binary) type occurred.
    #[error("A Binary-related error occurred")]
    #[non_exhaustive]
    Binary {},

    /// An error related to the [`DateTime`](crate::DateTime) type occurred.
    #[error("A DateTime-related error occurred")]
    #[non_exhaustive]
    DateTime {},

    /// An error related to the [`Decimal128`](crate::Decimal128) type occurred.
    #[error("A Decimal128-related error occurred: {kind}")]
    #[non_exhaustive]
    Decimal128 {
        /// The kind of error that occurred.
        kind: Decimal128ErrorKind,
    },

    /// A general error occurred during deserialization. This variant is constructed in the
    /// [`serde::de::Error`] implementation for the [`Error`](struct@Error) type.
    #[cfg(feature = "serde")]
    #[error("A deserialization-related error occurred")]
    #[non_exhaustive]
    Deserialization {},

    /// The end of the BSON input was reached too soon.
    #[error("End of stream")]
    #[non_exhaustive]
    EndOfStream {},

    /// Malformed BSON bytes were encountered.
    #[error("Malformed BSON bytes")]
    #[non_exhaustive]
    MalformedBytes {},

    /// An error related to the [`ObjectId`](crate::oid::ObjectId) type occurred.
    #[error("An ObjectId-related error occurred: {kind}")]
    #[non_exhaustive]
    ObjectId {
        /// The kind of error that occurred.
        kind: ObjectIdErrorKind,
    },

    /// A general error occurred during serialization. This variant is constructed in the
    /// [`serde::ser::Error`] implementation for the [`Error`](struct@Error) type.
    #[cfg(feature = "serde")]
    #[error("A serialization error occurred")]
    #[non_exhaustive]
    Serialization {},

    /// An unsigned integer could not fit into a BSON integer type.
    #[error("Unsigned integer {n} cannot fit into BSON")]
    #[non_exhaustive]
    TooLargeUnsignedInteger {
        /// The too-large unsigned integer.
        n: u64,
    },

    /// A cstring exceeded the maximum parsing length.
    #[cfg(feature = "sfp-internal")]
    #[error("cstring exceeded the maximum parsing length ({max_parse_len} bytes)")]
    #[non_exhaustive]
    #[doc(hidden)]
    TooLongCStr {
        /// The configured maximum parsing length.
        max_parse_len: usize,

        /// The bytes parsed before the maximum parsing length was reached.
        bytes: Vec<u8>,
    },

    /// Invalid UTF-8 bytes were encountered.
    #[error("Invalid UTF-8")]
    #[non_exhaustive]
    Utf8Encoding {},

    /// An error related to the [`Uuid`](crate::uuid::Uuid) type occurred.
    #[error("A UUID-related error occurred: {kind}")]
    #[non_exhaustive]
    Uuid {
        /// The kind of error that occurred.
        kind: UuidErrorKind,
    },

    /// An error occurred when attempting to access a value in a document.
    #[error("An error occurred when attempting to access a document value: {kind}")]
    #[non_exhaustive]
    ValueAccess {
        /// The kind of error that occurred.
        kind: ValueAccessErrorKind,
    },

    /// An IO error occurred.
    #[error("An IO error occurred")]
    #[non_exhaustive]
    Io {},
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self {
            kind,
            key: None,
            index: None,
            message: None,
            #[cfg(feature = "serde_path_to_error")]
            path: None,
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Error::from(ErrorKind::Io {}).with_message(value)
    }
}

#[cfg(feature = "serde")]
impl serde::de::Error for Error {
    fn custom<T>(message: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::deserialization(message)
    }
}

#[cfg(feature = "serde")]
impl serde::ser::Error for Error {
    fn custom<T>(message: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::serialization(message)
    }
}

impl Error {
    pub(crate) fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub(crate) fn with_index(mut self, index: usize) -> Self {
        self.index = Some(index);
        self
    }

    pub(crate) fn with_message(mut self, message: impl ToString) -> Self {
        self.message = Some(message.to_string());
        self
    }

    #[cfg(feature = "serde_path_to_error")]
    pub(crate) fn with_path(error: serde_path_to_error::Error<Self>) -> Self {
        let path = error.path().clone();
        let mut error = error.into_inner();
        error.path = Some(path);
        error
    }

    pub(crate) fn binary(message: impl ToString) -> Self {
        Self::from(ErrorKind::Binary {}).with_message(message)
    }

    pub(crate) fn datetime(message: impl ToString) -> Self {
        Self::from(ErrorKind::DateTime {}).with_message(message)
    }

    #[cfg(feature = "serde")]
    pub(crate) fn serialization(message: impl ToString) -> Self {
        Self::from(ErrorKind::Serialization {}).with_message(message)
    }

    #[cfg(feature = "serde")]
    pub(crate) fn invalid_key_type(key: impl AsRef<str>) -> Self {
        Self::serialization(format!("invalid document key type: {}", key.as_ref()))
    }

    #[cfg(feature = "serde")]
    pub(crate) fn deserialization(message: impl ToString) -> Self {
        Self::from(ErrorKind::Deserialization {}).with_message(message)
    }

    #[cfg(feature = "serde")]
    pub(crate) fn end_of_stream() -> Self {
        ErrorKind::EndOfStream {}.into()
    }

    pub(crate) fn malformed_bytes(message: impl ToString) -> Self {
        Self::from(ErrorKind::MalformedBytes {}).with_message(message)
    }

    #[cfg(all(test, feature = "serde"))]
    pub(crate) fn is_malformed_bytes(&self) -> bool {
        matches!(self.kind, ErrorKind::MalformedBytes { .. },)
    }

    #[cfg(feature = "serde")]
    pub(crate) fn too_large_integer(n: u64) -> Self {
        Self::from(ErrorKind::TooLargeUnsignedInteger { n })
    }
}
