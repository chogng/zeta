//! Bounded cross-file text and regular-expression search within one workspace.
//!
//! The crate owns query validation, frozen-ripgrep process execution, result parsing, and
//! owner-bound search jobs. Product clients adapt their own transport DTOs to these domain types.

mod service;
mod types;

pub use service::WorkspaceSearchService;
pub use types::{
    WorkspaceSearchCaseSensitivity, WorkspaceSearchError, WorkspaceSearchMatch,
    WorkspaceSearchMatchRange, WorkspaceSearchOwner, WorkspaceSearchPage, WorkspaceSearchPattern,
    WorkspaceSearchQuery,
};
