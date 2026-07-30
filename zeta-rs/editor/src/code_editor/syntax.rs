use std::ops::Range;

use zeta_ui::Color;

/// One syntax-colored UTF-8 byte range within a single code line.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeEditorSyntaxToken {
    pub range: Range<usize>,
    pub color: Color,
}

impl CodeEditorSyntaxToken {
    pub const fn new(range: Range<usize>, color: Color) -> Self {
        Self { range, color }
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
