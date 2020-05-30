use std::{error, fmt, fmt::Display, io, string};

use serde::de::{self, Error as _, Unexpected};

use crate::{document::ValueAccessError, oid, Bson};

/// Possible errors that can arise during decoding.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A [`std::io::Error`](https://doc.rust-lang.org/std/io/struct.Error.html) encountered while deserializing.
    IoError(io::Error),

    /// A [`std::string::FromUtf8Error`](https://doc.rust-lang.org/std/string/struct.FromUtf8Error.html) encountered
    /// while decoding a UTF-8 String from the input data.
    FromUtf8Error(string::FromUtf8Error),

    /// While decoding a `Document` from bytes, an unexpected or unsupported element type was
    /// encountered.
    UnrecognizedDocumentElementType {
        /// The key at which an unexpected/unsupported element type was encountered.
        key: String,

        /// The encountered element type.
        element_type: u8,
    },

    /// There was an error with the syntactical structure of the BSON.
    SyntaxError {
        message: String,
    },

    /// The end of the BSON input was reached too soon.
    EndOfStream,

    /// An invalid timestamp was encountered while decoding.
    InvalidTimestamp(i64),

    /// An ambiguous timestamp was encountered while decoding.
    AmbiguousTimestamp(i64),

    InvalidObjectId(oid::Error),

    /// A general error encountered during deserialization.
    /// See: https://docs.serde.rs/serde/de/trait.Error.html
    DeserializationError {
        /// A message describing the error.
        message: String,
    },

    ExtendedJsonParseError(serde_json::Error),
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IoError(err)
    }
}

impl From<string::FromUtf8Error> for Error {
    fn from(err: string::FromUtf8Error) -> Error {
        Error::FromUtf8Error(err)
    }
}

impl From<crate::document::ValueAccessError> for Error {
    fn from(err: crate::document::ValueAccessError) -> Self {
        match err {
            ValueAccessError::NotPresent => Error::custom("missing"),
            ValueAccessError::UnexpectedType => Error::custom("unexpected"),
        }
    }
}

impl From<oid::Error> for Error {
    fn from(err: oid::Error) -> Error {
        Error::InvalidObjectId(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::ExtendedJsonParseError(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::IoError(ref inner) => inner.fmt(fmt),
            Error::FromUtf8Error(ref inner) => inner.fmt(fmt),
            Error::UnrecognizedDocumentElementType {
                ref key,
                element_type,
            } => write!(
                fmt,
                "unrecognized element type for key \"{}\": `{:#x}`",
                key, element_type
            ),
            Error::SyntaxError { ref message } => message.fmt(fmt),
            Error::EndOfStream => fmt.write_str("end of stream"),
            Error::DeserializationError { ref message } => message.fmt(fmt),
            Error::InvalidTimestamp(ref i) => write!(fmt, "no such local time {}", i),
            Error::AmbiguousTimestamp(ref i) => write!(fmt, "ambiguous local time {}", i),
            _ => panic!(""),
        }
    }
}

impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match *self {
            Error::IoError(ref inner) => Some(inner),
            Error::FromUtf8Error(ref inner) => Some(inner),
            _ => None,
        }
    }
}

impl de::Error for Error {
    fn custom<T: Display>(msg: T) -> Error {
        Error::DeserializationError {
            message: msg.to_string(),
        }
    }
}

/// Alias for `Result<T, Error>`.
pub type Result<T> = std::result::Result<T, Error>;

impl Bson {
    /// Method for converting a given `Bson` value to a `serde::de::Unexpected` for error reporting.
    pub(crate) fn as_unexpected(&self) -> Unexpected {
        match self {
            Bson::Array(_) => Unexpected::Seq,
            Bson::Binary(b) => Unexpected::Bytes(b.bytes.as_slice()),
            Bson::Boolean(b) => Unexpected::Bool(*b),
            Bson::DbPointer(_) => Unexpected::Other("dbpointer"),
            Bson::Document(_) => Unexpected::Map,
            Bson::Double(f) => Unexpected::Float(*f),
            Bson::Int32(i) => Unexpected::Signed(*i as i64),
            Bson::Int64(i) => Unexpected::Signed(*i),
            Bson::JavaScriptCode(_) => Unexpected::Other("javascript code"),
            Bson::JavaScriptCodeWithScope(_) => Unexpected::Other("javascript code with scope"),
            Bson::MaxKey => Unexpected::Other("maxkey"),
            Bson::MinKey => Unexpected::Other("minkey"),
            Bson::Null => Unexpected::Unit,
            Bson::Undefined => Unexpected::Other("undefined"),
            Bson::ObjectId(_) => Unexpected::Other("objectid"),
            Bson::RegularExpression(_) => Unexpected::Other("regex"),
            Bson::String(s) => Unexpected::Str(s.as_str()),
            Bson::Symbol(_) => Unexpected::Other("symbol"),
            Bson::Timestamp(_) => Unexpected::Other("timestamp"),
            Bson::DateTime(_) => Unexpected::Other("datetime"),
            Bson::Decimal128(_) => Unexpected::Other("decimal128"),
        }
    }
}
