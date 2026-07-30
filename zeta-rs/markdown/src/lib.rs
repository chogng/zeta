//! Native Markdown parsing, layout, and presentation for Zeta product hosts.
//!
//! This crate owns a bounded CommonMark document projection and its presentation as `zeta-ui`
//! primitives. It does not own message identity, scrolling input, link activation, persistence,
//! network image loading, syntax highlighting, or product lifecycle.

mod component;
mod document;
mod inline_layout;
mod style;
mod table;
mod table_layout;

pub use component::{Markdown, MarkdownLayoutEngine, MarkdownLink};
pub use document::{MarkdownDocument, MarkdownError};
pub use style::MarkdownStyle;

#[cfg(test)]
#[path = "lib_tests.rs"]
mod tests;
