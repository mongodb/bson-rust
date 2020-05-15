use std::{error, fmt, fmt::Display, io, string};

use serde::de::{self, Expected, Unexpected};

/// Possible errors that can arise during decoding.
#[derive(Debug)]
#[non_exhaustive]
pub enum DecoderError {
    IoError(io::Error),
    FromUtf8Error(string::FromUtf8Error),
    UnrecognizedDocumentElementType { key: String, element_type: u8 },
    InvalidArrayKey(usize, String),
    // A field was expected, but none was found.
    ExpectedField(&'static str),
    // An unexpected field was found.
    UnknownField(String),
    // There was an error with the syntactical structure of the BSON.
    SyntaxError(String),
    // The end of the BSON input was reached too soon.
    EndOfStream,
    // Invalid Type
    InvalidType(String),
    // Invalid Length
    InvalidLength(usize, String),
    // Duplicated Field
    DuplicatedField(&'static str),
    // Unknown Variant
    UnknownVariant(String),
    // Invalid value
    InvalidValue(String),
    // Invalid timestamp
    InvalidTimestamp(i64),
    // Ambiguous timestamp
    AmbiguousTimestamp(i64),

    Unknown(String),
}

impl From<io::Error> for DecoderError {
    fn from(err: io::Error) -> DecoderError {
        DecoderError::IoError(err)
    }
}

impl From<string::FromUtf8Error> for DecoderError {
    fn from(err: string::FromUtf8Error) -> DecoderError {
        DecoderError::FromUtf8Error(err)
    }
}

impl fmt::Display for DecoderError {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            DecoderError::IoError(ref inner) => inner.fmt(fmt),
            DecoderError::FromUtf8Error(ref inner) => inner.fmt(fmt),
            DecoderError::UnrecognizedDocumentElementType {
                ref key,
                element_type,
            } => write!(
                fmt,
                "unrecognized element type for key \"{}\": `{}`",
                key, element_type
            ),
            DecoderError::InvalidArrayKey(ref want, ref got) => {
                write!(fmt, "invalid array key: expected `{}`, got `{}`", want, got)
            }
            DecoderError::ExpectedField(field_type) => {
                write!(fmt, "expected a field of type `{}`", field_type)
            }
            DecoderError::UnknownField(ref field) => write!(fmt, "unknown field `{}`", field),
            DecoderError::SyntaxError(ref inner) => inner.fmt(fmt),
            DecoderError::EndOfStream => fmt.write_str("end of stream"),
            DecoderError::InvalidType(ref desc) => desc.fmt(fmt),
            DecoderError::InvalidLength(ref len, ref desc) => {
                write!(fmt, "expecting length {}, {}", len, desc)
            }
            DecoderError::DuplicatedField(ref field) => write!(fmt, "duplicated field `{}`", field),
            DecoderError::UnknownVariant(ref var) => write!(fmt, "unknown variant `{}`", var),
            DecoderError::InvalidValue(ref desc) => desc.fmt(fmt),
            DecoderError::Unknown(ref inner) => inner.fmt(fmt),
            DecoderError::InvalidTimestamp(ref i) => write!(fmt, "no such local time {}", i),
            DecoderError::AmbiguousTimestamp(ref i) => write!(fmt, "ambiguous local time {}", i),
        }
    }
}

impl error::Error for DecoderError {
    fn description(&self) -> &str {
        match *self {
            DecoderError::IoError(ref inner) =>
            {
                #[allow(deprecated)]
                inner.description()
            }
            DecoderError::FromUtf8Error(ref inner) =>
            {
                #[allow(deprecated)]
                inner.description()
            }
            DecoderError::UnrecognizedDocumentElementType { .. } => "unrecognized element type",
            DecoderError::InvalidArrayKey(..) => "invalid array key",
            DecoderError::ExpectedField(_) => "expected a field",
            DecoderError::UnknownField(_) => "found an unknown field",
            DecoderError::SyntaxError(ref inner) => inner,
            DecoderError::EndOfStream => "end of stream",
            DecoderError::InvalidType(ref desc) => desc,
            DecoderError::InvalidLength(_, ref desc) => desc,
            DecoderError::DuplicatedField(_) => "duplicated field",
            DecoderError::UnknownVariant(_) => "unknown variant",
            DecoderError::InvalidValue(ref desc) => desc,
            DecoderError::Unknown(ref inner) => inner,
            DecoderError::InvalidTimestamp(..) => "no such local time",
            DecoderError::AmbiguousTimestamp(..) => "ambiguous local time",
        }
    }
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            DecoderError::IoError(ref inner) => Some(inner),
            DecoderError::FromUtf8Error(ref inner) => Some(inner),
            _ => None,
        }
    }
}

impl de::Error for DecoderError {
    fn custom<T: Display>(msg: T) -> DecoderError {
        DecoderError::Unknown(msg.to_string())
    }

    fn invalid_type(_unexp: Unexpected, exp: &dyn Expected) -> DecoderError {
        DecoderError::InvalidType(exp.to_string())
    }

    fn invalid_value(_unexp: Unexpected, exp: &dyn Expected) -> DecoderError {
        DecoderError::InvalidValue(exp.to_string())
    }

    fn invalid_length(len: usize, exp: &dyn Expected) -> DecoderError {
        DecoderError::InvalidLength(len, exp.to_string())
    }

    fn unknown_variant(variant: &str, _expected: &'static [&'static str]) -> DecoderError {
        DecoderError::UnknownVariant(variant.to_string())
    }

    fn unknown_field(field: &str, _expected: &'static [&'static str]) -> DecoderError {
        DecoderError::UnknownField(String::from(field))
    }

    fn missing_field(field: &'static str) -> DecoderError {
        DecoderError::ExpectedField(field)
    }

    fn duplicate_field(field: &'static str) -> DecoderError {
        DecoderError::DuplicatedField(field)
    }
}

/// Alias for `Result<T, DecoderError>`.
pub type DecoderResult<T> = Result<T, DecoderError>;
