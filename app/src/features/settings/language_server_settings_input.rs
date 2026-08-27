use zeta_app_server_protocol::protocol::config::LanguageServerModeDto;
use zui::input::{ElementState, Key, KeyEvent, NamedKey};
use zui::ui::ElementId;

use crate::NativeApp;
use crate::language_server_settings::{
    LANGUAGE_SERVER_BASH, LANGUAGE_SERVER_EXECUTABLE_INPUT, LANGUAGE_SERVER_JSON,
    LANGUAGE_SERVER_MODE_AUTOMATIC, LANGUAGE_SERVER_MODE_ENABLED, LANGUAGE_SERVER_RUST,
    LANGUAGE_SERVER_SETTINGS_CLOSE, LANGUAGE_SERVER_SETTINGS_RESET, LANGUAGE_SERVER_SETTINGS_SAVE,
    LANGUAGE_SERVER_SWITCH, LanguageServerSettingsTarget,
};
use crate::terminal_input::text_input_command;
use zeta_settings::SETTINGS_NAV_APPEARANCE;
use zeta_settings::SETTINGS_NAV_BACK;
use zeta_settings::SETTINGS_NAV_GENERAL;
use zeta_settings::SETTINGS_NAV_KEYBINDINGS;
use zeta_settings::SETTINGS_NAV_LANGUAGE_SERVERS;
use zeta_settings::SETTINGS_SEARCH_INPUT;
use zeta_settings::SettingsPageSection;

impl NativeApp {
    pub(super) fn activate_language_server_settings_element(&mut self, id: ElementId) -> bool {
        if !self.workbench_host.tab_part().is_settings() {
            return false;
        }
        match id {
            SETTINGS_NAV_BACK => self.close_language_server_settings(),
            SETTINGS_NAV_GENERAL => self.settings_section = SettingsPageSection::General,
            SETTINGS_NAV_LANGUAGE_SERVERS => {
                self.settings_section = SettingsPageSection::LanguageServers
            }
            SETTINGS_NAV_APPEARANCE => self.settings_section = SettingsPageSection::Appearance,
            SETTINGS_NAV_KEYBINDINGS => self.settings_section = SettingsPageSection::Keybindings,
            LANGUAGE_SERVER_SETTINGS_CLOSE => self.close_language_server_settings(),
            LANGUAGE_SERVER_RUST => self
                .language_server_settings
                .select_server(LanguageServerSettingsTarget::RustAnalyzer),
            LANGUAGE_SERVER_JSON => self
                .language_server_settings
                .select_server(LanguageServerSettingsTarget::Json),
            LANGUAGE_SERVER_BASH => self
                .language_server_settings
                .select_server(LanguageServerSettingsTarget::Bash),
            LANGUAGE_SERVER_MODE_AUTOMATIC => self
                .language_server_settings
                .select_mode(LanguageServerModeDto::Automatic),
            LANGUAGE_SERVER_MODE_ENABLED => self
                .language_server_settings
                .select_mode(LanguageServerModeDto::Enabled),
            LANGUAGE_SERVER_SWITCH => {
                let enabled = !self.language_server_settings.is_enabled();
                self.language_server_settings.set_enabled(enabled);
            }
            LANGUAGE_SERVER_SETTINGS_RESET => self.reset_language_server_settings(),
            LANGUAGE_SERVER_SETTINGS_SAVE => self.save_language_server_settings(),
            _ => return false,
        }
        true
    }

    pub(super) fn route_language_server_settings_keyboard(&mut self, event: &KeyEvent) -> bool {
        if !self.workbench_host.tab_part().is_settings() || event.state != ElementState::Pressed {
            return false;
        }
        if event.logical_key == Key::Named(NamedKey::Escape) {
            self.close_language_server_settings();
            self.rebuild_presentation();
            self.request_redraw();
            return true;
        }
        if self.ui_dispatch.is_focused(SETTINGS_SEARCH_INPUT) {
            if let Some(command) = text_input_command(event, self.modifiers) {
                self.language_server_settings.apply_search(command);
                self.caret_blink.activity(std::time::Instant::now());
                self.rebuild_presentation();
                self.request_redraw();
            }
            return true;
        }
        if self
            .ui_dispatch
            .is_focused(LANGUAGE_SERVER_EXECUTABLE_INPUT)
        {
            if event.logical_key == Key::Named(NamedKey::Enter) {
                self.save_language_server_settings();
                self.rebuild_presentation();
                self.request_redraw();
                return true;
            }
            if let Some(command) = text_input_command(event, self.modifiers) {
                self.language_server_settings.apply_executable(command);
                self.caret_blink.activity(std::time::Instant::now());
                self.rebuild_presentation();
                self.request_redraw();
                return true;
            }
        }
        if matches!(
            event.logical_key,
            Key::Named(
                NamedKey::Tab
                    | NamedKey::ArrowLeft
                    | NamedKey::ArrowRight
                    | NamedKey::ArrowUp
                    | NamedKey::ArrowDown
                    | NamedKey::Enter
            )
        ) {
            return self.dispatch_primary_keyboard_input(event);
        }
        false
    }

    fn save_language_server_settings(&mut self) {
        if !self.language_server_settings.can_save() {
            return;
        }
        let (expected_revision, server_id, config) =
            match self.language_server_settings.configuration() {
                Ok(configuration) => configuration,
                Err(error) => {
                    self.language_server_settings.save_failed(error);
                    return;
                }
            };
        self.language_server_settings.saving();
        let result = self
            .agent_session
            .as_ref()
            .ok_or_else(|| "App Server session is unavailable".to_owned())
            .and_then(|session| {
                session
                    .configure_language_server(expected_revision, server_id.to_owned(), config)
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(_) => self.language_server_settings.save_succeeded(),
            Err(error) => self.language_server_settings.save_failed(error),
        }
    }

    fn reset_language_server_settings(&mut self) {
        if !self.language_server_settings.can_reset() {
            return;
        }
        let (expected_revision, server_id) = match self.language_server_settings.reset_target() {
            Ok(target) => target,
            Err(error) => {
                self.language_server_settings.save_failed(error);
                return;
            }
        };
        self.language_server_settings.saving();
        let result = self
            .agent_session
            .as_ref()
            .ok_or_else(|| "App Server session is unavailable".to_owned())
            .and_then(|session| {
                session
                    .remove_language_server_configuration(expected_revision, server_id.to_owned())
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(_) => self.language_server_settings.save_succeeded(),
            Err(error) => self.language_server_settings.save_failed(error),
        }
    }

    fn close_language_server_settings(&mut self) {
        self.close_settings_tab();
    }
}
