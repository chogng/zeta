use zeta_app_server_protocol::protocol::language::LanguageCompletionInsertTextFormatDto;
use zeta_app_server_protocol::protocol::language::LanguageCompletionTriggerKindDto;
use zeta_language_service::LanguageCompletionInsertTextFormat;
use zeta_language_service::LanguageCompletionItem;
use zeta_language_service::LanguageCompletionItemKind;
use zeta_language_service::LanguageCompletionTrigger;
use zeta_language_service::LanguageTextEdit;
use zeta_language_service::LanguageTextRange;

use super::completion_item_to_dto;
use super::completion_trigger;

#[test]
fn completion_item_projection_preserves_utf16_and_snippet_semantics() {
    let item = LanguageCompletionItem {
        label: "println!".into(),
        kind: LanguageCompletionItemKind::Snippet,
        detail: Some("macro".into()),
        documentation: Some("Prints one line".into()),
        filter_text: Some("println".into()),
        sort_text: Some("001".into()),
        preselect: Some(true),
        commit_characters: vec!["(".into()],
        insert_text_format: LanguageCompletionInsertTextFormat::Snippet,
        edit: Some(LanguageTextEdit {
            range: LanguageTextRange::new(4..7),
            new_text: "println!($0)".into(),
        }),
    };

    let dto = completion_item_to_dto("🦀pri", item).expect("completion DTO");

    assert_eq!(dto.range.start.column_index, 2);
    assert_eq!(dto.range.end.column_index, 5);
    assert_eq!(
        dto.insert_text_format,
        LanguageCompletionInsertTextFormatDto::Snippet
    );
    assert_eq!(dto.insert_text, "println!($0)");
}

#[test]
fn completion_trigger_rejects_ambiguous_character_shapes() {
    assert!(matches!(
        completion_trigger(LanguageCompletionTriggerKindDto::Invoke, None),
        Ok(LanguageCompletionTrigger::Invoked)
    ));
    assert!(matches!(
        completion_trigger(
            LanguageCompletionTriggerKindDto::TriggerCharacter,
            Some(".")
        ),
        Ok(LanguageCompletionTrigger::TriggerCharacter(character)) if character == "."
    ));
    assert!(completion_trigger(LanguageCompletionTriggerKindDto::Invoke, Some(".")).is_err());
    assert!(
        completion_trigger(
            LanguageCompletionTriggerKindDto::TriggerCharacter,
            Some("ab")
        )
        .is_err()
    );
    assert!(
        completion_trigger(
            LanguageCompletionTriggerKindDto::TriggerCharacter,
            Some("\n")
        )
        .is_err()
    );
}
