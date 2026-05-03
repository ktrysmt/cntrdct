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
}
