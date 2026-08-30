use super::ChatComposer;
use super::ChatComposerOutcome;
use crate::components::chat_input::ChatInput;
use crate::components::chat_input::ChatInputItem;
use crate::components::chat_input::ChatInputMode;
use crate::components::chat_input::TuiSlashCommandAction;
use crate::components::chat_input::built_in_catalog_command;
use crate::components::chat_input::built_in_slash_command_definitions;
use crate::components::chat_input::default_slash_command_catalog;
use crate::components::suggest::SkillSelectorItem;
use crate::components::suggest::SuggestView;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_protocol::ContentDigest;
use zeta_protocol::SkillId;
use zeta_protocol::SkillName;
use zeta_protocol::SkillRef;
use zeta_protocol::SkillSourceId;
use zeta_slash_commands::{
    SlashCommandArgumentMode, SlashCommandCatalog, SlashCommandDefinition, SlashCommandOrigin,
};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

struct ChatComposerHarness {
    composer: ChatComposer,
    input: ChatInput,
}

impl ChatComposerHarness {
    fn new() -> Self {
        Self::with_slash_commands(default_slash_command_catalog())
    }

    fn with_slash_commands(catalog: SlashCommandCatalog) -> Self {
        Self {
            composer: ChatComposer::with_slash_commands(catalog),
            input: ChatInput::new(),
        }
    }

    fn insert_text(&mut self, text: &str) {
        self.composer.insert_text(&mut self.input, text);
    }

    fn handle_key(&mut self, key: KeyEvent) -> ChatComposerOutcome {
        self.composer.handle_key(&mut self.input, key)
    }

    fn handle_paste(&mut self, pasted: String) -> Result<(), String> {
        self.composer.handle_paste(&mut self.input, pasted)
    }

    fn attach_image_bytes(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        self.composer.attach_image_bytes(&mut self.input, bytes)
    }

    fn text(&self) -> &str {
        self.input.text()
    }

    fn slash_popup(&self) -> bool {
        matches!(
            self.composer.suggest(&self.input),
            Some(SuggestView::Slash(_))
        )
    }

    fn replace_chat_input_catalog(
        &mut self,
        slash_commands: SlashCommandCatalog,
        skills: Vec<SkillSelectorItem>,
    ) {
        self.composer.replace_chat_input_catalog(
            &mut self.input,
            slash_commands,
            skills,
            Vec::new(),
        );
    }
}

#[test]
fn cursor_aware_completion_preserves_an_existing_argument_tail() {
    let mut chat_input = ChatComposerHarness::new();
    chat_input.insert_text("/mod provider/model");
    chat_input.handle_key(key(KeyCode::Home));

    chat_input.handle_key(key(KeyCode::Tab));

    assert_eq!(chat_input.text(), "/model provider/model");
    let ChatComposerOutcome::Command(invocation) = chat_input.handle_key(key(KeyCode::Enter))
    else {
        panic!("expected inline command invocation");
    };
    assert_eq!(
        invocation.command,
        built_in_catalog_command(TuiSlashCommandAction::Model)
    );
    assert_eq!(invocation.display_arguments, "provider/model");
    assert_eq!(
        invocation.arguments,
        vec![ChatInputItem::Text("provider/model".into())]
    );
}

#[test]
fn completion_escape_precedes_vim_and_normal_mode_does_not_submit() {
    let mut chat_input = ChatComposerHarness::new();
    chat_input.input.set_input_mode(ChatInputMode::Vim);
    chat_input.insert_text("/mo");
    assert!(chat_input.slash_popup());

    assert_eq!(
        chat_input.handle_key(key(KeyCode::Esc)),
        ChatComposerOutcome::Consumed
    );
    assert_eq!(chat_input.input.prompt(), "❯ ");
    assert_eq!(
        chat_input.handle_key(key(KeyCode::Esc)),
        ChatComposerOutcome::Consumed
    );
    assert_eq!(chat_input.input.prompt(), "N ");
    assert_eq!(
        chat_input.handle_key(key(KeyCode::Enter)),
        ChatComposerOutcome::Consumed
    );
    chat_input.handle_key(key(KeyCode::Char('i')));
    assert!(matches!(
        chat_input.handle_key(key(KeyCode::Enter)),
        ChatComposerOutcome::Submit(_)
    ));
}

