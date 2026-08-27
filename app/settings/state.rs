use std::time::Instant;

use zeta_commands::AppCommandId;
use zeta_keybinding::HostPlatform;
use zeta_keybindings_host::recording_chord;
use zui::input::ElementState;
use zui::input::Key;
use zui::input::KeyEvent;
use zui::input::ModifiersState;
use zui::input::NamedKey;
use zui::ui::ElementId;
use zui::ui::TextInput;
use zui::ui::TextInputCommand;
use zui::ui::TextInputCompositionEvent;

use crate::KEYBOARD_SHORTCUTS_CLOSE;
use crate::KeyboardShortcutsState;
use crate::SETTINGS_NAV_APPEARANCE;
use crate::SETTINGS_NAV_BACK;
use crate::SETTINGS_NAV_GENERAL;
use crate::SETTINGS_NAV_KEYBINDINGS;
use crate::SettingsPageSection;
use crate::keybindings::ShortcutCommit;
use crate::keybindings::command_for_keyboard_shortcut_row;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettingsActivation {
    Ignored,
    Changed,
    Close,
}

pub struct SettingsState {
    section: SettingsPageSection,
    search: TextInput,
    keyboard_shortcuts: KeyboardShortcutsState,
}

impl Default for SettingsState {
    fn default() -> Self {
        Self {
            section: SettingsPageSection::default(),
            search: TextInput::default(),
            keyboard_shortcuts: KeyboardShortcutsState::default(),
        }
    }
}

impl SettingsState {
    pub const fn section(&self) -> SettingsPageSection {
        self.section
    }

    pub const fn search_input(&self) -> &TextInput {
        &self.search
    }

    pub fn selected_search_text(&self) -> Option<&str> {
        self.search.selected_text()
    }

    pub fn apply_search(&mut self, command: TextInputCommand) {
        self.search.apply(command);
    }

    pub fn apply_search_composition(&mut self, event: TextInputCompositionEvent) {
        self.search.apply_composition(event);
    }

    pub fn cancel_search_composition(&mut self) {
        self.search.cancel_composition();
    }

    pub const fn keyboard_shortcuts(&self) -> &KeyboardShortcutsState {
        &self.keyboard_shortcuts
    }

    pub fn reopen(&mut self) {
        self.keyboard_shortcuts.close();
    }

    pub fn open_keyboard_shortcuts(&mut self) {
        self.section = SettingsPageSection::Keybindings;
        if !self.keyboard_shortcuts.is_visible() {
            self.keyboard_shortcuts.toggle();
        }
    }

    pub fn close_keyboard_shortcuts(&mut self) {
        self.keyboard_shortcuts.close();
    }

    pub fn close(&mut self) {
        self.search.cancel_composition();
        self.keyboard_shortcuts.close();
    }

    pub fn activate(&mut self, id: ElementId) -> SettingsActivation {
        match id {
            SETTINGS_NAV_BACK => SettingsActivation::Close,
            SETTINGS_NAV_GENERAL => self.select(SettingsPageSection::General),
            SETTINGS_NAV_APPEARANCE => self.select(SettingsPageSection::Appearance),
            SETTINGS_NAV_KEYBINDINGS => self.select(SettingsPageSection::Keybindings),
            KEYBOARD_SHORTCUTS_CLOSE if self.keyboard_shortcuts.is_visible() => {
                self.keyboard_shortcuts.close();
                SettingsActivation::Changed
            }
            _ => {
                let Some(command) = command_for_keyboard_shortcut_row(id) else {
                    return SettingsActivation::Ignored;
                };
                if self.section != SettingsPageSection::Keybindings {
                    return SettingsActivation::Ignored;
                }
                if !self.keyboard_shortcuts.is_visible() {
                    self.keyboard_shortcuts.toggle();
                }
                self.keyboard_shortcuts.start_recording(command);
                SettingsActivation::Changed
            }
        }
    }

    pub fn route_keyboard_shortcut_input(
        &mut self,
        event: &KeyEvent,
        modifiers: ModifiersState,
        platform: HostPlatform,
        now: Instant,
    ) -> bool {
        if !self.keyboard_shortcuts.is_visible() || event.state != ElementState::Pressed {
            return false;
        }
        if event.logical_key == Key::Named(NamedKey::Escape) {
            if self.keyboard_shortcuts.is_recording() {
                self.keyboard_shortcuts.cancel_recording();
            } else {
                self.keyboard_shortcuts.close();
            }
            return true;
        }
        if !self.keyboard_shortcuts.is_recording() {
            return false;
        }
        if !event.repeat
            && let Some(chord) = recording_chord(event, modifiers, platform)
        {
            self.keyboard_shortcuts.record(chord, now);
        }
        true
    }

    pub fn advance_keyboard_shortcuts(
        &mut self,
        now: Instant,
    ) -> Option<ShortcutCommit<AppCommandId>> {
        self.keyboard_shortcuts.advance(now)
    }

    pub fn keyboard_shortcuts_saved(&mut self, command: AppCommandId) {
        self.keyboard_shortcuts.saved(command.label());
    }

    pub fn keyboard_shortcuts_save_failed(&mut self, error: impl Into<String>) {
        self.keyboard_shortcuts.save_failed(error);
    }

    pub fn keyboard_shortcuts_window_blurred(&mut self) {
        self.keyboard_shortcuts.window_blurred();
    }

    pub fn keyboard_shortcuts_deadline(&self) -> Option<Instant> {
        self.keyboard_shortcuts.deadline()
    }

    fn select(&mut self, section: SettingsPageSection) -> SettingsActivation {
        self.section = section;
        SettingsActivation::Changed
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
