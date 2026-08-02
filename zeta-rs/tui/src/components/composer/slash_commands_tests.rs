use super::{TuiSlashCommandAction, built_in_slash_command_definitions, built_in_slash_commands};
use zeta_slash_commands::SlashCommandArgumentMode;

#[test]
fn builtins_follow_enum_presentation_order() {
    assert_eq!(
        built_in_slash_commands(),
        vec![
            ("status", TuiSlashCommandAction::Status),
            ("skills", TuiSlashCommandAction::Skills),
            ("mcp", TuiSlashCommandAction::Mcp),
            ("resume", TuiSlashCommandAction::Resume),
            ("clear", TuiSlashCommandAction::Clear),
            ("config", TuiSlashCommandAction::Config),
            ("fork", TuiSlashCommandAction::Fork),
            ("help", TuiSlashCommandAction::Help),
            ("model", TuiSlashCommandAction::Model),
            ("theme", TuiSlashCommandAction::Theme),
            ("new", TuiSlashCommandAction::New),
            ("quit", TuiSlashCommandAction::Quit),
            ("exit", TuiSlashCommandAction::Exit),
        ]
    );
    assert_eq!(built_in_slash_command_definitions().len(), 13);
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
        TuiSlashCommandAction::Theme.argument_mode(),
        SlashCommandArgumentMode::Optional
    );
    assert_eq!(
        TuiSlashCommandAction::Quit.argument_mode(),
        SlashCommandArgumentMode::None
    );
}
