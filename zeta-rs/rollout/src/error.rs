use std::fmt;
use zeta_core::CoreError;
use zeta_session_store::SessionStoreError;
use zeta_thread_store::ThreadStoreError;

/// Failure while opening or recovering a local rollout repository.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RolloutError {
    Core(CoreError),
    SessionStore(SessionStoreError),
    ThreadStore(ThreadStoreError),
}

impl fmt::Display for RolloutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(error) => error.fmt(formatter),
            Self::SessionStore(error) => error.fmt(formatter),
            Self::ThreadStore(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RolloutError {}

impl From<CoreError> for RolloutError {
    fn from(error: CoreError) -> Self {
        Self::Core(error)
    }
}

impl From<SessionStoreError> for RolloutError {
    fn from(error: SessionStoreError) -> Self {
        Self::SessionStore(error)
    }
}

impl From<ThreadStoreError> for RolloutError {
    fn from(error: ThreadStoreError) -> Self {
        Self::ThreadStore(error)
    }
}
