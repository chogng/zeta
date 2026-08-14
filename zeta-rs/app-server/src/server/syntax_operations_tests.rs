use super::Utf16PositionIndex;
use super::byte_offset_for_utf16;
use super::byte_range_for_utf16;
use super::project;
use super::syntax_language;
use zeta_app_server_protocol::protocol::syntax::SyntaxLanguageDto;
use zeta_app_server_protocol::protocol::syntax::SyntaxPositionDto;
use zeta_app_server_protocol::protocol::syntax::SyntaxRangeDto;
use zeta_app_server_protocol::protocol::syntax::SyntaxTokenKindDto;
use zeta_syntax::DocumentRevision;
use zeta_syntax::SyntaxDocument;
use zeta_syntax::SyntaxLanguage;

#[test]
fn projects_syntax_ranges_as_utf16_positions() {
    let source = "{\n  \"emoji😀\": [\n  ]\n}\n";
    let document = SyntaxDocument::open(SyntaxLanguage::Json, DocumentRevision::new(12), source)
        .expect("JSON grammar should load");

    let result = project(source, document.snapshot());
    let string = result
        .tokens
        .iter()
        .find(|token| token.kind == SyntaxTokenKindDto::String)
        .expect("string token should be projected");

    assert_eq!(result.revision, 12);
    assert_eq!(string.range.start.line_index, 1);
    assert_eq!(string.range.start.column_index, 2);
    assert_eq!(string.range.end.line_index, 1);
    assert_eq!(string.range.end.column_index, 11);
    assert!(
        result
            .folding_ranges
            .iter()
            .any(|range| { range.range.start.line_index == 1 && range.range.end.line_index == 2 })
    );
}

#[test]
fn maps_ecmascript_protocol_languages_to_the_authoritative_grammars() {
    assert_eq!(
        syntax_language(SyntaxLanguageDto::Javascript),
        SyntaxLanguage::Javascript
    );
    assert_eq!(
        syntax_language(SyntaxLanguageDto::Javascriptreact),
        SyntaxLanguage::Javascriptreact
    );
    assert_eq!(
        syntax_language(SyntaxLanguageDto::Typescript),
        SyntaxLanguage::Typescript
    );
    assert_eq!(
        syntax_language(SyntaxLanguageDto::Typescriptreact),
        SyntaxLanguage::Typescriptreact
    );
}

#[test]
fn projects_only_requested_structural_selection_scopes() {
    let source = "fn café() {}\n";
    let document = SyntaxDocument::open(SyntaxLanguage::Rust, DocumentRevision::new(4), source)
        .expect("Rust grammar should load");
    let requested = byte_range_for_utf16(
        source,
        SyntaxRangeDto {
            start: SyntaxPositionDto {
                line_index: 0,
                column_index: 3,
            },
            end: SyntaxPositionDto {
                line_index: 0,
                column_index: 7,
            },
        },
    )
    .expect("UTF-16 range should project to byte boundaries");
    let ranges = document
        .selection_ranges(requested)
        .expect("selection query should succeed");
    let positions = Utf16PositionIndex::for_selection_ranges(source, &ranges);

    assert!(
        ranges
            .iter()
            .any(|selection| &source[selection.range.bytes.clone()] == "café")
    );
    assert!(
        ranges
            .iter()
            .any(|selection| &source[selection.range.bytes.clone()] == "fn café() {}")
    );
    assert!(
        ranges
            .iter()
            .all(|selection| selection.range.bytes != (0..source.len()))
    );
    let identifier = ranges
        .iter()
        .find(|selection| &source[selection.range.bytes.clone()] == "café")
        .expect("identifier scope");
    assert_eq!(
        positions.project_range(&identifier.range).end.column_index,
        7
    );
}

#[test]
fn rejects_utf16_positions_inside_surrogate_pairs() {
    assert_eq!(
        byte_offset_for_utf16(
            "😀",
            SyntaxPositionDto {
                line_index: 0,
                column_index: 1,
            },
        ),
        None
    );
}
