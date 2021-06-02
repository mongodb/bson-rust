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

    /// A general error that occurred during serialization.
    /// See: https://docs.rs/serde/1.0.110/serde/ser/trait.Error.html#tymethod.custom
    #[non_exhaustive]
    SerializationError {
        /// A message describing the error.
        message: String,
    },

    #[cfg(not(feature = "u2i"))]
    /// Returned when serialization of an unsigned integer was attempted. BSON only supports
    /// 32-bit and 64-bit signed integers.
    ///
    /// To serialize unsigned integers to BSON, use an appropriate helper from
    /// [`crate::serde_helpers`] or enable the "u2i" feature flag.
    UnsupportedUnsignedInteger(u64),

    #[cfg(feature = "u2i")]
    /// An unsigned integer type could not fit into a signed integer type.
    UnsignedIntegerExceededRange(u64),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(Arc::new(err))
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref inner) => inner.fmt(fmt),
            Error::InvalidDocumentKey(ref key) => write!(fmt, "Invalid map key type: {}", key),
            Error::SerializationError { ref message } => message.fmt(fmt),
            #[cfg(not(feature = "u2i"))]
            Error::UnsupportedUnsignedInteger(value) => write!(
                fmt,
                "BSON does not support unsigned integers, cannot serialize value: {}. To \
                 serialize unsigned integers to BSON, use an appropriate serde helper or enable \
                 the u2i feature.",
                value
            ),
            #[cfg(feature = "u2i")]
            Error::UnsignedIntegerExceededRange(value) => write!(
                fmt,
                "BSON does not support unsigned integers.
                 An attempt to serialize the value: {} in a signed type failed due to the value's \
                 size.",
                value
            ),
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
