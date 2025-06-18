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

    /// An optional message describing the error.
    pub message: Option<String>,

    /// The document key associated with the error, if any.
    pub key: Option<String>,

    /// The array index associated with the error, if any.
    pub index: Option<usize>,
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
            message: None,
        }
    }
}

impl From<crate::de::Error> for Error {
    fn from(value: crate::de::Error) -> Self {
        Self {
            kind: ErrorKind::DeError(value),
            key: None,
            index: None,
            message: None,
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

    pub(crate) fn with_message(mut self, message: impl ToString) -> Self {
        self.message = Some(message.to_string());
        self
    }

    pub(crate) fn binary(message: impl ToString) -> Self {
        Self::from(ErrorKind::Binary {}).with_message(message)
    }

    pub(crate) fn datetime(message: impl ToString) -> Self {
        Self::from(ErrorKind::DateTime {}).with_message(message)
    }

    pub(crate) fn malformed_bytes(message: impl ToString) -> Self {
        Self::from(ErrorKind::MalformedBytes {}).with_message(message)
    }
}
