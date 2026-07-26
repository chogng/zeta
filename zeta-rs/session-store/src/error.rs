use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SessionStoreError {
    InvalidBatch(String),
    SequenceConflict { expected: u64, actual: u64 },
    Storage(String),
}

impl fmt::Display for SessionStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBatch(message) => write!(formatter, "invalid Session batch: {message}"),
            Self::SequenceConflict { expected, actual } => {
                write!(
                    formatter,
                    "Session sequence conflict: expected {expected}, actual {actual}"
                )
            }
            Self::Storage(message) => write!(formatter, "Session storage error: {message}"),
        }
    }
}

impl std::error::Error for SessionStoreError {}
