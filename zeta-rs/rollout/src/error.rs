use std::fmt;
use zeta_core::CoreError;
use zeta_session_store::SessionStoreError;
use zeta_thread_store::ThreadStoreError;

/// Failure while opening or recovering the local authoritative state repository.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalStateError {
    Core(CoreError),
    SessionStore(SessionStoreError),
    ThreadStore(ThreadStoreError),
}

impl fmt::Display for LocalStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => error.fmt(formatter),
            Self::SessionStore(error) => error.fmt(formatter),
            Self::ThreadStore(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for LocalStateError {}

impl From<CoreError> for LocalStateError {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}

impl From<SessionStoreError> for LocalStateError {
    fn from(error: SessionStoreError) -> Self {
        Self::SessionStore(error)
    }
}

impl From<ThreadStoreError> for LocalStateError {
    fn from(error: ThreadStoreError) -> Self {
        Self::ThreadStore(error)
    }
}
