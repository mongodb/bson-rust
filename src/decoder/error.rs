use std::{io, error, fmt, str};
use serde::de;

/// Possible errors that can arise during decoding.
#[derive(Debug)]
pub enum DecoderError {
    IoError(io::Error),
    Utf8Error(str::Utf8Error),
    UnrecognizedElementType(u8),
    InvalidArrayKey(usize, String),
    // A field was expected, but none was found.
    ExpectedField(&'static str),
    // An unexpected field was found.
    UnknownField(String),
    // There was an error with the syntactical structure of the BSON.
    SyntaxError(String),
    // The end of the BSON input was reached too soon.
    EndOfStream,
    Unknown(String),
}

impl From<io::Error> for DecoderError {
    fn from(err: io::Error) -> DecoderError {
        DecoderError::IoError(err)
    }
}

impl From<str::Utf8Error> for DecoderError {
    fn from(err: str::Utf8Error) -> DecoderError {
        DecoderError::Utf8Error(err)
    }
}

impl fmt::Display for DecoderError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DecoderError::IoError(ref inner) => inner.fmt(fmt),
            DecoderError::Utf8Error(ref inner) => inner.fmt(fmt),
            DecoderError::UnrecognizedElementType(tag) => {
                write!(fmt, "unrecognized element type `{}`", tag)
            }
            DecoderError::InvalidArrayKey(ref want, ref got) => {
                write!(fmt, "invalid array key: expected `{}`, got `{}`", want, got)
            }
            DecoderError::ExpectedField(field_type) => {
                write!(fmt, "expected a field of type `{}`", field_type)
            }
            DecoderError::UnknownField(ref field) => write!(fmt, "unknown field `{}`", field),
            DecoderError::SyntaxError(ref inner) => inner.fmt(fmt),
            DecoderError::EndOfStream => write!(fmt, "end of stream"),
            DecoderError::Unknown(ref inner) => inner.fmt(fmt),
        }
    }
}

impl error::Error for DecoderError {
    fn description(&self) -> &str {
        match *self {
            DecoderError::IoError(ref inner) => inner.description(),
            DecoderError::Utf8Error(ref inner) => inner.description(),
            DecoderError::UnrecognizedElementType(_) => "unrecognized element type",
            DecoderError::InvalidArrayKey(_, _) => "invalid array key",
            DecoderError::ExpectedField(_) => "expected a field",
            DecoderError::UnknownField(_) => "found an unknown field",
            DecoderError::SyntaxError(ref inner) => inner,
            DecoderError::EndOfStream => "end of stream",
            DecoderError::Unknown(ref inner) => inner,
        }
    }
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            DecoderError::IoError(ref inner) => Some(inner),
            DecoderError::Utf8Error(ref inner) => Some(inner),
            _ => None,
        }
    }
}

impl de::Error for DecoderError {
    fn custom<T: Into<String>>(msg: T) -> DecoderError {
        DecoderError::Unknown(msg.into())
    }

    fn invalid_value(msg: &str) -> DecoderError {
        DecoderError::SyntaxError(msg.to_owned())
    }

    fn end_of_stream() -> DecoderError {
        DecoderError::EndOfStream
    }

    fn unknown_field(field: &str) -> DecoderError {
        DecoderError::UnknownField(String::from(field))
    }

    fn missing_field(field: &'static str) -> DecoderError {
        DecoderError::ExpectedField(field)
    }
}

/// Alias for `Result<T, DecoderError>`.
pub type DecoderResult<T> = Result<T, DecoderError>;
