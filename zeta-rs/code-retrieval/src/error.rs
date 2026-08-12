use thiserror::Error;

/// Failure to validate or execute the required local side of code retrieval.
#[derive(Debug, Error)]
pub enum CodeRetrievalError {
    #[error("code-retrieval query is invalid: {0}")]
    InvalidQuery(&'static str),
    #[error("local and cloud retrieval sources belong to different workspace roots")]
    RootMismatch,
    #[error("local code-index retrieval failed: {0}")]
    LocalIndex(#[from] zeta_code_index::CodeIndexError),
}
