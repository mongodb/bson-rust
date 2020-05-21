use std::{error, fmt, fmt::Display, io};

use serde::ser;

use crate::bson::Bson;

/// Possible errors that can arise during encoding.
#[derive(Debug)]
#[non_exhaustive]
pub enum EncoderError {
    /// A `std::io::Error` encountered while serializing.
    IoError(io::Error),

    /// A key could not be serialized to a BSON string.
    InvalidMapKeyType {
        /// The value that could not be used as a key.
        key: Bson,
    },

    /// A general error that ocurred during serialization.
    /// See: https://docs.rs/serde/1.0.110/serde/ser/trait.Error.html#tymethod.custom
    SerializationError {
        /// A message describing the error.
        message: String,
    },

    #[cfg(not(feature = "u2i"))]
    /// Returned when serialization of an unsigned integer was attempted. BSON only supports
    /// 32-bit and 64-bit signed integers.
    ///
    /// To allow serialization of unsigned integers as signed integers, enable the "u2i" feature
    /// flag.
    UnsupportedUnsignedType,

    #[cfg(feature = "u2i")]
    /// An unsigned integer type could not fit into a signed integer type.
    UnsignedTypesValueExceedsRange(u64),
}

impl From<io::Error> for EncoderError {
    fn from(err: io::Error) -> EncoderError {
        EncoderError::IoError(err)
    }
}

impl fmt::Display for EncoderError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            EncoderError::IoError(ref inner) => inner.fmt(fmt),
            EncoderError::InvalidMapKeyType { ref key } => {
                write!(fmt, "Invalid map key type: {}", key)
            }
            EncoderError::SerializationError { ref message } => message.fmt(fmt),
            #[cfg(not(feature = "u2i"))]
            EncoderError::UnsupportedUnsignedType => {
                fmt.write_str("BSON does not support unsigned type")
            }
            #[cfg(feature = "u2i")]
            EncoderError::UnsignedTypesValueExceedsRange(value) => write!(
                fmt,
                "BSON does not support unsigned types.
                 An attempt to encode the value: {} in a signed type failed due to the value's \
                 size.",
                value
            ),
        }
    }
}

impl error::Error for EncoderError {
    fn description(&self) -> &str {
        match *self {
            EncoderError::IoError(ref inner) =>
            {
                #[allow(deprecated)]
                inner.description()
            }
            EncoderError::InvalidMapKeyType { .. } => "Invalid map key type",
            EncoderError::SerializationError { ref message } => message.as_str(),
            #[cfg(not(feature = "u2i"))]
            EncoderError::UnsupportedUnsignedType => "BSON does not support unsigned type",
            #[cfg(feature = "u2i")]
            EncoderError::UnsignedTypesValueExceedsRange(_) => {
                "BSON does not support unsigned types.
                 An attempt to encode the value in a signed type failed due to the values size."
            }
        }
    }

    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            EncoderError::IoError(ref inner) => Some(inner),
            _ => None,
        }
    }
}

impl ser::Error for EncoderError {
    fn custom<T: Display>(msg: T) -> EncoderError {
        EncoderError::SerializationError {
            message: msg.to_string(),
        }
    }
}

/// Alias for `Result<T, EncoderError>`.
pub type EncoderResult<T> = Result<T, EncoderError>;
