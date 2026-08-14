use std::path::PathBuf;

use thiserror::Error;
use zeta_workspace::WorkspacePathError;

use crate::SourceRevision;

/// Failure to open, update, query, or verify a rebuildable code index.
#[derive(Debug, Error)]
pub enum CodeIndexError {
    #[error("invalid code-index limits: {0}")]
    InvalidLimits(&'static str),
    #[error("code-index query is invalid: {0}")]
    InvalidQuery(&'static str),
    #[error("code-index identity is invalid: {0}")]
    InvalidIdentity(&'static str),
    #[error("code-index storage belongs to another workspace root")]
    StorageRootMismatch,
    #[error("code-index storage failed: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("filesystem operation failed for {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(transparent)]
    WorkspacePath(#[from] WorkspacePathError),
    #[error("indexed source revision is stale (expected {expected}, observed {observed})")]
    StaleRevision {
        expected: SourceRevision,
        observed: SourceRevision,
    },
    #[error("current source exceeds the code-index verification byte limit")]
    SourceVerificationLimitExceeded,
    #[error("indexed chunk range is not valid for the current UTF-8 source")]
    InvalidChunkRange,
    #[error("indexed chunk line coverage does not match its byte range")]
    ChunkSpanMismatch,
    #[error("indexed chunk content no longer matches its stable identity")]
    ChunkIdentityMismatch,
    #[error("workspace document overlay path is not an authorized relative source path")]
    InvalidOverlayPath,
    #[error("workspace document overlay revision does not advance consistently")]
    OverlayRevisionConflict,
    #[error("a dirty document overlay supersedes the requested persistent source revision")]
    OverlaySupersedesPersistentSource,
}

pub(crate) fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> CodeIndexError {
    CodeIndexError::Io {
        path: path.into(),
        source,
    }
}
