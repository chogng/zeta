use std::str::FromStr;

use zeta_lsp::lsp_types::{
    CallHierarchyIncomingCall, CallHierarchyItem, CallHierarchyOutgoingCall, CodeAction,
    CodeActionKind, CodeActionOrCommand, Command, CompletionItem, CompletionResponse, CreateFile,
    DocumentChangeOperation, DocumentChanges, GotoDefinitionResponse, Hover, HoverContents,
    Location, MarkedString, OneOf, OptionalVersionedTextDocumentIdentifier, PositionEncodingKind,
    Range, ResourceOp, SymbolInformation, SymbolKind, TextDocumentEdit, TextEdit, Uri,
    WorkspaceEdit, WorkspaceSymbolResponse,
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
fn workspace_edit_projection_preserves_multiple_documents_versions_and_unicode_ranges() {
    let first = absolute_test_path("first.ts");
    let second = absolute_test_path("second.ts");
    let first_uri =
        Uri::from_str(url::Url::from_file_path(&first).expect("file URL").as_str()).expect("URI");
    let second_uri = Uri::from_str(
        url::Url::from_file_path(&second)
            .expect("file URL")
            .as_str(),
    )
    .expect("URI");
    let edit = WorkspaceEdit {
        document_changes: Some(DocumentChanges::Edits(vec![
            TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri: first_uri,
                    version: Some(3),
                },
                edits: vec![OneOf::Left(TextEdit::new(
                    Range::new(Position::new(0, 2), Position::new(0, 4)),
                    "new".into(),
                ))],
            },
            TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri: second_uri,
                    version: None,
                },
                edits: vec![OneOf::Left(TextEdit::new(
                    Range::new(Position::new(1, 0), Position::new(1, 1)),
                    "x".into(),
                ))],
            },
        ])),
        ..WorkspaceEdit::default()
    };
    let result = project_workspace_edit(
        LanguageRequestId::new(11),
        first.clone(),
        LanguageDocumentRevision::new(8),
        &PositionEncodingKind::UTF16,
        Some(edit),
    )
    .expect("workspace edit");
    assert_eq!(result.edit.entries.len(), 2);
    let LanguageWorkspaceEditEntry::TextDocument(first_edit) = &result.edit.entries[0] else {
        panic!("expected text edit")
    };
    assert_eq!(first_edit.server_version, Some(3));
    assert_eq!(first_edit.edits[0].range.start.character, 2);
    assert_eq!(result.edit.encoding, LanguagePositionEncoding::Utf16);
}

#[test]
fn workspace_edit_projection_preserves_resource_operation_order() {
    let source = absolute_test_path("main.ts");
    let created = Uri::from_str(
        url::Url::from_file_path(absolute_test_path("created.ts"))
            .expect("file URL")
            .as_str(),
    )
    .expect("URI");
    let edit = WorkspaceEdit {
        document_changes: Some(DocumentChanges::Operations(vec![
            DocumentChangeOperation::Op(ResourceOp::Create(CreateFile {
                uri: created,
                options: None,
                annotation_id: None,
            })),
        ])),
        ..WorkspaceEdit::default()
    };

    let result = project_workspace_edit(
        LanguageRequestId::new(12),
        source,
        LanguageDocumentRevision::new(1),
        &PositionEncodingKind::UTF16,
        Some(edit),
    )
    .expect("workspace edit");
    assert!(
        matches!(&result.edit.entries[0], LanguageWorkspaceEditEntry::Create { path, existing: LanguageExistingTargetBehavior::Error } if path == &absolute_test_path("created.ts"))
    );
}

#[test]
fn code_action_projection_disables_actions_that_cannot_be_applied() {
    let source = absolute_test_path("main.ts");
    let unresolved = CodeAction {
        title: "Extract function".into(),
        kind: Some(CodeActionKind::REFACTOR_EXTRACT),
        data: Some(serde_json::json!({ "id": 3 })),
        ..CodeAction::default()
    };
    let command = CodeAction {
        title: "Run server command".into(),
        command: Some(Command::new("Run".into(), "server.run".into(), None)),
        ..CodeAction::default()
    };

    let actions = project_code_actions(
        LanguageRequestId::new(13),
        source,
        LanguageDocumentRevision::new(2),
        &PositionEncodingKind::UTF16,
        false,
        Some(vec![
            CodeActionOrCommand::CodeAction(unresolved),
            CodeActionOrCommand::CodeAction(command),
        ]),
    );

    assert_eq!(actions.actions.len(), 2);
    assert_eq!(
        actions.actions[0].disabled_reason.as_deref(),
        Some("This action requires unsupported language-server resolution")
    );
    assert_eq!(actions.actions[0].provider_data["data"]["id"], 3);
    assert_eq!(
        actions.actions[1].disabled_reason.as_deref(),
        Some("This action requires an unsupported language-server command")
    );
}

