//! Native code and diff editor presentation shared by Zeta product hosts.
//!
//! This crate owns code-row projection, viewport geometry, line-number and decoration paint, and
//! side-by-side diff composition. It does not own files, tabs, editing commands, syntax services,
//! persistence, or product input routing.

mod code_editor;
mod diff_editor;
mod multi_diff_editor;

pub use code_editor::{
    CodeEditor, CodeEditorCommand, CodeEditorComposition, CodeEditorDocument, CodeEditorHeader,
    CodeEditorInlineHighlight, CodeEditorLocation, CodeEditorPosition, CodeEditorRow,
    CodeEditorRowSource, CodeEditorSelection, CodeEditorSelectionMode, CodeEditorStyle,
    CodeEditorSyntaxHighlighter, CodeEditorSyntaxToken, CodeEditorViewport,
};
pub use diff_editor::{
    DiffEditor, DiffEditorLabels, DiffEditorLocation, DiffEditorSide, DiffEditorState,
    DiffEditorStyle,
};
pub use multi_diff_editor::{MultiDiffEditor, MultiDiffEditorItem, MultiDiffEditorStyle};
