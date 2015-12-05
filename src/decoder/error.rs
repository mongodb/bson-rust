use std::{io, error, fmt, str};
use byteorder;
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
    SyntaxError,
    // The end of the BSON input was reached too soon.
    EndOfStream,
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

impl From<byteorder::Error> for DecoderError {
    fn from(err: byteorder::Error) -> DecoderError {
        DecoderError::IoError(From::from(err))
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
            DecoderError::SyntaxError => write!(fmt, "syntax error"),
            DecoderError::EndOfStream => write!(fmt, "end of stream"),
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
            DecoderError::SyntaxError => "syntax error",
            DecoderError::EndOfStream => "end of stream",
        }
    }
    fn cause(&self) -> Option<&error::Error> {
        match *self {
            DecoderError::IoError(ref inner) => Some(inner),
            DecoderError::Utf8Error(ref inner) => Some(inner),
            _ => None
        }
    }
}

impl de::Error for DecoderError {
    fn syntax(_: &str) -> DecoderError {
        DecoderError::SyntaxError
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