#[test]
#[allow(deprecated)]
fn workspace_symbol_projection_keeps_cross_file_location() {
    let path = absolute_test_path("symbols.ts");
    let uri =
        Uri::from_str(url::Url::from_file_path(&path).expect("file URL").as_str()).expect("URI");
    let response = WorkspaceSymbolResponse::Flat(vec![SymbolInformation {
        name: "Widget".into(),
        kind: SymbolKind::CLASS,
        tags: None,
        deprecated: None,
        location: Location::new(uri, Range::new(Position::new(4, 2), Position::new(7, 1))),
        container_name: Some("ui".into()),
    }]);

    let result = project_workspace_symbols(
        LanguageRequestId::new(14),
        "Wid".into(),
        &PositionEncodingKind::UTF16,
        Some(response),
    );
    assert_eq!(result.symbols.len(), 1);
    assert_eq!(result.symbols[0].path, path);
    assert_eq!(result.symbols[0].container_name.as_deref(), Some("ui"));
    assert_eq!(result.symbols[0].range.start.row, 4);
}

#[test]
fn hierarchy_projection_preserves_follow_up_data_and_call_site_ownership() {
    let request_id = LanguageRequestId::new(9);
    let revision = LanguageDocumentRevision::new(5);
    let source = absolute_test_path("main.ts");
    let caller = call_item(
        "caller",
        absolute_test_path("caller.ts"),
        serde_json::json!({ "id": 7 }),
    );
    let callee = call_item(
        "callee",
        absolute_test_path("callee.ts"),
        serde_json::json!(["opaque"]),
    );
    let incoming = project_incoming_calls(
        request_id,
        source.clone(),
        revision,
        &PositionEncodingKind::UTF16,
        vec![CallHierarchyIncomingCall {
            from: caller.clone(),
            from_ranges: vec![Range::new(Position::new(3, 4), Position::new(3, 10))],
        }],
    );
    assert_eq!(incoming.kind, LanguageHierarchyKind::IncomingCalls);
    assert_eq!(
        incoming.entries[0].from_path,
        Some(absolute_test_path("caller.ts"))
    );
    assert_eq!(
        incoming.entries[0].item.data,
        Some(serde_json::json!({ "id": 7 }))
    );

    let outgoing = project_outgoing_calls(
        request_id,
        source.clone(),
        revision,
        &PositionEncodingKind::UTF8,
        absolute_test_path("caller.ts"),
        vec![CallHierarchyOutgoingCall {
            to: callee,
            from_ranges: vec![Range::new(Position::new(1, 2), Position::new(1, 8))],
        }],
    );
    assert_eq!(
        outgoing.entries[0].from_path,
        Some(absolute_test_path("caller.ts"))
    );
    assert_eq!(
        outgoing.entries[0].item.encoding,
        LanguagePositionEncoding::Utf8
    );
    assert!(protocol_call_hierarchy_item(outgoing.entries[0].item.clone()).is_some());
}

fn call_item(name: &str, path: PathBuf, data: serde_json::Value) -> CallHierarchyItem {
    let uri =
        Uri::from_str(url::Url::from_file_path(path).expect("file URL").as_str()).expect("URI");
    CallHierarchyItem {
        name: name.into(),
        kind: SymbolKind::FUNCTION,
        tags: None,
        detail: Some(format!("{name}()")),
        uri,
        range: Range::new(Position::new(1, 0), Position::new(4, 1)),
        selection_range: Range::new(Position::new(1, 3), Position::new(1, 3 + name.len() as u32)),
        data: Some(data),
    }
}

fn absolute_test_path(file: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!(r"C:\workspace\src\{file}"))
    } else {
        PathBuf::from(format!("/workspace/src/{file}"))
    }
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

    let target_path = if cfg!(windows) {
        PathBuf::from(r"C:\workspace\src\lib.rs")
    } else {
        PathBuf::from("/workspace/src/lib.rs")
    };
    let target_url = url::Url::from_file_path(&target_path).expect("file URL");
    let target_uri = Uri::from_str(target_url.as_str()).expect("URI");
    let definitions = project_locations(
        request_id,
        LanguageLocationKind::Definition,
        path,
        revision,
        &PositionEncodingKind::UTF16,
        Some(GotoDefinitionResponse::Scalar(Location::new(
            target_uri,
            Range::new(Position::new(2, 4), Position::new(2, 8)),
        ))),
    );
    assert_eq!(definitions.targets[0].path, target_path);
    assert_eq!(definitions.targets[0].range.start.row, 2);
    assert_eq!(definitions.targets[0].range.start.character, 4);
    assert_eq!(definitions.targets[0].range.end.character, 8);
    assert_eq!(
        definitions.targets[0].selection_range,
        definitions.targets[0].range
    );
    assert_eq!(
        definitions.targets[0].encoding,
        LanguagePositionEncoding::Utf16
    );
}
