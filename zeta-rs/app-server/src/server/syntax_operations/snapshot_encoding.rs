use std::cmp::Reverse;
use std::collections::BTreeMap;
use std::ops::Range;

use zeta_app_server_protocol::protocol::syntax::{
    SyntaxAnalysisSnapshotDto, SyntaxDiagnosticDto, SyntaxDiagnosticSeverityDto,
    SyntaxDocumentSymbolDto, SyntaxDocumentSymbolKindDto, SyntaxPositionDto, SyntaxRangeDto,
    SyntaxTokenDataDto, SyntaxTokenTypeDto,
};
use zeta_syntax::{
    DocumentSymbolKind, SyntaxDiagnosticKind, SyntaxRange, SyntaxSnapshot, SyntaxTokenKind,
};

use super::SyntaxAnalysisError;

#[derive(Clone, Debug, Eq, PartialEq)]
struct TokenSegment {
    bytes: Range<usize>,
    kind: SyntaxTokenKind,
}

pub(super) fn syntax_analysis_snapshot(
    text: &str,
    snapshot: &SyntaxSnapshot,
) -> Result<SyntaxAnalysisSnapshotDto, SyntaxAnalysisError> {
    let line_starts = line_starts(text);
    Ok(SyntaxAnalysisSnapshotDto {
        revision: snapshot
            .revision()
            .value()
            .try_into()
            .map_err(|_| SyntaxAnalysisError::Failed)?,
        result_id: snapshot.revision().value().to_string(),
        has_errors: snapshot.has_errors(),
        tokens: SyntaxTokenDataDto {
            legend: token_legend(),
            data: encode_semantic_tokens(text, snapshot)?,
        },
        folding_ranges: snapshot
            .folding_ranges()
            .iter()
            .map(|folding| syntax_range_dto(text, &line_starts, &folding.range))
            .collect::<Result<_, _>>()?,
        symbols: snapshot
            .symbols()
            .iter()
            .map(|symbol| {
                Ok(SyntaxDocumentSymbolDto {
                    name: symbol.name.clone(),
                    kind: symbol_kind_dto(symbol.kind),
                    range: syntax_range_dto(text, &line_starts, &symbol.range)?,
                    selection_range: syntax_range_dto(text, &line_starts, &symbol.selection_range)?,
                })
            })
            .collect::<Result<_, SyntaxAnalysisError>>()?,
        diagnostics: snapshot
            .diagnostics()
            .iter()
            .map(|diagnostic| {
                Ok(SyntaxDiagnosticDto {
                    range: syntax_range_dto(text, &line_starts, &diagnostic.range)?,
                    severity: SyntaxDiagnosticSeverityDto::Error,
                    message: match diagnostic.kind {
                        SyntaxDiagnosticKind::Error => "Invalid syntax",
                        SyntaxDiagnosticKind::Missing => "Missing syntax",
                    }
                    .into(),
                    source: "tree-sitter".into(),
                })
            })
            .collect::<Result<_, SyntaxAnalysisError>>()?,
    })
}

fn syntax_range_dto(
    text: &str,
    line_starts: &[usize],
    range: &SyntaxRange,
) -> Result<SyntaxRangeDto, SyntaxAnalysisError> {
    Ok(SyntaxRangeDto {
        start: syntax_position_dto(text, line_starts, range.start.row, range.bytes.start)?,
        end: syntax_position_dto(text, line_starts, range.end.row, range.bytes.end)?,
    })
}

fn syntax_position_dto(
    text: &str,
    line_starts: &[usize],
    line: usize,
    byte_offset: usize,
) -> Result<SyntaxPositionDto, SyntaxAnalysisError> {
    let line_start = line_starts
        .get(line)
        .copied()
        .ok_or(SyntaxAnalysisError::Failed)?;
    let prefix = text
        .get(line_start..byte_offset)
        .ok_or(SyntaxAnalysisError::Failed)?;
    Ok(SyntaxPositionDto {
        line,
        character: prefix.encode_utf16().count(),
    })
}

fn token_legend() -> Vec<SyntaxTokenTypeDto> {
    vec![
        SyntaxTokenTypeDto::Attribute,
        SyntaxTokenTypeDto::Comment,
        SyntaxTokenTypeDto::Constant,
        SyntaxTokenTypeDto::Constructor,
        SyntaxTokenTypeDto::Embedded,
        SyntaxTokenTypeDto::Function,
        SyntaxTokenTypeDto::Keyword,
        SyntaxTokenTypeDto::Label,
        SyntaxTokenTypeDto::Module,
        SyntaxTokenTypeDto::Number,
        SyntaxTokenTypeDto::Operator,
        SyntaxTokenTypeDto::Property,
        SyntaxTokenTypeDto::String,
        SyntaxTokenTypeDto::Type,
        SyntaxTokenTypeDto::Variable,
    ]
}

