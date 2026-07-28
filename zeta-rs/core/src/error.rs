use std::fmt;
use zeta_session_store::SessionStoreError;
use zeta_thread_store::ThreadStoreError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoreError {
    Cancelled(String),
    Context(String),
    Execution(String),
    InvalidTransition { from: String, to: String },
    InvalidInput(String),
    Journal(String),
    Model(String),
    NotFound(String),
    Policy(String),
    PolicyCircuitBreaker(String),
    CommandConflict,
    SessionStore(SessionStoreError),
    ThreadStore(ThreadStoreError),
}

impl fmt::Display for CoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled(message) => write!(formatter, "cancelled: {message}"),
            Self::Context(message) => write!(formatter, "context error: {message}"),
            Self::Execution(message) => write!(formatter, "execution error: {message}"),
            Self::InvalidTransition { from, to } => {
                write!(formatter, "cannot transition from {from} to {to}")
            }
            Self::InvalidInput(message) => write!(formatter, "invalid input: {message}"),
            Self::Journal(message) => write!(formatter, "journal error: {message}"),
            Self::Model(message) => write!(formatter, "model error: {message}"),
            Self::NotFound(value) => write!(formatter, "not found: {value}"),
            Self::Policy(message) => write!(formatter, "policy error: {message}"),
            Self::PolicyCircuitBreaker(message) => {
                write!(formatter, "policy circuit breaker: {message}")
            }
            Self::CommandConflict => formatter.write_str("command ID conflict"),
            Self::SessionStore(error) => error.fmt(formatter),
            Self::ThreadStore(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for CoreError {}

impl From<ThreadStoreError> for CoreError {
    fn from(error: ThreadStoreError) -> Self {
        Self::ThreadStore(error)
    }
}

impl From<SessionStoreError> for CoreError {
    fn from(error: SessionStoreError) -> Self {
        Self::SessionStore(error)
    }
}