#[test]
fn inline_command_arguments_preserve_images_and_following_text() {
    let mut chat_input = ChatComposerHarness::new();
    chat_input.insert_text("/model ");
    chat_input
        .attach_image_bytes(b"\x89PNG\r\n\x1a\npayload".to_vec())
        .unwrap();
    chat_input.insert_text("inspect this");

    let ChatComposerOutcome::Command(invocation) = chat_input.handle_key(key(KeyCode::Enter))
    else {
        panic!("expected structured inline command invocation");
    };

    assert_eq!(invocation.display_arguments, "[Image #1] inspect this");
    assert_eq!(invocation.arguments.len(), 2);
    assert!(matches!(
        &invocation.arguments[0],
        ChatInputItem::Image { url } if url.starts_with("data:image/png;base64,")
    ));
    assert_eq!(
        invocation.arguments[1],
        ChatInputItem::Text("inspect this".into())
    );
}

#[test]
fn inline_command_arguments_expand_large_pastes_without_losing_binding() {
    let mut chat_input = ChatComposerHarness::new();
    let pasted = "p".repeat(1001);
    chat_input.insert_text("/model ");
    chat_input.handle_paste(pasted.clone()).unwrap();

    let ChatComposerOutcome::Command(invocation) = chat_input.handle_key(key(KeyCode::Enter))
    else {
        panic!("expected pasted inline command invocation");
    };

    assert_eq!(invocation.display_arguments, pasted);
    assert_eq!(
        invocation.arguments,
        vec![ChatInputItem::Text("p".repeat(1001))]
    );
}

#[test]
fn arguments_on_an_argument_free_command_remain_a_normal_prompt() {
    let mut chat_input = ChatComposerHarness::new();
    chat_input.insert_text("/quit now");

    let ChatComposerOutcome::Submit(submission) = chat_input.handle_key(key(KeyCode::Enter)) else {
        panic!("expected normal prompt submission");
    };

    assert_eq!(submission.display_text, "/quit now");
}

#[test]
fn deleting_an_atomic_command_clears_its_binding_and_allows_new_discovery() {
    let mut chat_input = ChatComposerHarness::new();
    chat_input.insert_text("/model ");
    chat_input.handle_key(key(KeyCode::Home));

    chat_input.handle_key(key(KeyCode::Delete));
    chat_input.insert_text("/");

    assert_eq!(chat_input.text(), "/ ");
    assert!(chat_input.slash_popup());
}

#[test]
fn dynamic_commands_share_popup_completion_and_submission() {
    let dynamic = SlashCommandDefinition {
        name: "diagnose".into(),
        description: "inspect the current dir".into(),
        argument_mode: SlashCommandArgumentMode::Optional,
    };
    let registry = SlashCommandCatalog::with_local_and_server(
        built_in_slash_command_definitions(),
        [dynamic.clone()],
    )
    .unwrap();
    let mut chat_input = ChatComposerHarness::with_slash_commands(registry);
    chat_input.insert_text("/diag logs");
    chat_input.handle_key(key(KeyCode::Home));

    chat_input.handle_key(key(KeyCode::Tab));
    assert_eq!(chat_input.text(), "/diagnose logs");

    let ChatComposerOutcome::Command(invocation) = chat_input.handle_key(key(KeyCode::Enter))
    else {
        panic!("expected dynamic command invocation");
    };
    assert_eq!(invocation.command, dynamic);
    assert_eq!(invocation.origin, SlashCommandOrigin::Server);
    assert_eq!(invocation.display_arguments, "logs");
    assert_eq!(
        invocation.arguments,
        vec![ChatInputItem::Text("logs".into())]
    );
}

