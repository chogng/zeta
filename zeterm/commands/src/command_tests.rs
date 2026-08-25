use super::ZetermCommandId;

#[test]
fn command_ids_are_unique_and_round_trip() {
    let commands = ZetermCommandId::ALL;
    let ids: Vec<_> = commands.iter().map(|command| command.id()).collect();
    let unique_ids: std::collections::HashSet<_> = ids.iter().copied().collect();

    assert_eq!(ids.len(), unique_ids.len());
    for command in commands {
        assert_eq!(ZetermCommandId::from_id(command.id()), Some(command));
    }
}

#[test]
fn only_currently_supported_commands_are_bindable() {
    assert_eq!(
        ZetermCommandId::from_id("workbench.action.toggleComposerMode"),
        None
    );
    assert_eq!(
        ZetermCommandId::bindable_from_id("workbench.action.toggleSideBar"),
        Some(ZetermCommandId::ToggleSessionSidebar)
    );
    assert_eq!(
        ZetermCommandId::bindable_from_id("workbench.action.newSession"),
        None
    );
    assert_eq!(
        ZetermCommandId::bindable_from_id("workbench.action.pickExecutionLocation"),
        Some(ZetermCommandId::PickExecutionLocation)
    );
    assert_eq!(
        ZetermCommandId::bindable_from_id("workbench.action.manageRemoteTunnels"),
        Some(ZetermCommandId::ManageRemoteTunnels)
    );
}
