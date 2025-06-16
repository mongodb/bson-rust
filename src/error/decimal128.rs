use thiserror::Error as ThisError;

use crate::error::{Error, ErrorKind};

/// The kinds of errors that can occur when working with the [`Decimal128`](crate::Decimal128) type.
#[derive(Clone, Debug, ThisError)]
#[non_exhaustive]
pub enum Decimal128ErrorKind {
    /// Empty exponent.
    #[error("empty exponent")]
    EmptyExponent,

    /// Invalid exponent.
    #[error("invalid exponent: {message}")]
    #[non_exhaustive]
    InvalidExponent {
        /// A message describing the error.
        message: String,
    },

    /// Invalid coefficient.
    #[error("invalid coefficient: {message}")]
    #[non_exhaustive]
    InvalidCoefficient {
        /// A message describing the error.
        message: String,
    },

    /// Overflow.
    #[error("overflow")]
    Overflow,

    /// Underflow.
    #[error("underflow")]
    Underflow,

    /// Inexact rounding.
    #[error("inexact rounding")]
    InexactRounding,

    /// Unparseable.
    #[error("unparseable")]
    Unparseable,
}

impl Error {
    pub(crate) fn decimal128(kind: Decimal128ErrorKind) -> Self {
        ErrorKind::Decimal128 { kind }.into()
    }

    #[cfg(test)]
    pub(crate) fn is_decimal128_unparseable(&self) -> bool {
        matches!(
            self.kind,
            ErrorKind::Decimal128 {
                kind: Decimal128ErrorKind::Unparseable,
            }
        )
    }
}
