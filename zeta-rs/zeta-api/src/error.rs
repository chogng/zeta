use std::fmt;
use zeta_client::ClientError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ApiError {
    InvalidRequest(String),
    Transport(String),
    HttpStatus(u16),
    InvalidResponse(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(formatter, "invalid model request: {message}"),
            Self::Transport(message) => write!(formatter, "model transport failed: {message}"),
            Self::HttpStatus(status) => write!(formatter, "model API returned HTTP {status}"),
            Self::InvalidResponse(message) => {
                write!(formatter, "invalid model response: {message}")
            }
        }
    }
}

impl std::error::Error for ApiError {}

impl From<ClientError> for ApiError {
    fn from(error: ClientError) -> Self {
        match error {
            ClientError::InvalidRequest(message) => Self::InvalidRequest(message),
            ClientError::Transport(message) => Self::Transport(message),
            ClientError::InvalidResponse(message) | ClientError::Framing(message) => {
                Self::InvalidResponse(message)
            }
        }
    }
}
