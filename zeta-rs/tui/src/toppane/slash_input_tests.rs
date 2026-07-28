use super::SlashInput;
use crate::toppane::SlashCommand;
use crate::toppane::slash_commands::DynamicSlashCommand;
use crate::toppane::slash_commands::SlashCommandArgumentMode;
use crate::toppane::slash_commands::SlashCommandItem;
use crate::toppane::slash_commands::SlashCommandRegistry;

fn builtins() -> SlashCommandRegistry {
    SlashCommandRegistry::default()
}

#[test]
fn popup_query_tracks_the_command_fragment_before_the_cursor() {
    let registry = builtins();

    assert_eq!(
        SlashInput::at_cursor("/", 1, &registry)
            .popup_query()
            .map(|query| query.text),
        Some("")
    );
    assert_eq!(
        SlashInput::at_cursor("/review details", 4, &registry)
            .popup_query()
            .map(|query| query.text),
        Some("rev")
    );
    assert_eq!(
        SlashInput::at_cursor("/review details", "/review".len(), &registry)
            .popup_query()
            .map(|query| query.text),
        Some("review")
    );
    assert_eq!(
        SlashInput::at_cursor("/review details", "/review ".len(), &registry).popup_query(),
        None
    );
    assert_eq!(
        SlashInput::at_cursor("review", 3, &registry).popup_query(),
        None
    );
}

#[test]
fn matching_commands_uses_the_registered_presentation_order() {
    let registry = builtins();
    assert_eq!(
        SlashInput::at_cursor("/m", 2, &registry).matching_commands(),
        Some(vec![
            SlashCommandItem::Builtin(SlashCommand::Models),
            SlashCommandItem::Builtin(SlashCommand::Mcp),
            SlashCommandItem::Builtin(SlashCommand::Model),
        ])
    );
    assert_eq!(
        SlashInput::at_cursor("/unknown", 8, &registry).matching_commands(),
        Some(Vec::new())
    );
}

#[test]
fn completion_replaces_the_whole_name_and_preserves_the_argument_tail() {
    let registry = builtins();
    let command = SlashCommandItem::Builtin(SlashCommand::Review);
    let completion = SlashInput::at_cursor("/review details", 4, &registry)
        .completion(&command)
        .unwrap();

    assert_eq!(completion.range, 0..8);
    assert_eq!(completion.replacement, "/review ");

    let mut text = "/review details".to_owned();
    text.replace_range(completion.range, &completion.replacement);
    assert_eq!(text, "/review details");
}

#[test]
fn completion_adds_a_separator_before_a_newline() {
    let registry = builtins();
    let command = SlashCommandItem::Builtin(SlashCommand::Review);
    let completion = SlashInput::at_cursor("/rev\nmore", 4, &registry)
        .completion(&command)
        .unwrap();

    assert_eq!(completion.range, 0..4);
    assert_eq!(completion.replacement, "/review ");
}

#[test]
fn submission_recognizes_bare_and_supported_inline_commands() {
    let registry = builtins();
    let bare = SlashInput::for_submission("/quit", &registry)
        .submission_command()
        .unwrap();
    assert_eq!(bare.command, SlashCommandItem::Builtin(SlashCommand::Quit));
    assert!(bare.arguments_range.is_empty());

    let inline = SlashInput::for_submission("/review   src/lib.rs  ", &registry)
        .submission_command()
        .unwrap();
    assert_eq!(
        inline.command,
        SlashCommandItem::Builtin(SlashCommand::Review)
    );
    assert_eq!(
        &"/review   src/lib.rs  "[inline.arguments_range],
        "src/lib.rs"
    );

    assert_eq!(
        SlashInput::for_submission("/quit now", &registry).submission_command(),
        None
    );
    assert_eq!(
        SlashInput::for_submission("/unknown", &registry).submission_command(),
        None
    );
}

#[test]
fn dynamic_commands_share_discovery_completion_and_submission() {
    let registry = SlashCommandRegistry::with_dynamic_commands([DynamicSlashCommand {
        name: "diagnose".into(),
        description: "inspect the current workspace".into(),
        argument_mode: SlashCommandArgumentMode::Optional,
    }])
    .unwrap();
    let command = SlashCommandItem::Dynamic(DynamicSlashCommand {
        name: "diagnose".into(),
        description: "inspect the current workspace".into(),
        argument_mode: SlashCommandArgumentMode::Optional,
    });

    assert_eq!(
        SlashInput::at_cursor("/diag", 5, &registry).matching_commands(),
        Some(vec![command.clone()])
    );
    assert_eq!(
        SlashInput::at_cursor("/diag", 5, &registry)
            .completion(&command)
            .unwrap()
            .replacement,
        "/diagnose "
    );
    let parsed = SlashInput::for_submission("/diagnose logs", &registry)
        .submission_command()
        .unwrap();
    assert_eq!(parsed.command, command);
    assert_eq!(&"/diagnose logs"[parsed.arguments_range], "logs");
}

#[test]
fn recognized_completed_names_become_atomic_only_outside_the_name() {
    let registry = builtins();

    assert_eq!(
        SlashInput::at_cursor("/review details", "/review details".len(), &registry)
            .command_element_range(),
        Some(0..7)
    );
    assert_eq!(
        SlashInput::at_cursor("/review details", 4, &registry).command_element_range(),
        None
    );
    assert_eq!(
        SlashInput::at_cursor("/quit details", "/quit details".len(), &registry)
            .command_element_range(),
        None
    );
}

#[test]
fn every_builtin_round_trips_through_completion_and_submission() {
    let registry = builtins();
    for command in super::super::slash_commands::built_in_slash_commands()
        .into_iter()
        .map(|(_, command)| SlashCommandItem::Builtin(command))
    {
        let draft = format!("/{}", command.command());
        let completion = SlashInput::at_cursor(&draft, draft.len(), &registry)
            .completion(&command)
            .unwrap();
        let parsed = SlashInput::for_submission(completion.replacement.trim(), &registry)
            .submission_command()
            .unwrap();
        assert_eq!(parsed.command, command);
    }
}
