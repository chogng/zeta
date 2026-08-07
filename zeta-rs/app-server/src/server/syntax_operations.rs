use super::AppServer;
use super::RpcError;
use super::decode;
use super::result;
use serde_json::Value;
use zeta_app_server_protocol::protocol::error::AppServerErrorName;
use zeta_app_server_protocol::protocol::syntax::SyntaxAnalyzeParams;
use zeta_app_server_protocol::protocol::syntax::SyntaxAnalyzeResult;
use zeta_app_server_protocol::protocol::syntax::SyntaxDiagnosticDto;
use zeta_app_server_protocol::protocol::syntax::SyntaxDiagnosticKindDto;
use zeta_app_server_protocol::protocol::syntax::SyntaxFoldingRangeDto;
use zeta_app_server_protocol::protocol::syntax::SyntaxLanguageDto;
use zeta_app_server_protocol::protocol::syntax::SyntaxPositionDto;
use zeta_app_server_protocol::protocol::syntax::SyntaxRangeDto;
use zeta_app_server_protocol::protocol::syntax::SyntaxSymbolDto;
use zeta_app_server_protocol::protocol::syntax::SyntaxSymbolKindDto;
use zeta_app_server_protocol::protocol::syntax::SyntaxTokenDto;
use zeta_app_server_protocol::protocol::syntax::SyntaxTokenKindDto;
use zeta_syntax::DocumentRevision;
use zeta_syntax::DocumentSymbolKind;
use zeta_syntax::SyntaxDiagnosticKind;
use zeta_syntax::SyntaxDocument;
use zeta_syntax::SyntaxError;
use zeta_syntax::SyntaxLanguage;
use zeta_syntax::SyntaxRange;
use zeta_syntax::SyntaxSnapshot;
use zeta_syntax::SyntaxTokenKind;

impl AppServer {
    pub(super) fn syntax_analyze(&self, params: &Value) -> Result<Value, RpcError> {
        let params: SyntaxAnalyzeParams = decode(params)?;
        let document = SyntaxDocument::open(
            syntax_language(params.language),
            DocumentRevision::new(params.revision),
            &params.text,
        )
        .map_err(syntax_error)?;
        result(&project(&params.text, document.snapshot()))
    }
}

fn syntax_language(language: SyntaxLanguageDto) -> SyntaxLanguage {
    match language {
        SyntaxLanguageDto::Javascript => SyntaxLanguage::Javascript,
        SyntaxLanguageDto::Javascriptreact => SyntaxLanguage::Javascriptreact,
        SyntaxLanguageDto::Json => SyntaxLanguage::Json,
        SyntaxLanguageDto::Jsonc => SyntaxLanguage::Jsonc,
        SyntaxLanguageDto::Rust => SyntaxLanguage::Rust,
        SyntaxLanguageDto::Shell => SyntaxLanguage::Shell,
        SyntaxLanguageDto::Typescript => SyntaxLanguage::Typescript,
        SyntaxLanguageDto::Typescriptreact => SyntaxLanguage::Typescriptreact,
    }
}

fn project(text: &str, snapshot: SyntaxSnapshot) -> SyntaxAnalyzeResult {
    let positions = Utf16PositionIndex::for_snapshot(text, &snapshot);
    SyntaxAnalyzeResult {
        revision: snapshot.revision().value(),
        has_errors: snapshot.has_errors(),
        tokens: snapshot
            .tokens()
            .iter()
            .map(|token| SyntaxTokenDto {
                range: positions.project_range(&token.range),
                kind: token_kind(token.kind),
            })
            .collect(),
        folding_ranges: snapshot
            .folding_ranges()
            .iter()
            .map(|fold| SyntaxFoldingRangeDto {
                range: positions.project_range(&fold.range),
            })
            .collect(),
        symbols: snapshot
            .symbols()
            .iter()
            .map(|symbol| SyntaxSymbolDto {
                name: symbol.name.clone(),
                kind: symbol_kind(symbol.kind),
                range: positions.project_range(&symbol.range),
                selection_range: positions.project_range(&symbol.selection_range),
            })
            .collect(),
        diagnostics: snapshot
            .diagnostics()
            .iter()
            .map(|diagnostic| SyntaxDiagnosticDto {
                range: positions.project_range(&diagnostic.range),
                kind: diagnostic_kind(diagnostic.kind),
            })
            .collect(),
    }
}

fn token_kind(kind: SyntaxTokenKind) -> SyntaxTokenKindDto {
    match kind {
        SyntaxTokenKind::Attribute => SyntaxTokenKindDto::Attribute,
        SyntaxTokenKind::Comment => SyntaxTokenKindDto::Comment,
        SyntaxTokenKind::Constant => SyntaxTokenKindDto::Constant,
        SyntaxTokenKind::Constructor => SyntaxTokenKindDto::Constructor,
        SyntaxTokenKind::Embedded => SyntaxTokenKindDto::Embedded,
        SyntaxTokenKind::Function => SyntaxTokenKindDto::Function,
        SyntaxTokenKind::Keyword => SyntaxTokenKindDto::Keyword,
        SyntaxTokenKind::Label => SyntaxTokenKindDto::Label,
        SyntaxTokenKind::Module => SyntaxTokenKindDto::Module,
        SyntaxTokenKind::Number => SyntaxTokenKindDto::Number,
        SyntaxTokenKind::Operator => SyntaxTokenKindDto::Operator,
        SyntaxTokenKind::Property => SyntaxTokenKindDto::Property,
        SyntaxTokenKind::Punctuation => SyntaxTokenKindDto::Punctuation,
        SyntaxTokenKind::String => SyntaxTokenKindDto::String,
        SyntaxTokenKind::Type => SyntaxTokenKindDto::Type,
        SyntaxTokenKind::Variable => SyntaxTokenKindDto::Variable,
    }
}

