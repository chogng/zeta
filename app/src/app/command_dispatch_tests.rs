use zeta_commands::AppCommandId;

use super::builtin_command_registry;

#[test]
fn every_capability_command_has_an_application_handler() {
    let registry = builtin_command_registry();

    let capability_commands = AppCommandId::ALL
        .into_iter()
        .filter(|command| *command != AppCommandId::ToggleTabContainer)
        .collect::<Vec<_>>();
    assert_eq!(registry.len(), capability_commands.len());
    for command in capability_commands {
        assert!(
            registry.contains(command),
            "missing handler for {}",
            command.id()
        );
    }
}

#[test]
fn only_commands_with_current_execution_are_user_bindable() {
    assert_eq!(
        AppCommandId::bindable_from_id("workbench.action.toggleTabContainer"),
        Some(AppCommandId::ToggleTabContainer)
    );
    assert_eq!(
        AppCommandId::bindable_from_id("workbench.action.newSession"),
        None
    );
}
