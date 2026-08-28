//! Workspace access authority shared by model context and filesystem-capable tools.
//!
//! Hosts resolve trust before adding a root. This crate owns the effective multi-root access
//! scope, source-bound authorization leases, monotonic revisions, and immutable snapshots without
//! owning Session storage, RPC, command parsing, or tool execution.

mod additional_directory;
#[path = "access/authority.rs"]
mod authority;
mod contributions;
#[path = "access/error.rs"]
mod error;
mod permissions;
#[path = "access/snapshot.rs"]
mod snapshot;

pub use additional_directory::AdditionalDirectory;
pub use additional_directory::AdditionalDirectorySource;
pub use authority::WorkspaceAccessAuthority;
pub use contributions::AdditionalDirectoryContribution;
pub use contributions::AdditionalDirectoryContributionPolicy;
pub use error::WorkspaceAccessError;
pub use permissions::AdditionalDirectoryPermission;
pub use permissions::AdditionalDirectoryPermissions;
pub use permissions::AdditionalDirectoryPermissionsError;
pub use snapshot::WorkspaceAccessMutation;
pub use snapshot::WorkspaceAccessRevision;
pub use snapshot::WorkspaceAccessSnapshot;