fn symbol_kind_dto(kind: DocumentSymbolKind) -> SyntaxDocumentSymbolKindDto {
    match kind {
        DocumentSymbolKind::Constant => SyntaxDocumentSymbolKindDto::Constant,
        DocumentSymbolKind::Enum => SyntaxDocumentSymbolKindDto::Enum,
        DocumentSymbolKind::Field => SyntaxDocumentSymbolKindDto::Field,
        DocumentSymbolKind::Function => SyntaxDocumentSymbolKindDto::Function,
        DocumentSymbolKind::Macro => SyntaxDocumentSymbolKindDto::Macro,
        DocumentSymbolKind::Method => SyntaxDocumentSymbolKindDto::Method,
        DocumentSymbolKind::Module => SyntaxDocumentSymbolKindDto::Module,
        DocumentSymbolKind::Static => SyntaxDocumentSymbolKindDto::Static,
        DocumentSymbolKind::Struct => SyntaxDocumentSymbolKindDto::Struct,
        DocumentSymbolKind::Trait => SyntaxDocumentSymbolKindDto::Trait,
        DocumentSymbolKind::Type => SyntaxDocumentSymbolKindDto::Type,
        DocumentSymbolKind::Variable => SyntaxDocumentSymbolKindDto::Variable,
    }
}

fn encode_semantic_tokens(
    text: &str,
    snapshot: &SyntaxSnapshot,
) -> Result<Vec<u32>, SyntaxAnalysisError> {
    let line_starts = line_starts(text);
    let mut lines = BTreeMap::<usize, Vec<TokenSegment>>::new();
    for token in snapshot.tokens() {
        for line in token.range.start.row..=token.range.end.row {
            let Some(&line_start) = line_starts.get(line) else {
                return Err(SyntaxAnalysisError::Failed);
            };
            let line_end = line_content_end(text, &line_starts, line);
            let start = token.range.bytes.start.max(line_start);
            let end = token.range.bytes.end.min(line_end);
            if start < end {
                overlay_segment(
                    lines.entry(line).or_default(),
                    TokenSegment {
                        bytes: start..end,
                        kind: token.kind,
                    },
                );
            }
        }
    }

    let mut data = Vec::new();
    let mut previous_line = 0usize;
    let mut previous_start = 0usize;
    let mut first = true;
    for (line, mut segments) in lines {
        segments.sort_by_key(|segment| (segment.bytes.start, Reverse(segment.bytes.end)));
        for segment in merge_adjacent_segments(segments) {
            let line_start = line_starts[line];
            let start = text[line_start..segment.bytes.start].encode_utf16().count();
            let length = text[segment.bytes.clone()].encode_utf16().count();
            if length == 0 {
                continue;
            }
            let delta_line = if first { line } else { line - previous_line };
            let delta_start = if !first && delta_line == 0 {
                start - previous_start
            } else {
                start
            };
            data.extend([
                to_u32(delta_line)?,
                to_u32(delta_start)?,
                to_u32(length)?,
                token_type(segment.kind),
                0,
            ]);
            previous_line = line;
            previous_start = start;
            first = false;
        }
    }
    Ok(data)
}

fn line_starts(text: &str) -> Vec<usize> {
    let mut starts = vec![0];
    starts.extend(
        text.bytes()
            .enumerate()
            .filter_map(|(index, byte)| (byte == b'\n').then_some(index + 1)),
    );
    starts
}

fn line_content_end(text: &str, starts: &[usize], line: usize) -> usize {
    let mut end = starts.get(line + 1).copied().unwrap_or(text.len());
    if end > 0 && text.as_bytes()[end - 1] == b'\n' {
        end -= 1;
    }
    if end > 0 && text.as_bytes()[end - 1] == b'\r' {
        end -= 1;
    }
    end
}

fn overlay_segment(segments: &mut Vec<TokenSegment>, overlay: TokenSegment) {
    let mut next = Vec::with_capacity(segments.len() + 1);
    for segment in segments.drain(..) {
        if segment.bytes.end <= overlay.bytes.start || segment.bytes.start >= overlay.bytes.end {
            next.push(segment);
            continue;
        }
        if segment.bytes.start < overlay.bytes.start {
            next.push(TokenSegment {
                bytes: segment.bytes.start..overlay.bytes.start,
                kind: segment.kind,
            });
        }
        if segment.bytes.end > overlay.bytes.end {
            next.push(TokenSegment {
                bytes: overlay.bytes.end..segment.bytes.end,
                kind: segment.kind,
            });
        }
    }
    next.push(overlay);
    *segments = next;
}

fn merge_adjacent_segments(mut segments: Vec<TokenSegment>) -> Vec<TokenSegment> {
    let mut merged: Vec<TokenSegment> = Vec::with_capacity(segments.len());
    for segment in segments.drain(..) {
        if let Some(previous) = merged.last_mut()
            && previous.kind == segment.kind
            && previous.bytes.end == segment.bytes.start
        {
            previous.bytes.end = segment.bytes.end;
        } else {
            merged.push(segment);
        }
    }
    merged
}

fn token_type(kind: SyntaxTokenKind) -> u32 {
    match kind {
        SyntaxTokenKind::Attribute => 0,
        SyntaxTokenKind::Comment => 1,
        SyntaxTokenKind::Constant => 2,
        SyntaxTokenKind::Constructor => 3,
        SyntaxTokenKind::Embedded => 4,
        SyntaxTokenKind::Function => 5,
        SyntaxTokenKind::Keyword => 6,
        SyntaxTokenKind::Label => 7,
        SyntaxTokenKind::Module => 8,
        SyntaxTokenKind::Number => 9,
        SyntaxTokenKind::Operator => 10,
        SyntaxTokenKind::Property => 11,
        SyntaxTokenKind::Punctuation => 10,
        SyntaxTokenKind::String => 12,
        SyntaxTokenKind::Type => 13,
        SyntaxTokenKind::Variable => 14,
    }
}

fn to_u32(value: usize) -> Result<u32, SyntaxAnalysisError> {
    value.try_into().map_err(|_| SyntaxAnalysisError::Failed)
}
