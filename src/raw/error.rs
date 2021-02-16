/// An error that occurs when attempting to parse raw BSON bytes.
#[derive(Debug, PartialEq)]
pub enum Error {
    /// A BSON value did not fit the expected type.
    UnexpectedType,

    /// A BSON value did not fit the proper format.
    MalformedValue { message: String },

    /// Improper UTF-8 bytes were found when proper UTF-8 was expected. The error value contains
    /// the malformed data as bytes.
    Utf8EncodingError(Vec<u8>),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::UnexpectedType => write!(f, "unexpected type"),
            Self::MalformedValue { message } => write!(f, "malformed value: {:?}", message),
            Self::Utf8EncodingError(_) => write!(f, "utf-8 encoding error"),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;
