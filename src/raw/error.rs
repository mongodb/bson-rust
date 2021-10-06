use std::str::Utf8Error;

use crate::spec::ElementType;

/// An error that occurs when attempting to parse raw BSON bytes.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// A BSON value did not fit the expected type.
    #[non_exhaustive]
    UnexpectedType {
        actual: ElementType,
        expected: ElementType,
    },

    /// A BSON value did not fit the proper format.
    #[non_exhaustive]
    MalformedValue { message: String },

    /// Improper UTF-8 bytes were found when proper UTF-8 was expected. The error value contains
    /// the malformed data as bytes.
    Utf8EncodingError(Utf8Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::UnexpectedType { actual, expected } => write!(
                f,
                "unexpected element type: {:?}, expected: {:?}",
                actual, expected
            ),
            Self::MalformedValue { message } => write!(f, "malformed value: {:?}", message),
            Self::Utf8EncodingError(e) => write!(f, "utf-8 encoding error: {}", e),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
