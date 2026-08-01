//! Native code and diff editor presentation shared by Zeta product hosts.
//!
//! This crate owns code-row projection, viewport geometry, line-number and decoration paint, and
//! side-by-side and unified diff composition. It owns editor-local syntax analysis, but does not
//! own files, tabs, persistence, or product input routing.

mod code_editor;
mod diff_editor;
mod multi_diff_editor;

pub use code_editor::{
    CodeEditor, CodeEditorCommand, CodeEditorComposition, CodeEditorDocument, CodeEditorHeader,
    CodeEditorInlineHighlight, CodeEditorLanguage, CodeEditorLocation, CodeEditorPalette,
    CodeEditorPosition, CodeEditorPresentation, CodeEditorRow, CodeEditorRowSource,
    CodeEditorSelection, CodeEditorSelectionMode, CodeEditorStyle, CodeEditorSyntaxPalette,
    CodeEditorSyntaxToken, CodeEditorTokenRole, CodeEditorViewport,
};
pub use diff_editor::{
    DiffEditor, DiffEditorDocument, DiffEditorFoldControl, DiffEditorFoldState, DiffEditorLabels,
    DiffEditorLocation, DiffEditorPalette, DiffEditorPresentation, DiffEditorSide, DiffEditorState,
    DiffEditorStyle,
};
pub use multi_diff_editor::{
    MultiDiffEditor, MultiDiffEditorFoldControl, MultiDiffEditorItem, MultiDiffEditorLayout,
    MultiDiffEditorPalette, MultiDiffEditorStyle,
};
