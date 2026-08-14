//! Rebuildable, workspace-side source-code indexing and lexical retrieval.
//!
//! The crate owns filesystem scanning, stable source/chunk identity, structural chunking,
//! projection persistence, and revision-bound retrieval. Product hosts retain workspace
//! authorization, watcher lifecycle, remote-data policy, and transport adaptation.

mod chunker;
mod error;
mod index;
mod overlay;
mod scanner;
mod store;
mod store_manifest;
mod types;

pub use error::CodeIndexError;
pub use index::CodeIndex;
pub use types::ChunkContentHash;
pub use types::ChunkKey;
pub use types::ChunkReference;
pub use types::ChunkSpan;
pub use types::CodeIndexLimits;
pub use types::CodeIndexManifest;
pub use types::CodeIndexOverlayDocument;
pub use types::CodeIndexOverlaySnapshot;
pub use types::CodeIndexQuery;
pub use types::CodeIndexSnapshot;
pub use types::CodeIndexStorage;
pub use types::IndexRootId;
pub use types::IndexedChunkReference;
pub use types::IndexedLanguage;
pub use types::IndexedSourceReference;
pub use types::MaterializedChunk;
pub use types::MaterializedExcerpt;
pub use types::MaterializedOverlayDocument;
pub use types::MaterializedSource;
pub use types::RefreshOutcome;
pub use types::SearchHit;
pub use types::SourceExcerptReference;
pub use types::SourceRevision;

#[cfg(test)]
#[path = "code_index_tests.rs"]
mod tests;
