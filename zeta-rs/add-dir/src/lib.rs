//! Additional-directory access scope for one primary working directory.
//!
//! The crate distinguishes project identity from extra filesystem access. It owns directory
//! source lifetimes, canonical-root deduplication, and source-derived contribution policy, while
//! hosts remain responsible for trust decisions, persistence, runtime activation, and UI.

mod contributions;
mod scope;

pub use contributions::{
    AdditionalDirectoryContribution, AdditionalDirectoryContributionPolicy,
    AdditionalInstructionsPolicy,
};
pub use scope::{
    AdditionalDirectory, AdditionalDirectorySource, DirectoryAccessScope, DirectoryScopeError,
    DirectoryScopeMutation,
};
