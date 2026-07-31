//! Native code and diff editor presentation shared by Zeta product hosts.
//!
//! This crate owns code-row projection, viewport geometry, line-number and decoration paint, and
//! side-by-side and unified diff composition. It does not own files, tabs, editing commands,
//! syntax services, persistence, or product input routing.

mod code_editor;
mod diff_editor;
mod multi_diff_editor;

pub use code_editor::{
    CodeEditor, CodeEditorCommand, CodeEditorComposition, CodeEditorDocument, CodeEditorHeader,
    CodeEditorInlineHighlight, CodeEditorLocation, CodeEditorPalette, CodeEditorPosition,
    CodeEditorPresentation, CodeEditorRow, CodeEditorRowSource, CodeEditorSelection,
    CodeEditorSelectionMode, CodeEditorStyle, CodeEditorSyntaxHighlighter, CodeEditorSyntaxPalette,
    CodeEditorSyntaxToken, CodeEditorTokenRole, CodeEditorViewport,
};
pub use diff_editor::{
    DiffEditor, DiffEditorFoldControl, DiffEditorFoldState, DiffEditorLabels, DiffEditorLocation,
    DiffEditorPalette, DiffEditorPresentation, DiffEditorSide, DiffEditorState, DiffEditorStyle,
};
pub use multi_diff_editor::{
    MultiDiffEditor, MultiDiffEditorFoldControl, MultiDiffEditorItem, MultiDiffEditorLayout,
    MultiDiffEditorPalette, MultiDiffEditorStyle,
};
