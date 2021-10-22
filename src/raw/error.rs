use std::str::Utf8Error;

use crate::spec::ElementType;

/// An error that occurs when attempting to parse raw BSON bytes.
#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
pub struct Error {
    /// The type of error that was encountered.
    pub kind: ErrorKind,

    /// They key associated with the error, if any.
    pub(crate) key: Option<String>,
}

impl Error {
    pub(crate) fn new_with_key(key: impl Into<String>, kind: ErrorKind) -> Self {
        Self {
            kind,
            key: Some(key.into()),
        }
    }

    pub(crate) fn new_without_key(kind: ErrorKind) -> Self {
        Self { key: None, kind }
    }

    pub(crate) fn with_key(mut self, key: impl AsRef<str>) -> Self {
        self.key = Some(key.as_ref().to_string());
        self
    }

    /// The key at which the error was encountered, if any.
    pub fn key(&self) -> Option<&str> {
        self.key.as_deref()
    }
}

/// The different categories of errors that can be returned when reading from raw BSON.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum ErrorKind {
    /// A BSON value did not fit the proper format.
    #[non_exhaustive]
    MalformedValue { message: String },

    /// Improper UTF-8 bytes were found when proper UTF-8 was expected.
    Utf8EncodingError(Utf8Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let p = self
            .key
            .as_ref()
            .map(|k| format!("error at key \"{}\": ", k));

        let prefix = p.as_ref().map_or("", |p| p.as_str());

        match &self.kind {
            ErrorKind::MalformedValue { message } => {
                write!(f, "{}malformed value: {:?}", prefix, message)
            }
            ErrorKind::Utf8EncodingError(e) => write!(f, "{}utf-8 encoding error: {}", prefix, e),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// Execute the provided closure, mapping the key of the returned error (if any) to the provided
/// key.
pub(crate) fn try_with_key<G, F: FnOnce() -> Result<G>>(key: impl AsRef<str>, f: F) -> Result<G> {
    f().map_err(|e| e.with_key(key))
}

pub type ValueAccessResult<T> = std::result::Result<T, ValueAccessError>;

/// Error to indicate that either a value was empty or it contained an unexpected
/// type, for use with the direct getters (e.g. [`crate::RawDocument::get_str`]).
#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
pub struct ValueAccessError {
    /// The type of error that was encountered.
    pub kind: ValueAccessErrorKind,

    /// The key at which the error was encountered.
    pub(crate) key: String,
}

impl ValueAccessError {
    /// The key at which the error was encountered.
    pub fn key(&self) -> &str {
        self.key.as_str()
    }
}

/// The type of error encountered when using a direct getter (e.g. [`crate::RawDocument::get_str`]).
#[derive(Debug, PartialEq, Clone)]
#[non_exhaustive]
pub enum ValueAccessErrorKind {
    /// Cannot find the expected field with the specified key
    NotPresent,

    /// Found a Bson value with the specified key, but not with the expected type
    #[non_exhaustive]
    UnexpectedType {
        /// The type that was expected.
        expected: ElementType,

        /// The actual type that was encountered.
        actual: ElementType,
    },

    /// An error was encountered attempting to decode the document.
    InvalidBson(super::Error),
}

impl std::fmt::Display for ValueAccessError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let prefix = format!("error at key: \"{}\": ", self.key);

        match &self.kind {
            ValueAccessErrorKind::UnexpectedType { actual, expected } => write!(
                f,
                "{} unexpected element type: {:?}, expected: {:?}",
                prefix, actual, expected
            ),
            ValueAccessErrorKind::InvalidBson(error) => {
                write!(f, "{}: {}", prefix, error)
            }
            ValueAccessErrorKind::NotPresent => write!(f, "{}value not present", prefix),
        }
    }
}

impl std::error::Error for ValueAccessError {}
