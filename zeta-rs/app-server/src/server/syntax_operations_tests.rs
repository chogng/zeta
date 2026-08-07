use super::project;
use super::syntax_language;
use zeta_app_server_protocol::protocol::syntax::SyntaxLanguageDto;
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
