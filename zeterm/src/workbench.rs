use zui::ui::ElementId;

use crate::NativeApp;
use crate::shell_interaction::ACTIVE_SESSION_TAB;

/// The top-level item displayed by the workbench navigator.
///
/// A session item keeps the existing presentation identity while its authoritative session
/// lifecycle remains owned by the App Server adapter. Settings is a singleton workbench item, not
/// a synthetic App Server session.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkbenchItem {
    Session(ElementId),
    Settings,
}

impl Default for WorkbenchItem {
    fn default() -> Self {
        Self::Session(ACTIVE_SESSION_TAB)
    }
}

impl WorkbenchItem {
    pub(crate) const fn is_settings(self) -> bool {
        matches!(self, Self::Settings)
    }

    pub(crate) const fn element_id(self) -> ElementId {
        match self {
            Self::Session(id) => id,
            Self::Settings => crate::shell_interaction::SETTINGS_WORKBENCH_TAB,
        }
    }
}

impl NativeApp {
    /// Selects the singleton Settings workbench item and prepares its feature-owned state.
    pub(crate) fn activate_settings_tab(&mut self) {
        if !self.workbench_item.is_settings() {
            self.settings_section = zeta_settings::SettingsPageSection::LanguageServers;
        }
        self.workbench_item = WorkbenchItem::Settings;
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
    pub(crate) fn activate_session_workbench_tab(&mut self, tab_id: ElementId) {
        self.workbench_item = WorkbenchItem::Session(tab_id);
        self.language_server_settings.close();
        self.keyboard_shortcuts.close();
        self.pending_focus = Some(if self.workspace_surface.is_editor() {
            crate::shell_interaction::FILE_EDITOR_DOCUMENT
        } else {
            crate::shell_interaction::COMPOSER
        });
    }

    pub(crate) fn synchronize_session_workbench_tab(&mut self, tab_id: ElementId) {
        if !self.workbench_item.is_settings() {
            self.workbench_item = WorkbenchItem::Session(tab_id);
        }
    }

    pub(crate) fn close_settings_tab(&mut self) {
        self.activate_session_workbench_tab(self.selected_session_tab);
    }
}

#[cfg(test)]
mod tests {
    use super::WorkbenchItem;
    use crate::shell_interaction::{ACTIVE_SESSION_TAB, SETTINGS_WORKBENCH_TAB};
    use zui::ui::ElementId;

    #[test]
    fn settings_is_a_singleton_workbench_item_with_its_own_element() {
        let settings = WorkbenchItem::Settings;

        assert!(settings.is_settings());
        assert_eq!(settings.element_id(), SETTINGS_WORKBENCH_TAB);
        assert_eq!(
            WorkbenchItem::default(),
            WorkbenchItem::Session(ACTIVE_SESSION_TAB)
        );
    }

    #[test]
    fn session_workbench_items_keep_their_session_element_identity() {
        let session = ElementId::scoped(14, 101);

        assert_eq!(WorkbenchItem::Session(session).element_id(), session);
        assert!(!WorkbenchItem::Session(session).is_settings());
    }
}
