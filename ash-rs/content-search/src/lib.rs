//! Bounded cross-file text and regular-expression search within one dir.
//!
//! The crate owns query validation, frozen-ripgrep process execution, result parsing, and
//! owner-bound search jobs. Product clients adapt their own transport DTOs to these domain types.

mod service;
mod types;

pub use service::ContentSearchService;
pub use types::{
    ContentSearchCaseSensitivity, ContentSearchError, ContentSearchMatch, ContentSearchMatchRange,
    ContentSearchOwner, ContentSearchPage, ContentSearchPattern, ContentSearchQuery,
};
