use std::time::Instant;

use zeta_terminal::{KeyModifiers, ScreenBuffer, TerminalCore, TerminalKey};
use zeta_ui::{TextInputCommand, TextInputSelectionMode};
use zeta_winit::{ElementState, Key, KeyEvent, ModifiersState, NamedKey};

use crate::NativeApp;
use crate::shell_interaction::COMPOSER;
use crate::terminal_selection::{read_clipboard_text, write_clipboard_text};
use zeta_ui_dispatch::{FocusDirection, NavigationAxis};

impl NativeApp {
    pub(super) fn keyboard_input(&mut self, event: KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        if self.active_screen() == ScreenBuffer::Alternate {
            self.direct_terminal_keyboard_input(&event);
        } else if !self.dispatch_primary_keyboard_input(&event) {
            self.composer_keyboard_input(&event);
        }
    }

    fn composer_keyboard_input(&mut self, event: &KeyEvent) {
        if is_clipboard_shortcut(&event.logical_key, "c", self.modifiers, false) {
            if !self.copy_composer_selection() {
                self.copy_terminal_selection();
            }
            return;
        }
        if is_clipboard_shortcut(&event.logical_key, "v", self.modifiers, false) {
            self.paste_into_composer();
            return;
        }
        if event.logical_key == Key::Named(NamedKey::Enter) {
            self.submit_composer_command();
            return;
        }
        if event.logical_key == Key::Named(NamedKey::Escape) {
            self.terminal_composer.cancel_composition();
            self.composer_changed();
            return;
        }
        let selection_mode = if self.modifiers.shift_key() {
            TextInputSelectionMode::Extend
        } else {
            TextInputSelectionMode::Move
        };
        let shortcut = self.modifiers.control_key() || self.modifiers.super_key();
        let command = match &event.logical_key {
            Key::Named(NamedKey::Backspace) => Some(TextInputCommand::Backspace),
            Key::Named(NamedKey::Delete) => Some(TextInputCommand::DeleteForward),
            Key::Named(NamedKey::ArrowLeft) => Some(TextInputCommand::MoveLeft(selection_mode)),
            Key::Named(NamedKey::ArrowRight) => Some(TextInputCommand::MoveRight(selection_mode)),
            Key::Named(NamedKey::Home) => Some(TextInputCommand::MoveToStart(selection_mode)),
            Key::Named(NamedKey::End) => Some(TextInputCommand::MoveToEnd(selection_mode)),
            Key::Character(text) if shortcut && text.eq_ignore_ascii_case("a") => {
                Some(TextInputCommand::SelectAll)
            }
            _ if !shortcut => event
                .text
                .as_ref()
                .map(|text| TextInputCommand::Insert(text.to_string())),
            _ => None,
        };
        if let Some(command) = command {
            self.terminal_composer.apply(command);
            self.composer_changed();
        }
    }

    fn dispatch_primary_keyboard_input(&mut self, event: &KeyEvent) -> bool {
        let Some(presentation) = self.presentation.as_ref() else {
            return false;
        };
        let frame = &presentation.interaction_frame;
        let outcome = if event.logical_key == Key::Named(NamedKey::Tab) {
            let direction = if self.modifiers.shift_key() {
                FocusDirection::Previous
            } else {
                FocusDirection::Next
            };
            Some(self.ui_dispatch.focus_in_order(frame, direction))
        } else if self.ui_dispatch.focused() != Some(COMPOSER) {
            match &event.logical_key {
                Key::Named(NamedKey::ArrowLeft) => Some(self.ui_dispatch.focus_within_group(
                    frame,
                    FocusDirection::Previous,
                    NavigationAxis::Horizontal,
                )),
                Key::Named(NamedKey::ArrowRight) => Some(self.ui_dispatch.focus_within_group(
                    frame,
                    FocusDirection::Next,
                    NavigationAxis::Horizontal,
                )),
                Key::Named(NamedKey::ArrowUp) => Some(self.ui_dispatch.focus_within_group(
                    frame,
                    FocusDirection::Previous,
                    NavigationAxis::Vertical,
                )),
                Key::Named(NamedKey::ArrowDown) => Some(self.ui_dispatch.focus_within_group(
                    frame,
                    FocusDirection::Next,
                    NavigationAxis::Vertical,
                )),
                Key::Named(NamedKey::Enter) => Some(self.ui_dispatch.activate_focused(frame)),
                Key::Character(text) if text == " " => {
                    Some(self.ui_dispatch.activate_focused(frame))
                }
                Key::Named(NamedKey::Escape) => {
                    Some(self.ui_dispatch.focus_element(frame, COMPOSER))
                }
                _ => Some(Default::default()),
            }
        } else {
            None
        };
        let Some(outcome) = outcome else {
            return false;
        };
        self.apply_dispatch_outcome(outcome);
        true
    }

    fn direct_terminal_keyboard_input(&mut self, event: &KeyEvent) {
        let copy_shortcut = is_clipboard_shortcut(&event.logical_key, "c", self.modifiers, true);
        if copy_shortcut && self.copy_terminal_selection() {
            return;
        }
        let paste_shortcut = is_clipboard_shortcut(&event.logical_key, "v", self.modifiers, true);
        if paste_shortcut && self.paste_into_terminal() {
            return;
        }
        let Some(terminal) = self.terminal.as_ref() else {
            return;
        };
        let input = encode_key_event(terminal.core(), event, self.modifiers);
        self.send_terminal_input(input, "could not send terminal input");
    }

