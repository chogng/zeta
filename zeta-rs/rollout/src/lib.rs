//! Composition boundary for one local authoritative Session and Thread event-history repository.
//!
//! This crate owns no event format or domain reducer. It opens the typed storage adapters as one
//! repository and recovers the runtime in dependency order: every Thread before every Session.

mod error;
mod repository;

pub use error::RolloutError;
pub use repository::RolloutRepository;

#[cfg(test)]
#[path = "repository_tests.rs"]
mod tests;
