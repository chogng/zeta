//! Structured Git repository operations for Zeta hosts.
//!
//! This crate owns invocation and parsing of the system Git executable. It does not own
//! App Server repository lifecycles, model tool authorization, or desktop presentation.

mod client;
mod content;
mod error;
mod fsmonitor;
mod graph;
mod history;
mod info;
mod mutation;
mod patch;
mod path;
mod repository;
mod status;
mod text_diff;

pub use client::GitClient;
pub use client::GitExecutionLimits;
pub use content::GitChangeFile;
pub use content::GitChangeFileComparison;
pub use content::GitFileRevision;
pub use error::GitError;
pub use error::GitResult;
pub use graph::GitGraph;
pub use graph::GitGraphCursor;
pub use graph::GitReference;
pub use graph::GitReferenceKind;
pub use history::GitCommitChange;
pub use history::GitCommitFile;
pub use info::GitBranch;
pub use info::GitCommitSummary;
pub use info::GitRemote;
pub use info::GitRemoteIdentity;
pub use info::GitRemoteProvider;
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
pub use text_diff::GitDiffStatistics;
pub use text_diff::GitTextDiff;
pub use text_diff::GitTextDiffLimits;
pub use text_diff::GitTextDiffSnapshot;

#[cfg(test)]
#[path = "test_support.rs"]
mod test_support;
