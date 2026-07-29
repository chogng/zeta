//! Structured Git repository operations for Zeta hosts.
//!
//! This crate owns invocation and parsing of the system Git executable. It does not own
//! App Server repository lifecycles, model tool authorization, or desktop presentation.

mod client;
mod error;
mod fsmonitor;
mod info;
mod mutation;
mod patch;
mod path;
mod repository;
mod status;

pub use client::GitClient;
pub use client::GitExecutionLimits;
pub use error::GitError;
pub use error::GitResult;
pub use info::GitBranch;
pub use info::GitCommitSummary;
pub use info::GitRemote;
pub use mutation::GitCommitRequest;
pub use mutation::GitCommitResult;
pub use mutation::GitPathspecSet;
pub use patch::GitPatchDirection;
pub use patch::GitPatchDisposition;
pub use patch::GitPatchExecution;
pub use patch::GitPatchRequest;
pub use patch::GitPatchResult;
pub use patch::extract_patch_paths;
pub use repository::GitRepository;
pub use repository::GitRepositoryKind;
pub use status::GitChangeStatus;
pub use status::GitHead;
pub use status::GitRepositoryChange;
pub use status::GitRepositorySnapshot;
pub use status::GitSubmoduleState;
pub use status::GitUpstream;

#[cfg(test)]
#[path = "test_support.rs"]
mod test_support;
