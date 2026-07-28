use super::DynamicSlashCommand;
use super::SlashCommand;
use super::SlashCommandArgumentMode;
use super::SlashCommandItem;
use super::SlashCommandRegistry;
use super::built_in_slash_commands;

#[test]
fn built_in_commands_follow_enum_presentation_order() {
    assert_eq!(
        built_in_slash_commands(),
        vec![
            ("models", SlashCommand::Models),
            ("statusline", SlashCommand::Statusline),
            ("review", SlashCommand::Review),
            ("init", SlashCommand::Init),
            ("status", SlashCommand::Status),
            ("skills", SlashCommand::Skills),
            ("mcp", SlashCommand::Mcp),
            ("resume", SlashCommand::Resume),
            ("plugins", SlashCommand::Plugins),
            ("clear", SlashCommand::Clear),
            ("compact", SlashCommand::Compact),
            ("config", SlashCommand::Config),
            ("fast", SlashCommand::Fast),
            ("fork", SlashCommand::Fork),
            ("goal", SlashCommand::Goal),
            ("help", SlashCommand::Help),
            ("ide", SlashCommand::Ide),
            ("hooks", SlashCommand::Hooks),
            ("login", SlashCommand::Login),
            ("logout", SlashCommand::Logout),
            ("model", SlashCommand::Model),
            ("plan", SlashCommand::Plan),
            ("permissions", SlashCommand::Permissions),
            ("new", SlashCommand::New),
            ("quit", SlashCommand::Quit),
            ("exit", SlashCommand::Exit),
        ]
    );
}

#[test]
fn inline_argument_support_is_explicit_command_metadata() {
    assert_eq!(
        SlashCommand::Review.argument_mode(),
        SlashCommandArgumentMode::Optional
    );
    assert_eq!(
        SlashCommand::Goal.argument_mode(),
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
        name: "review".into(),
        description: "duplicate".into(),
        argument_mode: SlashCommandArgumentMode::Optional,
    }]);
    assert_eq!(
        duplicate.unwrap_err(),
        "duplicate slash command name 'review'"
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
