use super::SlashCommandCatalog;
use zeta_app_server_protocol::protocol::slash_commands::{
    SlashCommandArgumentModeDto, SlashCommandDefinition,
};

fn command(name: &str) -> SlashCommandDefinition {
    SlashCommandDefinition {
        name: name.into(),
        description: "inspect the workspace".into(),
        argument_mode: SlashCommandArgumentModeDto::Optional,
    }
}

#[test]
fn catalog_accepts_unique_canonical_names() {
    let catalog = SlashCommandCatalog::new([command("diagnose"), command("check-tests")]).unwrap();

    assert_eq!(catalog.definitions().len(), 2);
}

#[test]
fn catalog_rejects_invalid_and_duplicate_names() {
    let invalid = SlashCommandCatalog::new([command("Diagnose")]).unwrap_err();
    assert!(invalid.0.contains("invalid slash command name"));

    let duplicate =
        SlashCommandCatalog::new([command("diagnose"), command("diagnose")]).unwrap_err();
    assert_eq!(duplicate.0, "duplicate slash command name 'diagnose'");
}

#[test]
fn catalog_rejects_blank_descriptions() {
    let mut definition = command("diagnose");
    definition.description = "  ".into();

    let error = SlashCommandCatalog::new([definition]).unwrap_err();

    assert_eq!(error.0, "slash command 'diagnose' must have a description");
}
