use super::{TuiSlashCommandAction, built_in_slash_command_definitions, built_in_slash_commands};
use zeta_slash_commands::SlashCommandArgumentMode;

#[test]
fn builtins_follow_enum_presentation_order() {
    assert_eq!(
        built_in_slash_commands(),
        vec![
            ("status", TuiSlashCommandAction::Status),
            ("statusline", TuiSlashCommandAction::StatusLine),
            ("skills", TuiSlashCommandAction::Skills),
            ("mcp", TuiSlashCommandAction::Mcp),
            ("resume", TuiSlashCommandAction::Resume),
            ("archive", TuiSlashCommandAction::Archive),
            ("connectors", TuiSlashCommandAction::Connectors),
            ("rewind", TuiSlashCommandAction::Rewind),
            ("clear", TuiSlashCommandAction::Clear),
            ("config", TuiSlashCommandAction::Config),
            ("add-dir", TuiSlashCommandAction::AddDir),
            ("fork", TuiSlashCommandAction::Fork),
            ("help", TuiSlashCommandAction::Help),
            ("shortcuts", TuiSlashCommandAction::Shortcuts),
            ("copy", TuiSlashCommandAction::Copy),
            ("export", TuiSlashCommandAction::Export),
            ("model", TuiSlashCommandAction::Model),
            ("theme", TuiSlashCommandAction::Theme),
            ("new", TuiSlashCommandAction::New),
            ("quit", TuiSlashCommandAction::Quit),
            ("sessions", TuiSlashCommandAction::Sessions),
            ("agents", TuiSlashCommandAction::Agents),
            ("subagents", TuiSlashCommandAction::Subagents),
            ("queue", TuiSlashCommandAction::Queue),
        ]
    );
    assert_eq!(built_in_slash_command_definitions().len(), 24);
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
