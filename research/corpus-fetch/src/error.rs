use thiserror::Error;

#[derive(Debug, Error)]
pub enum FetchError {
    #[error("http error: {0}")]
    Http(String),
    #[error("crate not found in sparse index: {0}")]
    NotFound(String),
    #[error("malformed sparse-index record: {0}")]
    Malformed(String),
    #[error("invalid crate name: {0}")]
    InvalidName(String),
    #[error("tarball checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },
    #[error("archive error: {0}")]
    Archive(String),
    #[error("io error: {0}")]
    Io(String),
}

impl From<std::io::Error> for FetchError {
    fn from(e: std::io::Error) -> Self {
        FetchError::Io(e.to_string())
    }
}
