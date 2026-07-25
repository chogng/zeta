use std::fmt;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApiError {
    InvalidRequest(String),
    Transport(String),
    InvalidResponse(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(formatter, "invalid model request: {message}"),
            Self::Transport(message) => write!(formatter, "model transport failed: {message}"),
            Self::InvalidResponse(message) => {
                write!(formatter, "invalid model response: {message}")
            }
        }
    }
}

impl std::error::Error for ApiError {}
