use thiserror::Error as ThisError;

use crate::error::{Error, ErrorKind};

/// The kinds of errors that can occur when working with the [`DateTime`](crate::DateTime) type.
#[derive(Clone, Debug, ThisError)]
pub enum DateTimeErrorKind {
    /// The `DateTime` could not be formatted.
    #[error("{message}")]
    #[non_exhaustive]
    CannotFormat {
        /// A message describing the error.
        message: String,
    },

    /// An invalid value was provided.
    #[error("{message}")]
    #[non_exhaustive]
    InvalidValue {
        /// A message describing the error.
        message: String,
    },
}

impl Error {
    pub(crate) fn invalid_datetime_value(message: impl ToString) -> Self {
        ErrorKind::DateTime {
            kind: DateTimeErrorKind::InvalidValue {
                message: message.to_string(),
            },
        }
        .into()
    }

    pub(crate) fn cannot_format_datetime(message: impl ToString) -> Self {
        ErrorKind::DateTime {
            kind: DateTimeErrorKind::CannotFormat {
                message: message.to_string(),
            },
        }
        .into()
    }
}
