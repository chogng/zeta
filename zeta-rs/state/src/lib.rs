//! Profile-local database runtime, durable state adapters, and rebuildable index leases.

mod dir_index;
mod issue_cache;
pub use issue_cache::CachedIssuePage;
pub use issue_cache::IssueCacheKey;
pub use issue_cache::SqliteIssueCache;
mod sqlite;
mod sqlite_runtime;

pub use dir_index::{ClearOutcome, DirIndexKind, DirIndexLease, StateRuntime};
pub use sqlite::{
    SqliteProjectStore, SqliteThreadStore, SqliteTurnChangeStore, TurnChangeCommandOutcome,
};
pub use sqlite_runtime::{SqliteDurability, open_in_memory_database, open_sqlite_database};

#[cfg(test)]
#[path = "sqlite_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "project_store_tests.rs"]
mod project_store_tests;
