use std::fmt;

/// A sanitized failure produced by WebSocket configuration or I/O.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WebSocketClientError {
    InvalidRequest(String),
    InvalidConfiguration(String),
    ConnectionFailed,
    ProtocolFailed,
    ConnectionClosed,
}

impl fmt::Display for WebSocketClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRequest(message) => {
                write!(formatter, "invalid WebSocket request: {message}")
            }
            Self::InvalidConfiguration(message) => {
                write!(
                    formatter,
                    "invalid WebSocket client configuration: {message}"
                )
            }
            Self::ConnectionFailed => formatter.write_str("WebSocket connection failed"),
            Self::ProtocolFailed => formatter.write_str("WebSocket protocol failed"),
            Self::ConnectionClosed => formatter.write_str("WebSocket connection closed"),
        }
    }
}

impl std::error::Error for WebSocketClientError {}

impl From<zeta_http_client::HttpClientError> for WebSocketClientError {
    fn from(error: zeta_http_client::HttpClientError) -> Self {
        match error {
            zeta_http_client::HttpClientError::InvalidRequest(message) => {
                Self::InvalidRequest(message)
            }
            zeta_http_client::HttpClientError::InvalidConfiguration(message) => {
                Self::InvalidConfiguration(message)
            }
            zeta_http_client::HttpClientError::Transport(_) => Self::ConnectionFailed,
        }
    }
}
