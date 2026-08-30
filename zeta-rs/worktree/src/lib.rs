//! Managed Git worktrees and durable Thread directory bindings.
//!
//! The crate discovers existing checkouts, provisions isolated Thread worktrees from immutable
//! baselines, and cleans them only after the change ledger proves every ChangeSet settled.

mod binding;
mod manager;
mod metadata;
mod settings;

pub use manager::ThreadRepositoryBinding;
pub use manager::ThreadWorktreeBinding;
pub use manager::ThreadWorktreeCleanupEligibility;
pub use manager::ThreadWorktreeKind;
pub use manager::ThreadWorktreeProvisionRequest;
pub use manager::ThreadWorktreeSource;
pub use manager::ThreadWorktreeTarget;
pub use manager::Worktree;
pub use manager::WorktreeAvailability;
pub use manager::WorktreeKind;
pub use manager::WorktreeManager;
pub use manager::WorktreeOwner;
pub use manager::WorktreeSelector;
pub use settings::DEFAULT_WORKTREE_KEEP_COUNT;
pub use settings::WorktreeSettings;

#[cfg(test)]
#[path = "worktree_tests.rs"]
mod tests;
