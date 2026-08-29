//! Profile-local database runtime, durable state adapters, and rebuildable index leases.

mod sqlite;
mod sqlite_runtime;
mod workspace_index;

pub use sqlite::{
    SqliteSessionStore, SqliteThreadStore, SqliteTurnChangeStore, TurnChangeCommandOutcome,
};
pub use sqlite_runtime::{SqliteDurability, open_in_memory_database, open_sqlite_database};
pub use workspace_index::{ClearOutcome, StateRuntime, WorkspaceIndexKind, WorkspaceIndexLease};

#[cfg(test)]
#[path = "sqlite_tests.rs"]
mod tests;
