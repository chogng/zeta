//! Explicit, durable cloud projection for a workspace-side code index.
//!
//! This crate owns egress grants, provider contracts, publication/deletion state, and the
//! publication of Workspace-produced chunks. Scanning, ignore semantics, chunking, chunk identity,
//! and current-source verification remain in `zeta-code-index`; remote services never receive
//! authority to traverse or rechunk the codebase.

mod controller;
mod error;
mod provider;
mod store;
mod types;

pub use controller::CloudCodeIndexController;
pub use error::CloudCodeIndexError;
pub use error::CloudCodeIndexProviderError;
pub use provider::CloudCodeIndexProvider;
pub use provider::CloudCodeIndexProviderRegistry;
pub use types::CloudCodeIndexCandidate;
pub use types::CloudCodeIndexCapabilities;
pub use types::CloudCodeIndexDeletionSupport;
pub use types::CloudCodeIndexDestination;
pub use types::CloudCodeIndexGrant;
pub use types::CloudCodeIndexGrantId;
pub use types::CloudCodeIndexLimitDisposition;
pub use types::CloudCodeIndexPreview;
pub use types::CloudCodeIndexProviderId;
pub use types::CloudCodeIndexPublication;
pub use types::CloudCodeIndexPublicationRequest;
pub use types::CloudCodeIndexQuery;
pub use types::CloudCodeIndexQueryRequest;
pub use types::CloudCodeIndexQueryResult;
pub use types::CloudCodeIndexSelection;
pub use types::CloudCodeIndexState;
pub use types::CloudCodeIndexStatus;
pub use types::CloudCodeIndexStorage;
pub use types::CodeIndexDeploymentMode;

#[cfg(test)]
#[path = "cloud_index_tests.rs"]
mod tests;
