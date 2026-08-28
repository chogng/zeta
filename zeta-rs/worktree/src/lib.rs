//! Git worktree discovery and Codex-compatible ownership metadata.
//!
//! The crate resolves an existing checkout into explicit switch targets. It does not replace the
//! caller's workspace, create sessions, or own product presentation.

mod manager;
mod metadata;
mod settings;

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
