//! Native code and diff editor presentation shared by Zeta product hosts.
//!
//! This crate owns code-row projection, viewport geometry, line-number and decoration paint, and
//! side-by-side and unified diff composition. It owns editor-local syntax analysis, but does not
//! own files, tabs, persistence, or product input routing.

mod code_editor;
mod diff_editor;
mod multi_diff_editor;

pub use code_editor::{
    CodeEditor, CodeEditorCaseSensitivity, CodeEditorCommand, CodeEditorComposition,
    CodeEditorDiagnostic, CodeEditorDiagnosticPalette, CodeEditorDiagnosticSeverity,
    CodeEditorDocument, CodeEditorFoldControl, CodeEditorFoldState, CodeEditorFoldingRange,
    CodeEditorHeader, CodeEditorIndentation, CodeEditorInlineHighlight, CodeEditorLanguage,
    CodeEditorLineWrapping, CodeEditorLocation, CodeEditorNavigation, CodeEditorPalette,
    CodeEditorPosition, CodeEditorPresentation, CodeEditorRevision, CodeEditorRow,
    CodeEditorRowSource, CodeEditorSearchMatch, CodeEditorSearchQuery, CodeEditorSelection,
    CodeEditorSelectionMode, CodeEditorStyle, CodeEditorSyntaxPalette, CodeEditorSyntaxToken,
    CodeEditorTextEdit, CodeEditorTokenRole, CodeEditorViewport,
};
pub use diff_editor::{
    DiffEditor, DiffEditorDocument, DiffEditorFoldControl, DiffEditorFoldState, DiffEditorLabels,
    DiffEditorLocation, DiffEditorPalette, DiffEditorPresentation, DiffEditorSide, DiffEditorState,
    DiffEditorStyle,
};
pub use multi_diff_editor::{
    MultiDiffEditor, MultiDiffEditorFoldControl, MultiDiffEditorItem, MultiDiffEditorItemIdentity,
    MultiDiffEditorLayout, MultiDiffEditorPalette, MultiDiffEditorStyle,
};
