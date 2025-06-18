use thiserror::Error as ThisError;

use crate::error::{Error, ErrorKind};

/// The kinds of errors that can occur when working with the [`Decimal128`](crate::Decimal128) type.
#[derive(Clone, Debug, ThisError)]
#[non_exhaustive]
pub enum Decimal128ErrorKind {
    /// Empty exponent.
    #[error("empty exponent")]
    #[non_exhaustive]
    EmptyExponent {},

    /// Invalid exponent.
    #[error("invalid exponent")]
    #[non_exhaustive]
    InvalidExponent {},

    /// Invalid coefficient.
    #[error("invalid coefficient")]
    #[non_exhaustive]
    InvalidCoefficient {},

    /// Overflow.
    #[error("overflow")]
    #[non_exhaustive]
    Overflow {},

    /// Underflow.
    #[error("underflow")]
    #[non_exhaustive]
    Underflow {},

    /// Inexact rounding.
    #[error("inexact rounding")]
    #[non_exhaustive]
    InexactRounding {},

    /// Unparseable.
    #[error("unparseable")]
    #[non_exhaustive]
    Unparseable {},
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
                kind: Decimal128ErrorKind::Unparseable {},
            }
        )
    }
}
