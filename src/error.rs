use thiserror::Error;

use crate::spec::ElementType;

pub type Result<T> = std::result::Result<T, Error>;

/// An error that can occur in the `bson` crate.
#[derive(Debug, Error)]
#[error("Kind: {kind}")]
#[non_exhaustive]
pub struct Error {
    /// The kind of error that occurred.
    pub kind: ErrorKind,
}

/// The types of errors that can occur in the `bson` crate.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ErrorKind {
    /// An error occurred when attempting to access a value in a document.
    #[error("An error occurred when attempting to access a document value for key {key}: {kind}")]
    #[non_exhaustive]
    ValueAccess {
        /// The key of the value.
        key: String,

        /// The kind of error that occurred.
        kind: ValueAccessErrorKind,
    },
}

/// The types of errors that can occur when attempting to access a value in a document.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ValueAccessErrorKind {
    /// No value for the specified key was present in the document.
    #[error("The key was not present in the document")]
    NotPresent,

    /// The type of the value in the document did not match the requested type.
    #[error("Expected type {expected:?}, got type {actual:?}")]
    #[non_exhaustive]
    UnexpectedType {
        /// The actual type of the value.
        actual: ElementType,

        /// The expected type of the value.
        expected: ElementType,
    },

    /// An error occurred when attempting to parse the document's BSON bytes.
    #[error("{message}")]
    #[non_exhaustive]
    InvalidBson { message: String },
}

impl Error {
    pub(crate) fn value_access_not_present(key: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::ValueAccess {
                key: key.into(),
                kind: ValueAccessErrorKind::NotPresent,
            },
        }
    }

    pub(crate) fn value_access_unexpected_type(
        key: impl Into<String>,
        actual: ElementType,
        expected: ElementType,
    ) -> Self {
        Self {
            kind: ErrorKind::ValueAccess {
                key: key.into(),
                kind: ValueAccessErrorKind::UnexpectedType { actual, expected },
            },
        }
    }

    pub(crate) fn value_access_invalid_bson(key: impl Into<String>, message: String) -> Self {
        Self {
            kind: ErrorKind::ValueAccess {
                key: key.into(),
                kind: ValueAccessErrorKind::InvalidBson { message },
            },
        }
    }

    #[cfg(test)]
    pub(crate) fn is_value_access_not_present(&self) -> bool {
        matches!(
            self.kind,
            ErrorKind::ValueAccess {
                kind: ValueAccessErrorKind::NotPresent,
                ..
            }
        )
    }

    #[cfg(test)]
    pub(crate) fn is_value_access_unexpected_type(&self) -> bool {
        matches!(
            self.kind,
            ErrorKind::ValueAccess {
                kind: ValueAccessErrorKind::UnexpectedType { .. },
                ..
            }
        )
    }
}
