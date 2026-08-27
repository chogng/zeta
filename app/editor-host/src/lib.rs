//! Editor host state and auxiliary presentation.
//!
//! This crate owns editor-specific retained UI state and result presentation. File I/O,
//! save-conflict policy, Tab lifetime, App Server requests, and host event routing remain
//! in the app composition layer.

mod auto_scroll;
mod diagnostics;
mod language_features;
mod search;
mod style;

pub use auto_scroll::{FileEditorAutoScrollDirection, FileEditorAutoScrollState};
pub use diagnostics::FileEditorDiagnosticTooltip;
pub use language_features::{LanguageCompletionPopover, LanguageHoverPopover};
pub use search::{FileEditorSearchMode, FileEditorSearchState};
pub use style::EditorOverlayStyle;
