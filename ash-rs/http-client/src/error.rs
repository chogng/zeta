use std::fmt;

/// A safe failure produced while configuring or executing a raw HTTP request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HttpClientError {
    InvalidRequest(String),
    InvalidConfiguration(String),
    Transport(String),
}

impl fmt::Display for HttpClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => write!(formatter, "invalid HTTP request: {message}"),
            Self::InvalidConfiguration(message) => {
                write!(formatter, "invalid HTTP client configuration: {message}")
            }
            Self::Transport(message) => write!(formatter, "HTTP transport failed: {message}"),
        }
    }
}

impl std::error::Error for HttpClientError {}
