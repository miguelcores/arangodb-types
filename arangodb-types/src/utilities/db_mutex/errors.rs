use std::error::Error;
use std::fmt;
use std::fmt::Display;

#[derive(Debug)]
pub enum DBMutexError {
    NotFound,
    Timeout,
    Other(anyhow::Error),
}

impl Error for DBMutexError {}

impl Display for DBMutexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DBMutexError::NotFound => f.write_str("Document not found"),
            DBMutexError::Timeout => f.write_str("Cannot lock document because timed out"),
            DBMutexError::Other(v) => v.fmt(f),
        }
    }
}

impl From<anyhow::Error> for DBMutexError {
    fn from(e: anyhow::Error) -> Self {
        DBMutexError::Other(e)
    }
}
