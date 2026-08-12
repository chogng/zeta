use std::num::NonZeroUsize;

use zeta_code_index::ChunkReference;
use zeta_code_index::IndexedLanguage;
use zeta_code_index::MaterializedChunk;
use zeta_model_provider::EmbeddingVector;

use crate::CodeIndexServiceError;

macro_rules! text_identity {
    ($name:ident, $message:literal) => {
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, CodeIndexServiceError> {
                let value = value.into();
                if value.is_empty() || value.len() > 256 || value.chars().any(char::is_control) {
                    return Err(CodeIndexServiceError::InvalidInput($message));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

text_identity!(
    CodeIndexCollectionId,
    "collection identity must be 1..=256 bytes without control characters"
);
text_identity!(
    CodeIndexGenerationId,
    "generation identity must be 1..=256 bytes without control characters"
);

/// Complete replacement of one remote collection generation using Workspace-produced chunks.
#[derive(Clone, Debug)]
pub struct CodeIndexSemanticPublication {
    pub collection: CodeIndexCollectionId,
    pub generation: CodeIndexGenerationId,
    pub chunks: Vec<MaterializedChunk>,
}

/// Bounded semantic query against one exact remote generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeIndexSemanticQuery {
    pub collection: CodeIndexCollectionId,
    pub generation: CodeIndexGenerationId,
    text: String,
    result_limit: NonZeroUsize,
}

impl CodeIndexSemanticQuery {
    pub fn new(
        collection: CodeIndexCollectionId,
        generation: CodeIndexGenerationId,
        text: impl Into<String>,
        result_limit: NonZeroUsize,
    ) -> Result<Self, CodeIndexServiceError> {
        let text = text.into();
        if text.trim().is_empty() || text.len() > 8 * 1024 {
            return Err(CodeIndexServiceError::InvalidInput(
                "query must contain 1..=8192 bytes of non-whitespace text",
            ));
        }
        if result_limit.get() > 100 {
            return Err(CodeIndexServiceError::InvalidInput(
                "query result limit must not exceed 100",
            ));
        }
        Ok(Self {
            collection,
            generation,
            text,
            result_limit,
        })
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn result_limit(&self) -> NonZeroUsize {
        self.result_limit
    }
}

/// Final provider-ranked references for one exact semantic generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeIndexSemanticQueryResult {
    pub generation: CodeIndexGenerationId,
    pub candidates: Vec<ChunkReference>,
}

/// One Workspace-produced chunk plus its model embedding stored by the remote service.
#[derive(Clone, Debug, PartialEq)]
pub struct EmbeddedCodeChunk {
    pub reference: ChunkReference,
    pub language: IndexedLanguage,
    pub content: String,
    pub embedding: EmbeddingVector,
}

/// One vector-recall candidate in vector-store relevance order.
#[derive(Clone, Debug, PartialEq)]
pub struct VectorSearchHit {
    pub chunk: EmbeddedCodeChunk,
    pub similarity: f32,
}
