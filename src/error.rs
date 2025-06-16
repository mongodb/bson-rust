mod decimal128;
mod oid;
mod uuid;
mod value_access;

use thiserror::Error;

pub use decimal128::Decimal128ErrorKind;
pub use oid::ObjectIdErrorKind;
pub use uuid::UuidErrorKind;
pub use value_access::ValueAccessErrorKind;

pub type Result<T> = std::result::Result<T, Error>;

/// An error that can occur in the `bson` crate.
#[derive(Clone, Debug, Error)]
#[non_exhaustive]
pub struct Error {
    /// The kind of error that occurred.
    pub kind: ErrorKind,

    /// The document key associated with the error, if any.
    pub key: Option<String>,

    /// The array index associated with the error, if any.
    pub index: Option<usize>,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(key) = self.key.as_deref() {
            write!(f, "Error at key \"{key}\": ")?;
        } else if let Some(index) = self.index {
            write!(f, "Error at array index {index}: ")?;
        }

        write!(f, "{}", self.kind)
    }
}

/// The types of errors that can occur in the `bson` crate.
#[derive(Clone, Debug, Error)]
#[non_exhaustive]
pub enum ErrorKind {
    /// An error related to the [`Binary`](crate::Binary) type occurred.
    #[error("A Binary-related error occurred: {message}")]
    Binary {
        /// A message describing the error.
        message: String,
    },

    /// An error related to the [`DateTime`](crate::DateTime) type occurred.
    #[error("A DateTime-related error occurred: {message}")]
    DateTime {
        /// A message describing the error.
        message: String,
    },

    /// An error related to the [`Decimal128`](crate::Decimal128) type occurred.
    #[error("A Decimal128-related error occurred: {kind}")]
    Decimal128 {
        /// The kind of error that occurred.
        kind: Decimal128ErrorKind,
    },

    /// Malformed BSON bytes were encountered.
    #[error("Malformed BSON bytes: {message}")]
    #[non_exhaustive]
    MalformedBytes {
        /// A message describing the error.
        message: String,
    },

    /// An error related to the [`ObjectId`](crate::oid::ObjectId) type occurred.
    #[error("An ObjectId-related error occurred: {kind}")]
    ObjectId {
        /// The kind of error that occurred.
        kind: ObjectIdErrorKind,
    },

    /// Invalid UTF-8 bytes were encountered.
    #[error("Invalid UTF-8")]
    Utf8Encoding,

    /// An error related to the [`Uuid`](crate::uuid::Uuid) type occurred.
    #[error("A UUID-related error occurred: {kind}")]
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

    /// A wrapped deserialization error.
    /// TODO RUST-1406: collapse this
    #[error("Deserialization error")]
    DeError(crate::de::Error),
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self {
            kind,
            key: None,
            index: None,
        }
    }
}

impl From<crate::de::Error> for Error {
    fn from(value: crate::de::Error) -> Self {
        Self {
            kind: ErrorKind::DeError(value),
            key: None,
            index: None,
        }
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

    pub(crate) fn binary(message: impl ToString) -> Self {
        ErrorKind::Binary {
            message: message.to_string(),
        }
        .into()
    }

    pub(crate) fn datetime(message: impl ToString) -> Self {
        ErrorKind::DateTime {
            message: message.to_string(),
        }
        .into()
    }

    pub(crate) fn malformed_bytes(message: impl ToString) -> Self {
        ErrorKind::MalformedBytes {
            message: message.to_string(),
        }
        .into()
    }
}