    fn submit_composer_command(&mut self) {
        let Some(command) = self.terminal_composer.command().map(ToOwned::to_owned) else {
            return;
        };
        let Some(terminal) = self.terminal.as_mut() else {
            return;
        };
        if let Err(error) = terminal.submit_command(&command) {
            eprintln!("could not submit terminal command: {error}");
            return;
        }
        self.terminal_composer.clear_after_submit();
        self.terminal_scroll.reset();
        self.terminal_selection.clear();
        self.composer_changed();
    }

    fn copy_composer_selection(&mut self) -> bool {
        let Some(text) = self.terminal_composer.input().selected_text() else {
            return false;
        };
        if let Err(error) = write_clipboard_text(text.to_string()) {
            eprintln!("could not copy command text: {error}");
        }
        true
    }

    fn paste_into_composer(&mut self) {
        let text = match read_clipboard_text() {
            Ok(text) => text,
            Err(error) => {
                eprintln!("could not paste clipboard text: {error}");
                return;
            }
        };
        self.terminal_composer.apply(TextInputCommand::Insert(text));
        self.composer_changed();
    }

    fn paste_into_terminal(&mut self) -> bool {
        let Some(terminal) = self.terminal.as_ref() else {
            return false;
        };
        let text = match read_clipboard_text() {
            Ok(text) => text,
            Err(error) => {
                eprintln!("could not paste clipboard text: {error}");
                return true;
            }
        };
        let input = terminal.core().encode_paste(&text);
        self.send_terminal_input(input, "could not send terminal paste");
        true
    }

    fn composer_changed(&mut self) {
        self.caret_blink.activity(Instant::now());
        self.rebuild_presentation();
        self.update_ime_cursor_area();
        self.request_redraw();
    }

    pub(super) fn send_terminal_input(&mut self, input: Vec<u8>, error_context: &str) {
        if input.is_empty() {
            return;
        }
        if let Some(terminal) = self.terminal.as_mut()
            && let Err(error) = terminal.send_input(input)
        {
            eprintln!("{error_context}: {error}");
            return;
        }
        self.terminal_scroll.reset();
        self.terminal_selection.clear();
        self.rebuild_presentation();
        self.request_redraw();
    }
}

fn encode_key_event(
    terminal: &TerminalCore,
    event: &KeyEvent,
    modifiers: ModifiersState,
) -> Vec<u8> {
    if modifiers.super_key() {
        return Vec::new();
    }
    let Some(key) = terminal_key(event) else {
        return Vec::new();
    };
    terminal.encode_key(key, terminal_modifiers(modifiers))
}

fn terminal_modifiers(modifiers: ModifiersState) -> KeyModifiers {
    let mut terminal = KeyModifiers::NONE;
    if modifiers.shift_key() {
        terminal = terminal.with_shift();
    }
    if modifiers.alt_key() {
        terminal = terminal.with_alt();
    }
    if modifiers.control_key() {
        terminal = terminal.with_control();
    }
    terminal
}

fn is_clipboard_shortcut(
    key: &Key,
    character: &str,
    modifiers: ModifiersState,
    direct_terminal_input: bool,
) -> bool {
    if !matches!(key, Key::Character(text) if text.eq_ignore_ascii_case(character)) {
        return false;
    }
    modifiers.super_key()
        || (modifiers.control_key() && (!direct_terminal_input || modifiers.shift_key()))
}

fn terminal_key(event: &KeyEvent) -> Option<TerminalKey<'_>> {
    match &event.logical_key {
        Key::Character(text) => Some(TerminalKey::Text(
            event.text.as_deref().unwrap_or(text.as_str()),
        )),
        Key::Named(NamedKey::Enter) => Some(TerminalKey::Enter),
        Key::Named(NamedKey::Tab) => Some(TerminalKey::Tab),
        Key::Named(NamedKey::Backspace) => Some(TerminalKey::Backspace),
        Key::Named(NamedKey::Escape) => Some(TerminalKey::Escape),
        Key::Named(NamedKey::ArrowUp) => Some(TerminalKey::ArrowUp),
        Key::Named(NamedKey::ArrowDown) => Some(TerminalKey::ArrowDown),
        Key::Named(NamedKey::ArrowRight) => Some(TerminalKey::ArrowRight),
        Key::Named(NamedKey::ArrowLeft) => Some(TerminalKey::ArrowLeft),
        Key::Named(NamedKey::Home) => Some(TerminalKey::Home),
        Key::Named(NamedKey::End) => Some(TerminalKey::End),
        Key::Named(NamedKey::Insert) => Some(TerminalKey::Insert),
        Key::Named(NamedKey::Delete) => Some(TerminalKey::Delete),
        Key::Named(NamedKey::PageUp) => Some(TerminalKey::PageUp),
        Key::Named(NamedKey::PageDown) => Some(TerminalKey::PageDown),
        Key::Named(NamedKey::F1) => Some(TerminalKey::F1),
        Key::Named(NamedKey::F2) => Some(TerminalKey::F2),
        Key::Named(NamedKey::F3) => Some(TerminalKey::F3),
        Key::Named(NamedKey::F4) => Some(TerminalKey::F4),
        Key::Named(NamedKey::F5) => Some(TerminalKey::F5),
        Key::Named(NamedKey::F6) => Some(TerminalKey::F6),
        Key::Named(NamedKey::F7) => Some(TerminalKey::F7),
        Key::Named(NamedKey::F8) => Some(TerminalKey::F8),
        Key::Named(NamedKey::F9) => Some(TerminalKey::F9),
        Key::Named(NamedKey::F10) => Some(TerminalKey::F10),
        Key::Named(NamedKey::F11) => Some(TerminalKey::F11),
        Key::Named(NamedKey::F12) => Some(TerminalKey::F12),
        _ => None,
    }
}

#[cfg(test)]
#[path = "terminal_input_tests.rs"]
mod tests;
