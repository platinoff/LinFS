use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("corruption: {0}")]
    Corruption(String),
    #[error("unsupported: {0}")]
    Unsupported(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("permission denied: {0}")]
    Permission(String),
}

pub type Result<T> = std::result::Result<T, Error>;
