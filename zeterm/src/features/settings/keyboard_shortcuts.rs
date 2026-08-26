use std::time::Instant;

use zeta_keybindings_host::recording_chord;
use zeterm_keybinding_ui::KeyboardShortcutRow;
use zeterm_keybinding_ui::KeyboardShortcutsIds;
use zeterm_keybinding_ui::KeyboardShortcutsState as GenericKeyboardShortcutsState;
use zui::input::{ElementState, Key, KeyEvent, NamedKey};
use zui::ui::ElementId;

use crate::NativeApp;
use crate::keybindings::KeybindingsResourcePoll;
use crate::keybindings::NativeKeybindings;
use crate::shell_interaction::WINDOW;
use zeta_commands::ZetermCommandId;
use zeta_settings::SettingsPageSection;

const SHORTCUT_SCOPE: u32 = 3;
pub(crate) const KEYBOARD_SHORTCUTS: ElementId = ElementId::scoped(SHORTCUT_SCOPE, 1);
const KEYBOARD_SHORTCUTS_CLOSE: ElementId = ElementId::scoped(SHORTCUT_SCOPE, 2);

pub(crate) type KeyboardShortcutsState = GenericKeyboardShortcutsState<ZetermCommandId>;

pub(crate) const fn keyboard_shortcuts_ids() -> KeyboardShortcutsIds {
    KeyboardShortcutsIds::new(WINDOW, KEYBOARD_SHORTCUTS, KEYBOARD_SHORTCUTS_CLOSE)
}

pub(crate) fn keyboard_shortcut_rows(
    keybindings: &NativeKeybindings,
) -> Vec<KeyboardShortcutRow<'_, ZetermCommandId>> {
    ZetermCommandId::BINDABLE
        .into_iter()
        .map(|command| {
            KeyboardShortcutRow::new(
                command,
                row_element(command),
                command.label(),
                keybindings.binding_for_command(command),
            )
        })
        .collect()
}

pub(super) fn row_element(command: ZetermCommandId) -> ElementId {
    let index = ZetermCommandId::BINDABLE
        .into_iter()
        .position(|candidate| candidate == command)
        .expect("bindable command must have a stable row");
    ElementId::scoped(SHORTCUT_SCOPE, 10 + index as u32)
}

fn command_for_row(id: ElementId) -> Option<ZetermCommandId> {
    ZetermCommandId::BINDABLE
        .into_iter()
        .find(|command| row_element(*command) == id)
}

impl NativeApp {
    pub(super) fn activate_keyboard_shortcuts_element(&mut self, id: ElementId) -> bool {
        if id == KEYBOARD_SHORTCUTS_CLOSE {
            if !self.keyboard_shortcuts.is_visible() {
                return false;
            }
            self.keyboard_shortcuts.close();
        } else if let Some(command) = command_for_row(id) {
            if !self.keyboard_shortcuts.is_visible() {
                if !self.workbench_host.tab_part().is_settings()
                    || self.settings_section != SettingsPageSection::Keybindings
                {
                    return false;
                }
                self.keyboard_shortcuts.toggle();
            }
            self.keyboard_shortcuts.start_recording(command);
        } else {
            return false;
        }
        self.rebuild_presentation();
        self.request_redraw();
        true
    }

    pub(super) fn route_keyboard_shortcuts_keyboard(&mut self, event: &KeyEvent) -> bool {
        if !self.keyboard_shortcuts.is_visible() || event.state != ElementState::Pressed {
            return false;
        }
        if event.logical_key == Key::Named(NamedKey::Escape) {
            if self.keyboard_shortcuts.is_recording() {
                self.keyboard_shortcuts.cancel_recording();
            } else {
                self.keyboard_shortcuts.close();
            }
            self.rebuild_presentation();
            self.request_redraw();
            return true;
        }
        if !self.keyboard_shortcuts.is_recording() {
            return false;
        }
        if event.repeat {
            return true;
        }
        if let Some(chord) = recording_chord(event, self.modifiers, self.keybindings.platform()) {
            self.keyboard_shortcuts.record(chord, Instant::now());
            self.rebuild_presentation();
            self.request_redraw();
        }
        true
    }

    pub(super) fn advance_keyboard_shortcuts(&mut self, now: Instant) {
        let Some(commit) = self.keyboard_shortcuts.advance(now) else {
            return;
        };
        match self.keybindings_resource.update_command_binding(
            commit.command,
            &commit.keybinding,
            now,
        ) {
            Ok(()) => match self.keybindings_resource.poll(now, &mut self.keybindings) {
                KeybindingsResourcePoll::Rejected(error) => {
                    self.keyboard_shortcuts.save_failed(error);
                }
                KeybindingsResourcePoll::Unchanged | KeybindingsResourcePoll::Updated => {
                    self.keyboard_shortcuts.saved(commit.command.label());
                }
            },
            Err(error) => self.keyboard_shortcuts.save_failed(error),
        }
        self.rebuild_presentation();
        self.request_redraw();
    }

    pub(super) fn keyboard_shortcuts_deadline(&self) -> Option<Instant> {
        self.keyboard_shortcuts.deadline()
    }
}

#[cfg(test)]
#[path = "keyboard_shortcuts_tests.rs"]
mod tests;
