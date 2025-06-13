use thiserror::Error as ThisError;

use crate::{
    error::{Error, ErrorKind},
    spec::ElementType,
};

/// The types of errors that can occur when attempting to access a value in a document.
#[derive(Clone, Debug, ThisError)]
#[non_exhaustive]
pub enum ValueAccessErrorKind {
    /// No value for the specified key was present in the document.
    #[error("the key was not present in the document")]
    NotPresent,

    /// The type of the value in the document did not match the requested type.
    #[error("expected type {expected:?}, got type {actual:?}")]
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
    pub(crate) fn value_access_not_present() -> Self {
        ErrorKind::ValueAccess {
            kind: ValueAccessErrorKind::NotPresent,
        }
        .into()
    }

    pub(crate) fn value_access_unexpected_type(actual: ElementType, expected: ElementType) -> Self {
        ErrorKind::ValueAccess {
            kind: ValueAccessErrorKind::UnexpectedType { actual, expected },
        }
        .into()
    }

    pub(crate) fn value_access_invalid_bson(message: String) -> Self {
        ErrorKind::ValueAccess {
            kind: ValueAccessErrorKind::InvalidBson { message },
        }
        .into()
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
