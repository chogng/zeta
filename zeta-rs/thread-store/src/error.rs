use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ThreadStoreError {
    InvalidBatch(String),
    InvalidQuery(String),
    SequenceConflict { expected: u64, actual: u64 },
    Storage(String),
}

impl fmt::Display for ThreadStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBatch(message) => {
                write!(formatter, "invalid Thread event batch: {message}")
            }
            Self::InvalidQuery(message) => {
                write!(formatter, "invalid Thread history query: {message}")
            }
            Self::SequenceConflict { expected, actual } => {
                write!(
                    formatter,
                    "Thread sequence conflict: expected {expected}, actual {actual}"
                )
            }
            Self::Storage(message) => write!(formatter, "Thread storage error: {message}"),
        }
    }
}

impl std::error::Error for ThreadStoreError {}
