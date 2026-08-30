//! Profile-local database runtime, durable state adapters, and rebuildable index leases.

mod dir_index;
mod sqlite;
mod sqlite_runtime;

pub use dir_index::{ClearOutcome, DirIndexKind, DirIndexLease, StateRuntime};
pub use sqlite::{SqliteThreadStore, SqliteTurnChangeStore, TurnChangeCommandOutcome};
pub use sqlite_runtime::{SqliteDurability, open_in_memory_database, open_sqlite_database};

#[cfg(test)]
#[path = "sqlite_tests.rs"]
mod tests;
