use std::{error, fmt, fmt::Display, io, string};

use serde::de::{self, Expected, Unexpected};

// TODO: eliminate serde duplication?
/// Possible errors that can arise during decoding.
#[derive(Debug)]
#[non_exhaustive]
pub enum DecoderError {
    /// A `std::io::Error` encountered while deserializing.
    IoError(io::Error),

    /// A `std::string::FromUtf8Error` encountered while decoding a UTF-8 String from the input
    /// data.
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
    SyntaxError { message: String },

    /// The end of the BSON input was reached too soon.
    EndOfStream,

    /// An invalid timestamp was encountered while decoding.
    InvalidTimestamp(i64),

    /// An ambiguous timestamp was encountered while decoding.
    AmbiguousTimestamp(i64),

    /// Returned when an index into an array was expected, but got a something else instead.
    /// BSON arrays are expected to be stored as subdocuments with numbered keys. If the input data
    /// contains one which is stored with a different format for its keys, this error will be
    /// returned.
    InvalidArrayKey {
        /// The key that was encountered in the input data.
        actual_key: String,

        /// The index the key was expected to correspond to.
        expected_key: usize,
    },

    /// A field was expected during deserialization, but none was found.
    /// See: https://docs.serde.rs/serde/de/trait.Error.html#method.missing_field
    MissingField(&'static str),

    /// An unexpected field was found during deserialization.
    /// See: https://docs.serde.rs/serde/de/trait.Error.html#method.unknown_field
    UnknownField {
        /// The name of the field that was encountered in the input data.
        actual_field: String,

        /// The list of expected field names.
        expected_fields: Vec<String>,
    },

    /// A value was expected to have a certain type but it had a different one instead.
    /// See: https://docs.serde.rs/serde/de/trait.Error.html#method.invalid_type
    InvalidType {
        /// Information about the type that was encountered in the input data.
        actual_type: String,

        /// Information about the type that was expected.
        expected_type: String,
    },

    /// A sequence or map and the input data contained too many or too few elements.
    /// See: https://docs.serde.rs/serde/de/trait.Error.html#method.invalid_length
    InvalidLength {
        /// The length of the sequence or map in the input data.
        actual_length: usize,

        /// Information about the length that was expected.
        expected_length: String,
    },

    /// A field was repeated.
    /// See: https://docs.serde.rs/serde/de/trait.Error.html#method.duplicate_field
    DuplicatedField(&'static str),

    /// An unknown variant name was encountered when deserializing an enum.
    /// See: https://docs.serde.rs/serde/de/trait.Error.html#method.unknown_variant
    UnknownVariant {
        /// The variant name that was encountered in the input data.
        actual_variant: String,

        /// The list of expected variants names.
        expected_variants: Vec<String>,
    },

    /// A value of the right type was received, but it was wrong for some other reason.
    /// See: https://docs.serde.rs/serde/de/trait.Error.html#method.invalid_value
    InvalidValue {
        /// Information about the value that was received from the input data.
        actual_value: String,

        /// Information about the value that was expected.
        expected_value: String,
    },

    /// A general error encountered during deserialization.
    /// See: https://docs.serde.rs/serde/de/trait.Error.html#tymethod.custom
    Custom {
        /// A message describing the error.
        message: String,
    },
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
            DecoderError::InvalidArrayKey {
                ref actual_key,
                ref expected_key,
            } => write!(
                fmt,
                "invalid array key: expected `{}`, got `{}`",
                expected_key, actual_key
            ),
            DecoderError::MissingField(field_type) => {
                write!(fmt, "expected a field of type `{}`", field_type)
            }
            DecoderError::UnknownField {
                ref expected_fields,
                ref actual_field,
            } => write!(
                fmt,
                "unknown field `{}`. Expected one of {:?}",
                actual_field, expected_fields
            ),
            DecoderError::SyntaxError { message } => message.fmt(fmt),
            DecoderError::EndOfStream => fmt.write_str("end of stream"),
            DecoderError::InvalidType {
                actual_type,
                expected_type,
            } => write!(
                fmt,
                "invalid type \"{}\". expected {}",
                actual_type, expected_type
            ),
            DecoderError::InvalidLength {
                ref actual_length,
                ref expected_length,
            } => write!(
                fmt,
                "invalid length {}, expected {}",
                actual_length, expected_length
            ),
            DecoderError::DuplicatedField(ref field) => write!(fmt, "duplicated field `{}`", field),
            DecoderError::UnknownVariant(ref var) => write!(fmt, "unknown variant `{}`", var),
            DecoderError::InvalidValue(ref desc) => desc.fmt(fmt),
            DecoderError::Custom { ref message } => message.fmt(fmt),
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
            DecoderError::MissingField(_) => "expected a field",
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
        DecoderError::MissingField(field)
    }

    fn duplicate_field(field: &'static str) -> DecoderError {
        DecoderError::DuplicatedField(field)
    }
}

/// Alias for `Result<T, DecoderError>`.
pub type DecoderResult<T> = Result<T, DecoderError>;
