use super::{NativeCommand, command_for_element};
use crate::shell_interaction::{
    AGENT_FILES, AGENT_FILES_REFRESH, CONTEXT_DIFF, CONTEXT_WORKING_DIRECTORY,
    SESSION_SIDEBAR_TOGGLE,
};

#[test]
fn element_entry_points_map_to_stable_product_commands() {
    assert_eq!(
        command_for_element(SESSION_SIDEBAR_TOGGLE),
        Some(NativeCommand::ToggleSessionSidebar)
    );
    assert_eq!(
        command_for_element(AGENT_FILES),
        Some(NativeCommand::SelectAgentPane(
            crate::shell_interaction::AgentSidebarPaneAction::Files
        ))
    );
    assert_eq!(
        command_for_element(AGENT_FILES_REFRESH)
            .expect("refresh command")
            .id(),
        "workbench.action.refreshAgentFiles"
    );
    assert_eq!(
        command_for_element(CONTEXT_DIFF)
            .expect("workspace diff command")
            .id(),
        "workbench.action.showWorkspaceDiff"
    );
    assert_eq!(
        command_for_element(CONTEXT_WORKING_DIRECTORY)
            .expect("workspace picker command")
            .id(),
        "workbench.action.pickWorkingDirectory"
    );
}

#[test]
fn only_commands_with_current_execution_are_user_bindable() {
    assert_eq!(
        NativeCommand::bindable_from_id("workbench.action.toggleSideBar"),
        Some(NativeCommand::ToggleSessionSidebar)
    );
    assert_eq!(
        NativeCommand::bindable_from_id("workbench.action.newSession"),
        None
    );
}
