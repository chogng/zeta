//! Incremental, presentation-independent source syntax analysis.

mod document;
mod error;
mod language;
mod snapshot;

pub use document::{DocumentRevision, SyntaxDocument, SyntaxEdit};
pub use error::SyntaxError;
pub use language::SyntaxLanguage;
pub use snapshot::{
    AnalysisLimits, DocumentSymbol, DocumentSymbolKind, FoldingRange, SyntaxDiagnostic,
    SyntaxPoint, SyntaxRange, SyntaxSnapshot, SyntaxToken, SyntaxTokenKind,
};

#[cfg(test)]
#[path = "syntax_tests.rs"]
mod tests;
