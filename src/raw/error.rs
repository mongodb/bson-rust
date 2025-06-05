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
    pub(crate) fn new(kind: ErrorKind) -> Self {
        Self { key: None, kind }
    }

    pub(crate) fn malformed(e: impl ToString) -> Self {
        Self::new(ErrorKind::MalformedValue {
            message: e.to_string(),
        })
    }

    pub(crate) fn with_key(mut self, key: impl Into<String>) -> Self {
        self.key = Some(key.into());
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
    Utf8EncodingError,
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
            ErrorKind::Utf8EncodingError => write!(f, "{}utf-8 encoding error", prefix),
        }
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

/// Execute the provided closure, mapping the key of the returned error (if any) to the provided
/// key.
pub(crate) fn try_with_key<G, F: FnOnce() -> Result<G>>(key: impl Into<String>, f: F) -> Result<G> {
    f().map_err(|e| e.with_key(key))
}
