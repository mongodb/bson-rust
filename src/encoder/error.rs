use bson::Bson;
use serde::ser;
use std::fmt::Display;
use std::{error, fmt, io};

/// Possible errors that can arise during encoding.
#[derive(Debug)]
pub enum EncoderError {
    IoError(io::Error),
    InvalidMapKeyType(Bson),
    Unknown(String),
    UnsupportedUnsignedType,
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
            EncoderError::InvalidMapKeyType(ref bson) => write!(fmt, "Invalid map key type: {:?}", bson),
            EncoderError::Unknown(ref inner) => inner.fmt(fmt),
            EncoderError::UnsupportedUnsignedType => fmt.write_str("BSON does not support unsigned type"),
            EncoderError::UnsignedTypesValueExceedsRange(value) => write!(
                                                                          fmt,
                                                                          "BSON does not support unsigned types.
                 An attempt to encode the value: {} in a signed type failed due to the value's size.",
                                                                          value
            ),
        }
    }
}

impl error::Error for EncoderError {
    fn description(&self) -> &str {
        match *self {
            EncoderError::IoError(ref inner) => inner.description(),
            EncoderError::InvalidMapKeyType(_) => "Invalid map key type",
            EncoderError::Unknown(ref inner) => inner,
            EncoderError::UnsupportedUnsignedType => "BSON does not support unsigned type",
            EncoderError::UnsignedTypesValueExceedsRange(_) => {
                "BSON does not support unsigned types.
                 An attempt to encode the value: {} in a signed type failed due to the values size."
            }
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            EncoderError::IoError(ref inner) => Some(inner),
            _ => None,
        }
    }
}

impl ser::Error for EncoderError {
    fn custom<T: Display>(msg: T) -> EncoderError {
        EncoderError::Unknown(msg.to_string())
    }
}

/// Alias for `Result<T, EncoderError>`.
pub type EncoderResult<T> = Result<T, EncoderError>;
