use super::ChatComposer;
use super::ComposerInput;
use super::ComposerOutcome;
use crate::components::composer::TuiSlashCommandAction;
use crate::components::composer::built_in_catalog_command;
use crate::components::composer::built_in_slash_command_definitions;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_slash_commands::{
    SlashCommandArgumentMode, SlashCommandCatalog, SlashCommandDefinition, SlashCommandOrigin,
};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn cursor_aware_completion_preserves_an_existing_argument_tail() {
    let mut composer = ChatComposer::new();
    composer.insert_text("/mod provider/model");
    composer.handle_key(key(KeyCode::Home));

    composer.handle_key(key(KeyCode::Tab));

    assert_eq!(composer.text(), "/model provider/model");
    let ComposerOutcome::Command(invocation) = composer.handle_key(key(KeyCode::Enter)) else {
        panic!("expected inline command invocation");
    };
    assert_eq!(
        invocation.command,
        built_in_catalog_command(TuiSlashCommandAction::Model)
    );
    assert_eq!(invocation.display_arguments, "provider/model");
    assert_eq!(
        invocation.arguments,
        vec![ComposerInput::Text("provider/model".into())]
    );
}

#[test]
fn inline_command_arguments_preserve_images_and_following_text() {
    let mut composer = ChatComposer::new();
    composer.insert_text("/model ");
    composer
        .attach_image_bytes(b"\x89PNG\r\n\x1a\npayload".to_vec())
        .unwrap();
    composer.insert_text("inspect this");

    let ComposerOutcome::Command(invocation) = composer.handle_key(key(KeyCode::Enter)) else {
        panic!("expected structured inline command invocation");
    };

    assert_eq!(invocation.display_arguments, "[Image #1] inspect this");
    assert_eq!(invocation.arguments.len(), 2);
    assert!(matches!(
        &invocation.arguments[0],
        ComposerInput::Image { url } if url.starts_with("data:image/png;base64,")
    ));
    assert_eq!(
        invocation.arguments[1],
        ComposerInput::Text("inspect this".into())
    );
}

#[test]
fn inline_command_arguments_expand_large_pastes_without_losing_binding() {
    let mut composer = ChatComposer::new();
    let pasted = "p".repeat(1001);
    composer.insert_text("/model ");
    composer.handle_paste(pasted.clone()).unwrap();

    let ComposerOutcome::Command(invocation) = composer.handle_key(key(KeyCode::Enter)) else {
        panic!("expected pasted inline command invocation");
    };

    assert_eq!(invocation.display_arguments, pasted);
    assert_eq!(
        invocation.arguments,
        vec![ComposerInput::Text("p".repeat(1001))]
    );
}

#[test]
fn arguments_on_an_argument_free_command_remain_a_normal_prompt() {
    let mut composer = ChatComposer::new();
    composer.insert_text("/quit now");

    let ComposerOutcome::Submit(submission) = composer.handle_key(key(KeyCode::Enter)) else {
        panic!("expected normal prompt submission");
    };

    assert_eq!(submission.display_text, "/quit now");
}

#[test]
fn deleting_an_atomic_command_clears_its_binding_and_allows_new_discovery() {
    let mut composer = ChatComposer::new();
    composer.insert_text("/model ");
    composer.handle_key(key(KeyCode::Home));

    composer.handle_key(key(KeyCode::Delete));
    composer.insert_text("/");

    assert_eq!(composer.text(), "/ ");
    assert!(composer.slash_popup().is_some());
}

#[test]
fn dynamic_commands_share_popup_completion_and_submission() {
    let dynamic = SlashCommandDefinition {
        name: "diagnose".into(),
        description: "inspect the current workspace".into(),
        argument_mode: SlashCommandArgumentMode::Optional,
    };
    let registry = SlashCommandCatalog::with_local_and_server(
        built_in_slash_command_definitions(),
        [dynamic.clone()],
    )
    .unwrap();
    let mut composer = ChatComposer::with_slash_commands(registry);
    composer.insert_text("/diag logs");
    composer.handle_key(key(KeyCode::Home));

    composer.handle_key(key(KeyCode::Tab));
    assert_eq!(composer.text(), "/diagnose logs");

    let ComposerOutcome::Command(invocation) = composer.handle_key(key(KeyCode::Enter)) else {
        panic!("expected dynamic command invocation");
    };
    assert_eq!(invocation.command, dynamic);
    assert_eq!(invocation.origin, SlashCommandOrigin::Server);
    assert_eq!(invocation.display_arguments, "logs");
    assert_eq!(
        invocation.arguments,
        vec![ComposerInput::Text("logs".into())]
    );
}

#[test]
fn forwarded_dynamic_command_restores_command_text_before_structured_arguments() {
    let dynamic = SlashCommandDefinition {
        name: "diagnose".into(),
        description: "inspect the current workspace".into(),
        argument_mode: SlashCommandArgumentMode::Optional,
    };
    let registry =
        SlashCommandCatalog::with_local_and_server(built_in_slash_command_definitions(), [dynamic])
            .unwrap();
    let mut composer = ChatComposer::with_slash_commands(registry);
    composer.insert_text("/diagnose ");
    composer
        .attach_image_bytes(b"\x89PNG\r\n\x1a\npayload".to_vec())
        .unwrap();
    composer.insert_text("logs");

    let ComposerOutcome::Command(invocation) = composer.handle_key(key(KeyCode::Enter)) else {
        panic!("expected dynamic command invocation");
    };
    let submission = invocation.into_forwarded_submission();

    assert_eq!(submission.display_text, "/diagnose [Image #1] logs");
    assert_eq!(
        submission.input.first(),
        Some(&ComposerInput::Text("/diagnose".into()))
    );
    assert!(matches!(
        submission.input.get(1),
        Some(ComposerInput::Image { url }) if url.starts_with("data:image/png;base64,")
    ));
    assert_eq!(
        submission.input.get(2),
        Some(&ComposerInput::Text("logs".into()))
    );
}

#[test]
fn up_and_down_recall_plain_submissions_and_restore_the_current_draft() {
    let mut composer = ChatComposer::new();
    for prompt in ["first", "second"] {
        composer.insert_text(prompt);
        assert!(matches!(
            composer.handle_key(key(KeyCode::Enter)),
            ComposerOutcome::Submit(_)
        ));
    }
    composer.insert_text("draft");

    composer.handle_key(key(KeyCode::Up));
    assert_eq!(composer.text(), "second");
    composer.handle_key(key(KeyCode::Up));
    assert_eq!(composer.text(), "first");
    composer.handle_key(key(KeyCode::Down));
    assert_eq!(composer.text(), "second");
    composer.handle_key(key(KeyCode::Down));
    assert_eq!(composer.text(), "draft");
}

#[test]
fn shift_enter_inserts_a_newline_and_enter_submits_the_multiline_prompt() {
    let mut composer = ChatComposer::new();
    composer.insert_text("first line");

    assert_eq!(
        composer.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)),
        ComposerOutcome::Consumed
    );
    composer.insert_text("second line");

    let ComposerOutcome::Submit(submission) = composer.handle_key(key(KeyCode::Enter)) else {
        panic!("multiline prompt should submit");
    };
    assert_eq!(submission.display_text, "first line\nsecond line");
    assert_eq!(
        submission.input,
        vec![ComposerInput::Text("first line\nsecond line".into())]
    );
}
