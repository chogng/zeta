use std::path::PathBuf;

use thiserror::Error;

/// Non-secret failure returned by one concrete cloud code-index provider adapter.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("{message}")]
pub struct CloudCodeIndexProviderError {
    message: String,
}

impl CloudCodeIndexProviderError {
    /// Creates a redacted provider failure safe for local diagnostics.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Failure to preview, authorize, publish, recover, or delete a cloud index projection.
#[derive(Debug, Error)]
pub enum CloudCodeIndexError {
    #[error("cloud code-index input is invalid: {0}")]
    InvalidInput(&'static str),
    #[error("cloud code-index storage belongs to another workspace root")]
    StorageRootMismatch,
    #[error("cloud code-index storage schema is incompatible")]
    IncompatibleStorage,
    #[error("another cloud code-index grant must be revoked before this grant can be activated")]
    ConsentConflict,
    #[error("cloud code-index provider is unavailable")]
    ProviderUnavailable,
    #[error("cloud code-index provider does not guarantee idempotent grant deletion")]
    DeletionUnsupported,
    #[error("cloud code-index preview exceeds the grant byte limit")]
    EgressLimitExceeded,
    #[error("local code index has no published generation")]
    LocalIndexNotReady,
    #[error("cloud code-index has no active grant")]
    NoActiveGrant,
    #[error("cloud code-index operation conflicts with the current lifecycle state")]
    InvalidState,
    #[error("cloud code-index provider returned an invalid query result: {0}")]
    InvalidProviderResult(&'static str),
    #[error("cloud code-index provider operation failed: {0}")]
    Provider(#[from] CloudCodeIndexProviderError),
    #[error("local code-index operation failed: {0}")]
    LocalIndex(#[from] zeta_code_index::CodeIndexError),
    #[error("cloud code-index storage failed: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("cloud code-index state serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("filesystem operation failed for {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
