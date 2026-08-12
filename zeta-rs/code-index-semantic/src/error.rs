use std::fmt;

use thiserror::Error;

/// Redacted failure returned by a vector-store implementation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeIndexVectorStoreError {
    message: String,
}

impl CodeIndexVectorStoreError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for CodeIndexVectorStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for CodeIndexVectorStoreError {}

/// Failure to synchronize or query one local semantic code-index generation.
#[derive(Debug, Error)]
pub enum CodeIndexSemanticError {
    #[error("semantic code-index input is invalid: {0}")]
    InvalidInput(&'static str),
    #[error("the local lexical code index has not published a generation yet")]
    IndexNotReady,
    #[error("semantic code-index operation was cancelled")]
    Cancelled,
    #[error("semantic model returned an invalid response: {0}")]
    InvalidModelResponse(&'static str),
    #[error("local code-index access failed: {0}")]
    LocalIndex(#[from] zeta_code_index::CodeIndexError),
    #[error("semantic model invocation failed: {0}")]
    Model(#[from] zeta_model_provider::ModelProviderError),
    #[error("semantic vector store failed: {0}")]
    VectorStore(#[from] CodeIndexVectorStoreError),
}
