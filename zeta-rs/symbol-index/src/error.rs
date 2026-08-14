use std::path::PathBuf;

use thiserror::Error;

/// Failure to open, reconcile, query, or persist a rebuildable symbol index.
#[derive(Debug, Error)]
pub enum SymbolIndexError {
    #[error("invalid symbol-index limits: {0}")]
    InvalidLimits(&'static str),
    #[error("symbol-index query exceeds its byte limit")]
    QueryTooLarge,
    #[error("symbol-index storage belongs to another workspace root")]
    StorageRootMismatch,
    #[error("symbol-index source projection does not match the current code-index root")]
    SourceRootMismatch,
    #[error("symbol-index storage contains an unknown symbol kind: {0}")]
    InvalidStoredSymbolKind(String),
    #[error("symbol-index storage failed: {0}")]
    Storage(#[from] rusqlite::Error),
    #[error("symbol extraction failed for {}: {source}", path.display())]
    Syntax {
        path: PathBuf,
        #[source]
        source: zeta_syntax::SyntaxError,
    },
    #[error("filesystem operation failed for {}: {source}", path.display())]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("symbol-index operation was cancelled: {0}")]
    Cancelled(String),
    #[error(transparent)]
    CodeIndex(#[from] zeta_code_index::CodeIndexError),
}

pub(crate) fn io_error(path: impl Into<PathBuf>, source: std::io::Error) -> SymbolIndexError {
    SymbolIndexError::Io {
        path: path.into(),
        source,
    }
}
