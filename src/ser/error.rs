use std::{error, fmt, fmt::Display, io, sync::Arc};

use serde::ser;

use crate::bson::Bson;

/// Possible errors that can arise during encoding.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Error {
    /// A [`std::io::Error`](https://doc.rust-lang.org/std/io/struct.Error.html) encountered while serializing.
    Io(Arc<io::Error>),

    /// A key could not be serialized to a BSON string.
    InvalidDocumentKey(Bson),

    /// An invalid string was specified.
    InvalidCString(String),

    /// A general error that occurred during serialization.
    /// See: <https://docs.rs/serde/1.0.110/serde/ser/trait.Error.html#tymethod.custom>
    #[non_exhaustive]
    SerializationError {
        /// A message describing the error.
        message: String,
    },

    /// An unsigned integer type could not fit into a signed integer type.
    UnsignedIntegerExceededRange(u64),

    #[cfg(feature = "serde_path_to_error")]
    #[non_exhaustive]
    WithPath {
        /// The path to the error.
        path: serde_path_to_error::Path,

        /// The original error.
        source: Box<Error>,
    },
}

impl Error {
    #[cfg(feature = "serde_path_to_error")]
    pub(crate) fn with_path(err: serde_path_to_error::Error<Error>) -> Self {
        let path = err.path().clone();
        let source = Box::new(err.into_inner());
        Self::WithPath { path, source }
    }

    #[cfg(test)]
    pub(crate) fn strip_path(self) -> Self {
        #[cfg(feature = "serde_path_to_error")]
        match self {
            Self::WithPath { path: _, source } => *source,
            _ => self,
        }
        #[cfg(not(feature = "serde_path_to_error"))]
        {
            self
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(Arc::new(err))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::Io(inner) => inner.fmt(fmt),
            Error::InvalidDocumentKey(key) => write!(fmt, "Invalid map key type: {}", key),
            Error::InvalidCString(ref string) => {
                write!(fmt, "cstrings cannot contain null bytes: {:?}", string)
            }
            Error::SerializationError { message } => message.fmt(fmt),
            Error::UnsignedIntegerExceededRange(value) => write!(
                fmt,
                "BSON does not support unsigned integers.
                 An attempt to serialize the value: {} in a signed type failed due to the value's \
                 size.",
                value
            ),
            #[cfg(feature = "serde_path_to_error")]
            Error::WithPath { path, source } => write!(fmt, "error at {}: {}", path, source),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            Error::Io(ref inner) => Some(inner.as_ref()),
            _ => None,
        }
    }
}

impl ser::Error for Error {
    fn custom<T: Display>(msg: T) -> Error {
        Error::SerializationError {
            message: msg.to_string(),
        }
    }
}

/// Alias for `Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;
