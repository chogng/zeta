//! Product workbench actions shared by the titlebar, tabs, panes, and settings surfaces.

use crate::NativeApp;

#[path = "workbench/terminal_workspace.rs"]
pub(crate) mod terminal_workspace;

impl NativeApp {
    /// Selects the singleton Settings workbench item and prepares its feature-owned state.
    pub(super) fn activate_settings_tab(&mut self) {
        if !self.tab_inputs.is_settings() {
            self.settings_section = zeta_settings::SettingsPageSection::LanguageServers;
        }
        let _ = self.tab_inputs.activate_settings();
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
        let _ = self.tab_inputs.activate_last_session();
        if let Some(session_id) = self.tab_inputs.selected_session().cloned() {
            let _ = self.activate_terminal_for_session(&session_id);
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
