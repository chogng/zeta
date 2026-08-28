//! Unified command entry for Workbench-owned UI and state.

use zeta_commands::AppCommandId;
use zeta_commands::CommandRequest;
use zui::ui::ElementId;

use crate::ADD_SESSION;
use crate::TAB_CONTAINER_TOGGLE;
use crate::WORKSPACE_PANE_TOGGLE;

/// Result of routing one command through Workbench.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WorkbenchCommandDispatch {
    /// Workbench applied the complete state transition.
    Handled,
    /// The command belongs to the capability mounted by the application leaf.
    Capability(CommandRequest),
}

/// Resolves a Workbench or mounted-capability element into its stable product command.
pub fn command_request_for_element(element: ElementId) -> Option<CommandRequest> {
    let command = match element {
        TAB_CONTAINER_TOGGLE => AppCommandId::ToggleTabContainer,
        WORKSPACE_PANE_TOGGLE => AppCommandId::ToggleWorkspacePane,
        ADD_SESSION => AppCommandId::AddSession,
        _ => {
            return zeta_files::command_request_for_element(element)
                .or_else(|| zeta_session::interaction::command_request_for_element(element));
        }
    };
    Some(command.into())
}

#[cfg(test)]
#[path = "command_tests.rs"]
mod tests;
