use thiserror::Error as ThisError;

use crate::{
    error::{Error, ErrorKind},
    spec::BinarySubtype,
    UuidRepresentation,
};

/// The kinds of errors that can occur when working with the [`Uuid`](crate::uuid::Uuid) type.
#[derive(Clone, Debug, ThisError)]
#[non_exhaustive]
pub enum UuidErrorKind {
    /// An invalid string was used to construct a UUID.
    #[error("invalid UUID string")]
    #[non_exhaustive]
    InvalidString {},

    /// The requested `UuidRepresentation` does not match the binary subtype of a `Binary`
    /// value.
    #[error(
        "expected binary subtype {expected_binary_subtype:?} for representation \
         {requested_representation:?}, got {actual_binary_subtype:?}"
    )]
    #[non_exhaustive]
    RepresentationMismatch {
        /// The subtype that was expected given the requested representation.
        expected_binary_subtype: BinarySubtype,

        /// The actual subtype of the binary value.
        actual_binary_subtype: BinarySubtype,

        /// The requested representation.
        requested_representation: UuidRepresentation,
    },

    /// An invalid length of bytes was used to construct a UUID value.
    #[error("expected length of 16 bytes, got {length}")]
    #[non_exhaustive]
    InvalidLength {
        /// The actual length of the data.
        length: usize,
    },
}

impl Error {
    pub(crate) fn invalid_uuid_string(message: impl ToString) -> Self {
        Self::from(ErrorKind::Uuid {
            kind: UuidErrorKind::InvalidString {},
        })
        .with_message(message)
    }

    pub(crate) fn uuid_representation_mismatch(
        requested_representation: UuidRepresentation,
        actual_binary_subtype: BinarySubtype,
        expected_binary_subtype: BinarySubtype,
    ) -> Self {
        ErrorKind::Uuid {
            kind: UuidErrorKind::RepresentationMismatch {
                expected_binary_subtype,
                actual_binary_subtype,
                requested_representation,
            },
        }
        .into()
    }

    pub(crate) fn invalid_uuid_length(length: usize) -> Self {
        ErrorKind::Uuid {
            kind: UuidErrorKind::InvalidLength { length },
        }
        .into()
    }
}