#[test]
fn forwarded_dynamic_command_restores_command_text_before_structured_arguments() {
    let dynamic = SlashCommandDefinition {
        name: "diagnose".into(),
        description: "inspect the current dir".into(),
        argument_mode: SlashCommandArgumentMode::Optional,
    };
    let registry =
        SlashCommandCatalog::with_local_and_server(built_in_slash_command_definitions(), [dynamic])
            .unwrap();
    let mut chat_input = ChatComposerHarness::with_slash_commands(registry);
    chat_input.insert_text("/diagnose ");
    chat_input
        .attach_image_bytes(b"\x89PNG\r\n\x1a\npayload".to_vec())
        .unwrap();
    chat_input.insert_text("logs");

    let ChatComposerOutcome::Command(invocation) = chat_input.handle_key(key(KeyCode::Enter))
    else {
        panic!("expected dynamic command invocation");
    };
    let submission = invocation.into_forwarded_submission();

    assert_eq!(submission.display_text, "/diagnose [Image #1] logs");
    assert_eq!(
        submission.input.first(),
        Some(&ChatInputItem::Text("/diagnose".into()))
    );
    assert!(matches!(
        submission.input.get(1),
        Some(ChatInputItem::Image { url }) if url.starts_with("data:image/png;base64,")
    ));
    assert_eq!(
        submission.input.get(2),
        Some(&ChatInputItem::Text("logs".into()))
    );
}

#[test]
fn up_and_down_recall_plain_submissions_and_restore_the_current_draft() {
    let mut chat_input = ChatComposerHarness::new();
    for prompt in ["first", "second"] {
        chat_input.insert_text(prompt);
        assert!(matches!(
            chat_input.handle_key(key(KeyCode::Enter)),
            ChatComposerOutcome::Submit(_)
        ));
    }
    chat_input.insert_text("draft");

    chat_input.handle_key(key(KeyCode::Up));
    assert_eq!(chat_input.text(), "second");
    chat_input.handle_key(key(KeyCode::Up));
    assert_eq!(chat_input.text(), "first");
    chat_input.handle_key(key(KeyCode::Down));
    assert_eq!(chat_input.text(), "second");
    chat_input.handle_key(key(KeyCode::Down));
    assert_eq!(chat_input.text(), "draft");
}

#[test]
fn shift_enter_inserts_a_newline_and_enter_submits_the_multiline_prompt() {
    let mut chat_input = ChatComposerHarness::new();
    chat_input.insert_text("first line");

    assert_eq!(
        chat_input.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::SHIFT)),
        ChatComposerOutcome::Consumed
    );
    chat_input.insert_text("second line");

    let ChatComposerOutcome::Submit(submission) = chat_input.handle_key(key(KeyCode::Enter)) else {
        panic!("multiline prompt should submit");
    };
    assert_eq!(submission.display_text, "first line\nsecond line");
    assert_eq!(
        submission.input,
        vec![ChatInputItem::Text("first line\nsecond line".into())]
    );
}

#[test]
fn queued_input_preserves_exact_text_image_paste_and_skill_bindings() {
    let skill = SkillRef::pinned(
        SkillId::new(
            SkillSourceId::new("user:skill-source:test").unwrap(),
            SkillName::new("commit").unwrap(),
        ),
        ContentDigest::sha256(b"commit skill"),
    );
    let mut chat_input = ChatComposerHarness::new();
    chat_input.replace_chat_input_catalog(
        default_slash_command_catalog(),
        vec![SkillSelectorItem::new(
            "commit".into(),
            "draft a commit message".into(),
            skill,
        )],
    );
    chat_input.insert_text("$com");
    chat_input.handle_key(key(KeyCode::Tab));
    chat_input.insert_text("inspect ");
    chat_input
        .attach_image_bytes(b"\x89PNG\r\n\x1a\npayload".to_vec())
        .unwrap();
    chat_input.handle_paste("p".repeat(1001)).unwrap();

    let ChatComposerOutcome::Queued(queued) = chat_input
        .composer
        .handle_queued_turn_key(&mut chat_input.input, key(KeyCode::Enter))
    else {
        panic!("expected Queue content");
    };
    assert!(chat_input.text().is_empty());

    let actual = queued.submission();
    assert_eq!(
        actual.display_text,
        format!("$commit inspect [Image #1] {}", "p".repeat(1001))
    );
    assert_eq!(actual.input.len(), 4);
    assert_eq!(
        actual
            .input
            .iter()
            .filter(|item| matches!(item, ChatInputItem::Skill { .. }))
            .count(),
        1
    );
}
