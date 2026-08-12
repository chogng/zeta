use crate::{
    SlashCommandArgumentMode, SlashCommandCatalog, SlashCommandDefinition, SlashCommandOrigin,
};

fn command(name: &str) -> SlashCommandDefinition {
    SlashCommandDefinition {
        name: name.into(),
        description: "inspect the workspace".into(),
        argument_mode: SlashCommandArgumentMode::Optional,
    }
}

#[test]
fn catalog_preserves_local_then_server_order_and_origin() {
    let catalog = SlashCommandCatalog::with_local_and_server(
        [command("model")],
        [command("diagnose"), command("check-tests")],
    )
    .unwrap();

    assert_eq!(
        catalog
            .commands()
            .iter()
            .map(|command| (
                command.name.as_str(),
                catalog.origin(&command.name).unwrap()
            ))
            .collect::<Vec<_>>(),
        vec![
            ("model", SlashCommandOrigin::Local),
            ("diagnose", SlashCommandOrigin::Server),
            ("check-tests", SlashCommandOrigin::Server),
        ]
    );
}

#[test]
fn catalog_rejects_invalid_duplicate_and_blank_definitions() {
    assert!(
        SlashCommandCatalog::new([command("Diagnose")])
            .unwrap_err()
            .0
            .contains("invalid slash command name")
    );
    assert_eq!(
        SlashCommandCatalog::with_local_and_server([command("diagnose")], [command("diagnose")])
            .unwrap_err()
            .0,
        "duplicate slash command name 'diagnose'"
    );
    let mut blank = command("blank");
    blank.description = "  ".into();
    assert_eq!(
        SlashCommandCatalog::new([blank]).unwrap_err().0,
        "slash command 'blank' must have a description"
    );
}

#[test]
fn skill_definitions_keep_a_distinct_dispatch_origin() {
    let catalog = SlashCommandCatalog::with_local_server_and_skills(
        [command("status")],
        [command("diagnose")],
        [command("commit")],
    )
    .unwrap();

    assert_eq!(catalog.origin("status"), Some(SlashCommandOrigin::Local));
    assert_eq!(catalog.origin("diagnose"), Some(SlashCommandOrigin::Server));
    assert_eq!(catalog.origin("commit"), Some(SlashCommandOrigin::Skill));
}
