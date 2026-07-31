use std::ops::Range;

use zeta_ui::Color;

/// Stable editor presentation role independent from syntax or semantic token provenance.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CodeEditorTokenRole {
    Attribute,
    Comment,
    Constant,
    Constructor,
    Embedded,
    Function,
    Keyword,
    Label,
    Module,
    Number,
    Operator,
    Property,
    Punctuation,
    Regexp,
    String,
    Type,
    Variable,
}

impl CodeEditorTokenRole {
    const COUNT: usize = 17;

    const fn index(self) -> usize {
        self as usize
    }
}

/// Resolved syntax-role colors supplied by the current editor theme snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeEditorSyntaxPalette {
    colors: [Color; CodeEditorTokenRole::COUNT],
}

impl CodeEditorSyntaxPalette {
    pub const fn uniform(color: Color) -> Self {
        Self {
            colors: [color; CodeEditorTokenRole::COUNT],
        }
    }

    pub const fn with_color(mut self, role: CodeEditorTokenRole, color: Color) -> Self {
        self.colors[role.index()] = color;
        self
    }

    pub const fn color(&self, role: CodeEditorTokenRole) -> Color {
        self.colors[role.index()]
    }
}

/// One role-classified UTF-8 byte range within a single code line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeEditorSyntaxToken {
    pub range: Range<usize>,
    pub role: CodeEditorTokenRole,
}

impl CodeEditorSyntaxToken {
    pub const fn new(range: Range<usize>, role: CodeEditorTokenRole) -> Self {
        Self { range, role }
    }
}

/// Synchronous syntax-token projection used to build a document presentation snapshot.
///
/// Implementations must return ranges relative to the provided line and should avoid I/O. Hosts
/// with asynchronous parsers should compute tokens off-thread and apply them only when the
/// document revision still matches.
pub trait CodeEditorSyntaxHighlighter {
    fn highlight_line(&self, line_number: usize, text: &str) -> Vec<CodeEditorSyntaxToken>;
}
