use std::str::FromStr;

use zeta_lsp::lsp_types::{
    CompletionItem, CompletionResponse, GotoDefinitionResponse, Hover, HoverContents, Location,
    MarkedString, PositionEncodingKind, Range, TextEdit, Uri,
};

use super::*;

#[test]
fn request_position_uses_the_negotiated_encoding_without_splitting_unicode() {
    let text = "zero\n🦀rust";
    let position = LanguageDocumentPosition::new(1, "🦀".len() as u32);

    assert_eq!(
        protocol_position(text, position, &PositionEncodingKind::UTF8),
        Some(Position::new(1, 4))
    );
    assert_eq!(
        protocol_position(text, position, &PositionEncodingKind::UTF16),
        Some(Position::new(1, 2))
    );
    assert_eq!(
        protocol_position(
            text,
            LanguageDocumentPosition::new(1, 1),
            &PositionEncodingKind::UTF8
        ),
        None
    );
}

#[test]
fn hover_completion_and_definition_projection_remove_protocol_types() {
    let request_id = LanguageRequestId::new(7);
    let revision = LanguageDocumentRevision::new(4);
    let path = PathBuf::from("/workspace/src/main.rs");
    let hover = project_hover(
        request_id,
        path.clone(),
        revision,
        "fn main() {}",
        &PositionEncodingKind::UTF8,
        Some(Hover {
            contents: HoverContents::Scalar(MarkedString::String("main docs".into())),
            range: Some(Range::new(Position::new(0, 3), Position::new(0, 7))),
        }),
    )
    .expect("hover");
    assert_eq!(hover.contents, "main docs");
    assert_eq!(hover.range.expect("range").byte_range(), 3..7);

    let completions = project_completions(
        request_id,
        path.clone(),
        revision,
        LanguageDocumentPosition::new(0, 7),
        "println",
        &PositionEncodingKind::UTF8,
        Some(CompletionResponse::Array(vec![CompletionItem {
            label: "println!".into(),
            detail: Some("macro".into()),
            insert_text: Some("println!($0)".into()),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit::new(
                Range::new(Position::new(0, 0), Position::new(0, 7)),
                "println!()".into(),
            ))),
            ..CompletionItem::default()
        }])),
    );
    assert_eq!(completions.items[0].insert_text, "println!($0)");
    assert_eq!(
        completions.items[0]
            .edit
            .as_ref()
            .expect("safe edit")
            .range
            .byte_range(),
        0..7
    );

    let insertion = project_completions(
        request_id,
        path.clone(),
        revision,
        LanguageDocumentPosition::new(0, "🦀".len() as u32),
        "🦀value",
        &PositionEncodingKind::UTF16,
        Some(CompletionResponse::Array(vec![CompletionItem {
            label: "::new()".into(),
            ..CompletionItem::default()
        }])),
    );
    assert_eq!(
        insertion.items[0]
            .edit
            .as_ref()
            .expect("plain insertion edit")
            .range
            .byte_range(),
        4..4
    );

    let target_uri = Uri::from_str("file:///workspace/src/lib.rs").expect("URI");
    let definitions = project_definitions(
        request_id,
        path,
        revision,
        &PositionEncodingKind::UTF16,
        Some(GotoDefinitionResponse::Scalar(Location::new(
            target_uri,
            Range::new(Position::new(2, 4), Position::new(2, 8)),
        ))),
    );
    assert_eq!(
        definitions.targets[0].path,
        PathBuf::from("/workspace/src/lib.rs")
    );
    assert_eq!(definitions.targets[0].row, 2);
    assert_eq!(definitions.targets[0].character, 4);
    assert_eq!(
        definitions.targets[0].encoding,
        LanguagePositionEncoding::Utf16
    );
}
