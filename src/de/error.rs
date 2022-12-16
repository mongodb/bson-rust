use std::{error, fmt, fmt::Display, io, string, sync::Arc};

use serde::de::{self, Unexpected};

use crate::Bson;

/// Possible errors that can arise during decoding.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum Error {
    /// A [`std::io::Error`](https://doc.rust-lang.org/std/io/struct.Error.html) encountered while deserializing.
    Io(Arc<io::Error>),

    /// A [`std::string::FromUtf8Error`](https://doc.rust-lang.org/std/string/struct.FromUtf8Error.html) encountered
    /// while decoding a UTF-8 String from the input data.
    InvalidUtf8String(string::FromUtf8Error),

    /// While decoding a [`Document`](crate::Document) from bytes, an unexpected or unsupported
    /// element type was encountered.
    #[non_exhaustive]
    UnrecognizedDocumentElementType {
        /// The key at which an unexpected/unsupported element type was encountered.
        key: String,

        /// The encountered element type.
        element_type: u8,
    },

    /// The end of the BSON input was reached too soon.
    EndOfStream,

    /// A general error encountered during deserialization.
    /// See: <https://docs.serde.rs/serde/de/trait.Error.html>
    #[non_exhaustive]
    DeserializationError {
        /// A message describing the error.
        message: String,
    },
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(Arc::new(err))
    }
}

impl From<string::FromUtf8Error> for Error {
    fn from(err: string::FromUtf8Error) -> Error {
        Error::InvalidUtf8String(err)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref inner) => inner.fmt(fmt),
            Error::InvalidUtf8String(ref inner) => inner.fmt(fmt),
            Error::UnrecognizedDocumentElementType {
                ref key,
                element_type,
            } => write!(
                fmt,
                "unrecognized element type for key \"{}\": `{:#x}`",
                key, element_type
            ),
            Error::EndOfStream => fmt.write_str("end of stream"),
            Error::DeserializationError { ref message } => message.fmt(fmt),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match *self {
            Error::Io(ref inner) => Some(inner.as_ref()),
            Error::InvalidUtf8String(ref inner) => Some(inner),
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
    /// Method for converting a given [`Bson`] value to a [`serde::de::Unexpected`] for error
    /// reporting.
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
