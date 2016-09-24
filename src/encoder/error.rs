use std::{io, error, fmt};
use byteorder;
use serde::ser;
use bson::Bson;

/// Possible errors that can arise during encoding.
#[derive(Debug)]
pub enum EncoderError {
    IoError(io::Error),
    InvalidMapKeyType(Bson),
    Unknown(String),
}

impl From<io::Error> for EncoderError {
    fn from(err: io::Error) -> EncoderError {
        EncoderError::IoError(err)
    }
}

impl From<byteorder::Error> for EncoderError {
    fn from(err: byteorder::Error) -> EncoderError {
        EncoderError::IoError(From::from(err))
    }
}

impl fmt::Display for EncoderError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match self {
            &EncoderError::IoError(ref inner) => inner.fmt(fmt),
            &EncoderError::InvalidMapKeyType(ref bson) => {
                write!(fmt, "Invalid map key type: {:?}", bson)
            }
            &EncoderError::Unknown(ref inner) => inner.fmt(fmt),
        }
    }
}

impl error::Error for EncoderError {
    fn description(&self) -> &str {
        match self {
            &EncoderError::IoError(ref inner) => inner.description(),
            &EncoderError::InvalidMapKeyType(_) => "Invalid map key type",
            &EncoderError::Unknown(ref inner) => inner,
        }
    }
    fn cause(&self) -> Option<&error::Error> {
        match self {
            &EncoderError::IoError(ref inner) => Some(inner),
            _ => None,
        }
    }
}

impl ser::Error for EncoderError {
    fn custom<T: Into<String>>(msg: T) -> EncoderError {
        EncoderError::Unknown(msg.into())
    }
}

/// Alias for `Result<T, EncoderError>`.
pub type EncoderResult<T> = Result<T, EncoderError>;
