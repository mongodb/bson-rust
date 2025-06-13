use hex::FromHexError;
use thiserror::Error as ThisError;

use crate::error::{Error, ErrorKind};

/// The kinds of errors that can occur when working with the [`ObjectId`](crate::oid::ObjectId)
/// type.
#[derive(Clone, Debug, ThisError)]
#[non_exhaustive]
pub enum ObjectIdErrorKind {
    /// An invalid character was found in the provided hex string. Valid characters are: `0...9`,
    /// `a...f`, or `A...F`.
    #[error("invalid character '{c}' encountered at index {index}")]
    #[non_exhaustive]
    InvalidHexStringCharacter {
        /// The invalid character.
        c: char,

        /// The index at which the invalid character was encountered.
        index: usize,
    },

    /// An `ObjectId` with an invalid length was encountered.
    #[error("invalid hex string length {length}")]
    #[non_exhaustive]
    InvalidHexStringLength {
        /// The length of the invalid hex string.
        length: usize,
    },
}

impl Error {
    // This method is not a From implementation so that it is not part of the public API.
    pub(crate) fn from_hex_error(error: FromHexError, length: usize) -> Self {
        let kind = match error {
            FromHexError::InvalidHexCharacter { c, index } => {
                ObjectIdErrorKind::InvalidHexStringCharacter { c, index }
            }
            FromHexError::InvalidStringLength | FromHexError::OddLength => {
                ObjectIdErrorKind::InvalidHexStringLength { length }
            }
        };
        ErrorKind::ObjectId { kind }.into()
    }

    pub(crate) fn oid_invalid_length(length: usize) -> Self {
        ErrorKind::ObjectId {
            kind: ObjectIdErrorKind::InvalidHexStringLength { length },
        }
        .into()
    }
}