fn symbol_kind(kind: DocumentSymbolKind) -> SyntaxSymbolKindDto {
    match kind {
        DocumentSymbolKind::Constant => SyntaxSymbolKindDto::Constant,
        DocumentSymbolKind::Enum => SyntaxSymbolKindDto::Enum,
        DocumentSymbolKind::Field => SyntaxSymbolKindDto::Field,
        DocumentSymbolKind::Function => SyntaxSymbolKindDto::Function,
        DocumentSymbolKind::Macro => SyntaxSymbolKindDto::Macro,
        DocumentSymbolKind::Method => SyntaxSymbolKindDto::Method,
        DocumentSymbolKind::Module => SyntaxSymbolKindDto::Module,
        DocumentSymbolKind::Static => SyntaxSymbolKindDto::Static,
        DocumentSymbolKind::Struct => SyntaxSymbolKindDto::Struct,
        DocumentSymbolKind::Trait => SyntaxSymbolKindDto::Trait,
        DocumentSymbolKind::Type => SyntaxSymbolKindDto::Type,
        DocumentSymbolKind::Variable => SyntaxSymbolKindDto::Variable,
    }
}

fn diagnostic_kind(kind: SyntaxDiagnosticKind) -> SyntaxDiagnosticKindDto {
    match kind {
        SyntaxDiagnosticKind::Error => SyntaxDiagnosticKindDto::Error,
        SyntaxDiagnosticKind::Missing => SyntaxDiagnosticKindDto::Missing,
    }
}

fn syntax_error(error: SyntaxError) -> RpcError {
    match error {
        SyntaxError::DocumentTooLarge { .. } => {
            RpcError::new(-32602, AppServerErrorName::InvalidParams)
        }
        SyntaxError::Language { .. }
        | SyntaxError::Query { .. }
        | SyntaxError::ParseCancelled
        | SyntaxError::NonIncreasingRevision { .. }
        | SyntaxError::InvalidEditRange { .. }
        | SyntaxError::InvalidEditBoundary { .. }
        | SyntaxError::OverlappingEdits => {
            RpcError::new(-32071, AppServerErrorName::SyntaxAnalysisFailed)
        }
    }
}

struct Utf16PositionIndex {
    byte_offsets: Vec<usize>,
    positions: Vec<SyntaxPositionDto>,
}

impl Utf16PositionIndex {
    fn for_snapshot(text: &str, snapshot: &SyntaxSnapshot) -> Self {
        let mut byte_offsets = Vec::with_capacity(
            snapshot.tokens().len() * 2
                + snapshot.folding_ranges().len() * 2
                + snapshot.symbols().len() * 4
                + snapshot.diagnostics().len() * 2,
        );
        for token in snapshot.tokens() {
            push_range_offsets(&mut byte_offsets, &token.range);
        }
        for folding_range in snapshot.folding_ranges() {
            push_range_offsets(&mut byte_offsets, &folding_range.range);
        }
        for symbol in snapshot.symbols() {
            push_range_offsets(&mut byte_offsets, &symbol.range);
            push_range_offsets(&mut byte_offsets, &symbol.selection_range);
        }
        for diagnostic in snapshot.diagnostics() {
            push_range_offsets(&mut byte_offsets, &diagnostic.range);
        }
        byte_offsets.sort_unstable();
        byte_offsets.dedup();

        let mut positions = Vec::with_capacity(byte_offsets.len());
        let mut next_offset = 0;
        let mut line_index = 0;
        let mut column_index = 0;
        for (byte_offset, character) in text.char_indices() {
            while byte_offsets.get(next_offset) == Some(&byte_offset) {
                positions.push(SyntaxPositionDto {
                    line_index,
                    column_index,
                });
                next_offset += 1;
            }
            if character == '\n' {
                line_index += 1;
                column_index = 0;
            } else {
                column_index += character.len_utf16();
            }
        }
        while byte_offsets.get(next_offset) == Some(&text.len()) {
            positions.push(SyntaxPositionDto {
                line_index,
                column_index,
            });
            next_offset += 1;
        }
        assert_eq!(
            next_offset,
            byte_offsets.len(),
            "syntax snapshots must contain only UTF-8 boundaries"
        );
        Self {
            byte_offsets,
            positions,
        }
    }

    fn project_range(&self, range: &SyntaxRange) -> SyntaxRangeDto {
        SyntaxRangeDto {
            start: self.position(range.bytes.start),
            end: self.position(range.bytes.end),
        }
    }

    fn position(&self, byte_offset: usize) -> SyntaxPositionDto {
        let index = self
            .byte_offsets
            .binary_search(&byte_offset)
            .expect("syntax range boundary must be indexed");
        self.positions[index]
    }
}

fn push_range_offsets(offsets: &mut Vec<usize>, range: &SyntaxRange) {
    offsets.push(range.bytes.start);
    offsets.push(range.bytes.end);
}

#[cfg(test)]
#[path = "syntax_operations_tests.rs"]
mod tests;
