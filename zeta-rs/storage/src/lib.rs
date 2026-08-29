//! Durable typed event streams, rebuildable projections, and aggregate writer leases.

mod lease;
mod sqlite;

pub use lease::FileLease;
pub use lease::LeaseDirectory;
pub use sqlite::{
    SqliteSessionStore, SqliteThreadStore, SqliteTurnChangeStore, TurnChangeCommandOutcome,
};

#[cfg(test)]
#[path = "sqlite_tests.rs"]
mod tests;
