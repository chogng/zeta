//! Durable, user-controlled memories without model or product-host dependencies.

mod model;
mod service;
mod store;

pub use model::Memory;
pub use model::MemoryDeleteResult;
pub use model::MemoryId;
pub use model::MemoryListPage;
pub use model::MemoryMutationDisposition;
pub use model::MemoryMutationResult;
pub use model::MemoryScope;
pub use model::MemorySearchMatch;
pub use model::MemorySearchPage;
pub use model::MemorySource;
pub use model::MemorySummary;
pub use service::AddMemoryRequest;
pub use service::DeleteMemoryRequest;
pub use service::ListMemoriesRequest;
pub use service::Memories;
pub use service::MemoryError;
pub use service::ReadMemoryRequest;
pub use service::SearchMemoriesRequest;
pub use store::MemoryAddCommit;
pub use store::MemoryDeleteCommit;
pub use store::MemoryStore;
pub use store::MemoryStoreError;
pub use store::MemoryStoreListRequest;
pub use store::MemoryStorePage;
pub use store::MemoryStoreSearchRequest;

#[cfg(test)]
#[path = "memories_tests.rs"]
mod tests;
