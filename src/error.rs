use thiserror::Error;

use crate::spec::ElementType;

pub type Result<T> = std::result::Result<T, Error>;

/// An error that can occur in the `bson` crate.
#[derive(Debug, Error)]
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
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Malformed BSON bytes were encountered.
    #[error("Malformed BSON: {message}")]
    #[non_exhaustive]
    MalformedValue { message: String },

    /// Invalid UTF-8 bytes were encountered.
    #[error("Invalid UTF-8")]
    Utf8Encoding,

    /// An error occurred when attempting to access a value in a document.
    #[error("An error occurred when attempting to access a document value: {kind}")]
    #[non_exhaustive]
    ValueAccess {
        /// The kind of error that occurred.
        kind: ValueAccessErrorKind,
    },

    /// A [`std::io::Error`] occurred.
    #[error("An IO error occurred: {0}")]
    Io(std::io::Error),

    /// A wrapped deserialization error.
    /// TODO RUST-1406: collapse this
    #[cfg(feature = "serde")]
    #[error("Deserialization error: {0}")]
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

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        ErrorKind::Io(value).into()
    }
}

#[cfg(feature = "serde")]
impl From<crate::de::Error> for Error {
    fn from(value: crate::de::Error) -> Self {
        ErrorKind::DeError(value).into()
    }
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
    pub(crate) fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
        self
    }

    pub(crate) fn with_index(mut self, index: usize) -> Self {
        self.index = Some(index);
        self
    }

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

    pub(crate) fn malformed_value(message: impl ToString) -> Self {
        ErrorKind::MalformedValue {
            message: message.to_string(),
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

    #[cfg(all(test, feature = "serde"))]
    pub(crate) fn is_malformed_value(&self) -> bool {
        matches!(self.kind, ErrorKind::MalformedValue { .. },)
    }
}
