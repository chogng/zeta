use std::fmt;

/// A safe, provider-neutral failure from request execution or wire framing.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClientError {
    InvalidRequest(String),
    Transport(String),
    InvalidResponse(String),
    Framing(String),
}

impl fmt::Display for ClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(formatter, "invalid client request: {message}"),
            Self::Transport(message) => write!(formatter, "HTTP client failed: {message}"),
            Self::InvalidResponse(message) => {
                write!(formatter, "invalid client response: {message}")
            }
            Self::Framing(message) => write!(formatter, "invalid response framing: {message}"),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<zeta_http_client::HttpClientError> for ClientError {
    fn from(error: zeta_http_client::HttpClientError) -> Self {
        match error {
            zeta_http_client::HttpClientError::InvalidRequest(message) => {
                Self::InvalidRequest(message)
            }
            zeta_http_client::HttpClientError::InvalidConfiguration(message) => {
                Self::Transport(message)
            }
            zeta_http_client::HttpClientError::Transport(message) => Self::Transport(message),
        }
    }
}
