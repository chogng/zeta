use super::DynamicSlashCommand;
use super::SlashCommand;
use super::SlashCommandArgumentMode;
use super::SlashCommandItem;
use super::SlashCommandRegistry;
use super::built_in_slash_commands;

#[test]
fn builtins_follow_enum_presentation_order() {
    assert_eq!(
        built_in_slash_commands(),
        vec![
            ("status", SlashCommand::Status),
            ("skills", SlashCommand::Skills),
            ("mcp", SlashCommand::Mcp),
            ("resume", SlashCommand::Resume),
            ("clear", SlashCommand::Clear),
            ("config", SlashCommand::Config),
            ("fork", SlashCommand::Fork),
            ("help", SlashCommand::Help),
            ("model", SlashCommand::Model),
            ("new", SlashCommand::New),
            ("quit", SlashCommand::Quit),
            ("exit", SlashCommand::Exit),
        ]
    );
}

#[test]
fn inline_argument_support_is_explicit_command_metadata() {
    assert_eq!(
        SlashCommand::Model.argument_mode(),
        SlashCommandArgumentMode::Optional
    );
    assert_eq!(
        SlashCommand::Fork.argument_mode(),
        SlashCommandArgumentMode::Optional
    );
    assert_eq!(
        SlashCommand::Quit.argument_mode(),
        SlashCommandArgumentMode::None
    );
}

#[test]
fn dynamic_registry_rejects_invalid_and_duplicate_names() {
    let duplicate = SlashCommandRegistry::with_dynamic_commands([DynamicSlashCommand {
        name: "status".into(),
        description: "duplicate".into(),
        argument_mode: SlashCommandArgumentMode::Optional,
    }]);
    assert_eq!(
        duplicate.unwrap_err(),
        "duplicate slash command name 'status'"
    );

    let invalid = SlashCommandRegistry::with_dynamic_commands([DynamicSlashCommand {
        name: "Bad Name".into(),
        description: "invalid".into(),
        argument_mode: SlashCommandArgumentMode::None,
    }]);
    assert!(
        invalid
            .unwrap_err()
            .starts_with("invalid slash command name 'Bad Name'")
    );
}

#[test]
fn dynamic_registry_appends_valid_commands_after_builtins() {
    let dynamic = DynamicSlashCommand {
        name: "diagnose".into(),
        description: "inspect the workspace".into(),
        argument_mode: SlashCommandArgumentMode::Optional,
    };
    let registry = SlashCommandRegistry::with_dynamic_commands([dynamic.clone()]).unwrap();

    assert_eq!(
        registry.matching("diag"),
        vec![SlashCommandItem::Dynamic(dynamic)]
    );
}
