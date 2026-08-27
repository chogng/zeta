//! Native host boundary between the reusable Workbench model and app features.

use crate::NativeApp;

pub(crate) use pane_input::PaneBinding;
pub(crate) use tab_container_state::TabContainerState;
pub(crate) use terminal_workspace::{TerminalReadyOutcome, TerminalWorkspace};
pub(crate) use zeta_workbench_host::PaneHostScope;
pub(crate) use zeta_workbench_host::{
    InspectorPartState, PaneGroupId as PaneId, PaneInput, PaneInputKind, PanePart,
    PaneSplitDirection, PaneSplitId, TabGroupId, TabInput, TabInputChange, TabInputKey, TabPart,
};

#[cfg(test)]
pub(crate) type PaneHost = zeta_workbench_host::PaneHost<PaneBinding>;
pub(crate) type PaneViewMount<'a> = zeta_workbench_host::PaneMount<'a, PaneBinding>;
pub(crate) type WorkbenchHost = zeta_workbench_host::WorkbenchHost<PaneBinding>;

#[path = "workbench_host/inspector_part.rs"]
pub(crate) mod inspector_part;
#[path = "workbench_host/pane_input.rs"]
pub(crate) mod pane_input;
#[path = "workbench_host/tab_container.rs"]
pub(crate) mod tab_container;
#[path = "workbench_host/tab_container_state.rs"]
pub(crate) mod tab_container_state;
#[path = "workbench_host/tab_container_toolbar.rs"]
pub(crate) mod tab_container_toolbar;
#[path = "workbench_host/terminal_workspace.rs"]
pub(crate) mod terminal_workspace;
#[path = "workbench_host/titlebar.rs"]
pub(crate) mod titlebar;

impl NativeApp {
    /// Selects the singleton Settings workbench item and prepares its feature-owned state.
    pub(super) fn activate_settings_tab(&mut self) {
        if !self.workbench_host.workbench().tab_part().is_settings() {
            self.settings_section = zeta_settings::SettingsPageSection::LanguageServers;
        }
        let _ = self.workbench_host.workbench_mut().activate_settings();
        self.language_server_settings.open();
        self.keyboard_shortcuts.close();
        let _ = self.git_branch_context_menu.dismiss();
        let _ = self.workspace_path_picker.dismiss();
        let _ = self.remote_connection_picker.dismiss();
        self.dismiss_remote_connection_manager();
        self.dismiss_remote_tunnel_manager();
        self.dismiss_session_context_menu();
        self.pending_focus = Some(zeta_settings::SETTINGS_SEARCH_INPUT);
        self.keybindings.cancel_chord();
    }

    /// Returns to the last selected session without fabricating a session for Settings.
    pub(super) fn activate_session_workbench_tab(&mut self) {
        let was_terminal = self.workspace_surface.is_terminal();
        let _ = self
            .workbench_host
            .workbench_mut()
            .tab_part_mut()
            .activate_last_session();
        if let Some(session_id) = self
            .workbench_host
            .workbench()
            .tab_part()
            .selected_session()
            .cloned()
        {
            let _ = self.activate_terminal_for_session(&session_id);
            if !was_terminal {
                let _ = self.bind_agent_pane();
            }
        }
        self.language_server_settings.close();
        self.keyboard_shortcuts.close();
        self.pending_focus = Some(if self.workspace_surface.is_editor() {
            crate::shell_interaction::FILE_EDITOR_DOCUMENT
        } else {
            crate::shell_interaction::COMPOSER
        });
    }

    pub(super) fn close_settings_tab(&mut self) {
        self.activate_session_workbench_tab();
    }
}
