//! Durable typed event streams, rebuildable projections, and aggregate writer leases.

mod event_stream;
mod lease;
mod rollout;
mod session_rollout;

pub use lease::FileLease;
pub use lease::LeaseDirectory;
pub use rollout::ThreadRolloutStore;
pub use session_rollout::SessionRolloutStore;

#[cfg(test)]
#[path = "rollout_tests.rs"]
mod tests;
