//! Profile-local database runtime, durable state adapters, and rebuildable index leases.

mod dir_index;
mod sqlite;
mod sqlite_runtime;

pub use dir_index::{ClearOutcome, DirIndexKind, DirIndexLease, StateRuntime};
pub use sqlite::{
    SqliteProjectStore, SqliteThreadStore, SqliteTurnChangeStore, SqliteWorkRunStore,
    TurnChangeCommandOutcome,
};
pub use sqlite_runtime::{SqliteDurability, open_in_memory_database, open_sqlite_database};

#[cfg(test)]
#[path = "sqlite_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "work_coordination_store_tests.rs"]
mod work_coordination_store_tests;

#[cfg(test)]
#[path = "project_store_tests.rs"]
mod project_store_tests;
