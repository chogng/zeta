use crate::{
    SlashCommandArgumentMode, SlashCommandCatalog, SlashCommandDefinition, SlashCommandInput,
};

fn catalog() -> SlashCommandCatalog {
    SlashCommandCatalog::new([
        SlashCommandDefinition {
            name: "model".into(),
            description: "select a model".into(),
            argument_mode: SlashCommandArgumentMode::Optional,
        },
        SlashCommandDefinition {
            name: "status".into(),
            description: "show status".into(),
            argument_mode: SlashCommandArgumentMode::None,
        },
    ])
    .unwrap()
}

#[test]
fn query_completion_and_submission_share_one_grammar() {
    let catalog = catalog();
    let input = SlashCommandInput::at_cursor("/mo", 3, &catalog);
    let matches = input.matching_commands().unwrap();
    assert_eq!(matches.len(), 1);
    let completion = input.completion(&matches[0]).unwrap();
    assert_eq!(completion.range, 0..3);
    assert_eq!(completion.replacement, "/model ");

    let invocation = SlashCommandInput::for_submission("/model provider/name", &catalog)
        .invocation()
        .unwrap();
    assert_eq!(invocation.command.name, "model");
    assert_eq!(
        &"/model provider/name"[invocation.arguments_range],
        "provider/name"
    );
}

#[test]
fn commands_without_arguments_reject_extra_text() {
    let catalog = catalog();
    assert!(
        SlashCommandInput::for_submission("/status now", &catalog)
            .invocation()
            .is_none()
    );
    assert!(
        SlashCommandInput::for_submission(" /status", &catalog)
            .invocation()
            .is_none()
    );
}
