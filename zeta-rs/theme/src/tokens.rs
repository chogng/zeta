//! Stable semantic color-token identifiers consumed by Rust presentation adapters.

pub const FOREGROUND: &str = "foreground";
pub const DESCRIPTION_FOREGROUND: &str = "description.foreground";
pub const MUTED_FOREGROUND: &str = "muted.foreground";
pub const ACCENT_FOREGROUND: &str = "accent.foreground";
pub const ERROR_FOREGROUND: &str = "error.foreground";
pub const WARNING_FOREGROUND: &str = "warning.foreground";
pub const SUCCESS_FOREGROUND: &str = "success.foreground";
pub const BORDER: &str = "border";
pub const WORKBENCH_BACKGROUND: &str = "workbench.background";
pub const EDITOR_BACKGROUND: &str = "editor.background";
pub const EDITOR_FOREGROUND: &str = "editor.foreground";
pub const INPUT_BACKGROUND: &str = "input.background";
pub const SELECTION_BACKGROUND: &str = "selection.background";
pub const LIST_HOVER_BACKGROUND: &str = "list.hoverBackground";
pub const LIST_ACTIVE_SELECTION_BACKGROUND: &str = "list.activeSelectionBackground";
pub const SIDE_BAR_BACKGROUND: &str = "sideBar.background";
pub const SCROLLBAR_SLIDER_BACKGROUND: &str = "scrollbar.sliderBackground";
pub const SCROLLBAR_SLIDER_HOVER_BACKGROUND: &str = "scrollbar.sliderHoverBackground";
pub const SCROLLBAR_SLIDER_ACTIVE_BACKGROUND: &str = "scrollbar.sliderActiveBackground";

pub const DIFF_REMOVED_LINE_BACKGROUND: &str = "diffEditor.removedLineBackground";
pub const DIFF_INSERTED_LINE_BACKGROUND: &str = "diffEditor.insertedLineBackground";
pub const DIFF_REMOVED_TEXT_BACKGROUND: &str = "diffEditor.removedTextBackground";
pub const DIFF_INSERTED_TEXT_BACKGROUND: &str = "diffEditor.insertedTextBackground";
pub const DIFF_MISSING_LINE_BACKGROUND: &str = "diffEditor.missingLineBackground";
pub const DIFF_UNCHANGED_REGION_BACKGROUND: &str = "diffEditor.unchangedRegionBackground";
pub const DIFF_UNCHANGED_REGION_FOREGROUND: &str = "diffEditor.unchangedRegionForeground";
pub const DIFF_REMOVED_LINE_MARKER: &str = "diffEditor.removedLineMarker";
pub const DIFF_INSERTED_LINE_MARKER: &str = "diffEditor.insertedLineMarker";

pub const EDITOR_TOKEN_ATTRIBUTE: &str = "editor.token.attributeForeground";
pub const EDITOR_TOKEN_COMMENT: &str = "editor.token.commentForeground";
pub const EDITOR_TOKEN_CONSTANT: &str = "editor.token.constantForeground";
pub const EDITOR_TOKEN_CONSTRUCTOR: &str = "editor.token.constructorForeground";
pub const EDITOR_TOKEN_EMBEDDED: &str = "editor.token.embeddedForeground";
pub const EDITOR_TOKEN_FUNCTION: &str = "editor.token.functionForeground";
pub const EDITOR_TOKEN_KEYWORD: &str = "editor.token.keywordForeground";
pub const EDITOR_TOKEN_LABEL: &str = "editor.token.labelForeground";
pub const EDITOR_TOKEN_MODULE: &str = "editor.token.moduleForeground";
pub const EDITOR_TOKEN_NUMBER: &str = "editor.token.numberForeground";
pub const EDITOR_TOKEN_OPERATOR: &str = "editor.token.operatorForeground";
pub const EDITOR_TOKEN_PROPERTY: &str = "editor.token.propertyForeground";
pub const EDITOR_TOKEN_PUNCTUATION: &str = "editor.token.punctuationForeground";
pub const EDITOR_TOKEN_REGEXP: &str = "editor.token.regexpForeground";
pub const EDITOR_TOKEN_STRING: &str = "editor.token.stringForeground";
pub const EDITOR_TOKEN_TYPE: &str = "editor.token.typeForeground";
pub const EDITOR_TOKEN_VARIABLE: &str = "editor.token.variableForeground";

pub const TERMINAL_BACKGROUND: &str = "terminal.background";
pub const TERMINAL_FOREGROUND: &str = "terminal.foreground";
pub const TERMINAL_CURSOR_FOREGROUND: &str = "terminal.cursorForeground";
pub const TUI_HIGHLIGHT_FOREGROUND: &str = "tui.highlightForeground";
pub const TERMINAL_ANSI: [&str; 16] = [
    "terminal.ansiBlack",
    "terminal.ansiRed",
    "terminal.ansiGreen",
    "terminal.ansiYellow",
    "terminal.ansiBlue",
    "terminal.ansiMagenta",
    "terminal.ansiCyan",
    "terminal.ansiWhite",
    "terminal.ansiBrightBlack",
    "terminal.ansiBrightRed",
    "terminal.ansiBrightGreen",
    "terminal.ansiBrightYellow",
    "terminal.ansiBrightBlue",
    "terminal.ansiBrightMagenta",
    "terminal.ansiBrightCyan",
    "terminal.ansiBrightWhite",
];
