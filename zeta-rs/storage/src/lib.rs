//! Durable rollout logs, rebuildable SQLite projections, and Thread writer leases.

mod idempotency;
mod lease;
mod rollout;

pub use idempotency::FileIdempotencyLedger;
pub use lease::FileThreadLease;
pub use lease::ThreadLeaseDirectory;
pub use rollout::RolloutLog;
pub use rollout::ThreadRolloutStore;

#[cfg(test)]
#[path = "rollout_tests.rs"]
mod tests;
