use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoreError {
    InvalidTransition { from: String, to: String },
    Journal(String),
    NotFound(String),
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTransition { from, to } => {
                write!(formatter, "cannot transition from {from} to {to}")
            }
            Self::Journal(message) => write!(formatter, "journal error: {message}"),
            Self::NotFound(value) => write!(formatter, "not found: {value}"),
        }
    }
}

impl std::error::Error for CoreError {}
