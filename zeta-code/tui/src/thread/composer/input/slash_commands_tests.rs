use super::{TuiSlashCommandAction, built_in_slash_command_definitions};
use zeta_slash_commands::SlashCommandArgumentMode;

#[test]
fn builtins_follow_enum_presentation_order() {
    let definitions = built_in_slash_command_definitions();
    assert_eq!(
        definitions
            .iter()
            .map(|definition| definition.name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "status",
            "statusline",
            "skills",
            "mcp",
            "resume",
            "archive",
            "connectors",
            "rewind",
            "config",
            "add-dir",
            "fork",
            "help",
            "shortcuts",
            "export",
            "model",
            "theme",
            "new",
            "quit",
            "sessions",
            "agents",
            "subagents",
            "queue",
        ]
    );
    assert_eq!(definitions.len(), 22);
}

#[test]
fn builtins_declare_argument_support() {
    assert_eq!(
        TuiSlashCommandAction::Model.argument_mode(),
        SlashCommandArgumentMode::Optional
    );
    assert_eq!(
        TuiSlashCommandAction::Fork.argument_mode(),
        SlashCommandArgumentMode::Optional
    );
    assert_eq!(
        TuiSlashCommandAction::Rewind.argument_mode(),
        SlashCommandArgumentMode::Optional
    );
    assert_eq!(
        TuiSlashCommandAction::AddDir.argument_mode(),
        SlashCommandArgumentMode::Optional
    );
    assert_eq!(
        TuiSlashCommandAction::Theme.argument_mode(),
        SlashCommandArgumentMode::Optional
    );
    assert_eq!(
        TuiSlashCommandAction::Export.argument_mode(),
        SlashCommandArgumentMode::Optional
    );
    assert_eq!(
        TuiSlashCommandAction::Quit.argument_mode(),
        SlashCommandArgumentMode::None
    );
    assert_eq!(
        TuiSlashCommandAction::Archive.argument_mode(),
        SlashCommandArgumentMode::None
    );
}
