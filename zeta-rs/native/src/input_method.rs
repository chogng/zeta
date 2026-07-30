use std::time::Instant;

use zeta_terminal::{KeyModifiers, ScreenBuffer, TerminalCore, TerminalKey};
use zeta_ui::{TextInputCompositionCursor, TextInputCompositionEvent};
use zeta_winit::{Ime, ImeCursorArea};

use crate::NativeApp;
use crate::shell_interaction::{COMPOSER, SESSION_SEARCH_INPUT};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InputMethodTarget {
    Disabled,
    Composer,
    SessionSearch,
    TerminalGrid,
}

#[derive(Clone, Copy, Debug)]
struct InputMethodContext {
    window_active: bool,
    screen: ScreenBuffer,
    composer_focused: bool,
    session_search_focused: bool,
}

impl InputMethodTarget {
    fn for_context(context: InputMethodContext) -> Self {
        if !context.window_active {
            return Self::Disabled;
        }
        if context.session_search_focused {
            return Self::SessionSearch;
        }
        match context.screen {
            ScreenBuffer::Primary if context.composer_focused => Self::Composer,
            ScreenBuffer::Primary => Self::Disabled,
            ScreenBuffer::Alternate => Self::TerminalGrid,
        }
    }

    const fn is_enabled(self) -> bool {
        !matches!(self, Self::Disabled)
    }
}

impl NativeApp {
    pub(super) fn ime_input(&mut self, event: Ime) {
        let target = self.input_method_target();
        if matches!(event, Ime::Enabled) {
            if target.is_enabled() {
                self.update_ime_cursor_area();
            }
            return;
        }
        match target {
            InputMethodTarget::Disabled => {}
            InputMethodTarget::Composer => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.caret_blink.activity(Instant::now());
                self.terminal_composer.apply_composition(composition);
                self.rebuild_presentation();
                self.request_redraw();
            }
            InputMethodTarget::SessionSearch => {
                let Some(composition) = text_input_composition_event(event) else {
                    return;
                };
                self.caret_blink.activity(Instant::now());
                self.session_search.apply_composition(composition);
                self.rebuild_presentation();
                self.request_redraw();
            }
            InputMethodTarget::TerminalGrid => {
                let Some(terminal) = self.terminal.as_ref() else {
                    return;
                };
                let input = encode_terminal_ime_event(terminal.core(), &event);
                self.send_terminal_input(input, "could not send terminal IME commit");
            }
        }
    }

    pub(super) fn update_ime_cursor_area(&self) {
        if !self.input_method_target().is_enabled() {
            return;
        }
        let Some(bounds) = self
            .presentation
            .as_ref()
            .and_then(|presentation| presentation.ime_cursor_area)
        else {
            return;
        };
        if let Some(window) = self.window.as_ref() {
            window.set_ime_cursor_area(ImeCursorArea::new(
                bounds.origin.x as f64,
                bounds.origin.y as f64,
                bounds.size.width as f64,
                bounds.size.height as f64,
            ));
        }
    }

    pub(super) fn sync_input_focus(&mut self) {
        let target = self.input_method_target();
        if matches!(
            target,
            InputMethodTarget::Composer | InputMethodTarget::SessionSearch
        ) {
            self.caret_blink.focus(Instant::now());
        } else {
            self.caret_blink.blur();
        }
        if target != InputMethodTarget::Composer {
            self.terminal_composer.cancel_composition();
        }
        if target != InputMethodTarget::SessionSearch {
            self.session_search.cancel_composition();
        }
        if let Some(window) = self.window.as_ref() {
            if target.is_enabled() {
                window.enable_ime();
            } else {
                window.disable_ime();
            }
        }
    }

    fn input_method_target(&self) -> InputMethodTarget {
        InputMethodTarget::for_context(InputMethodContext {
            window_active: self.ui_dispatch.window_active(),
            screen: self.active_screen(),
            composer_focused: self.ui_dispatch.is_focused(COMPOSER),
            session_search_focused: self.ui_dispatch.is_focused(SESSION_SEARCH_INPUT),
        })
    }
}

fn text_input_composition_event(event: Ime) -> Option<TextInputCompositionEvent> {
    match event {
        Ime::Preedit(text, Some((start, end))) => Some(TextInputCompositionEvent::Preedit {
            text,
            cursor: TextInputCompositionCursor::Visible(start..end),
        }),
        Ime::Preedit(text, None) => Some(TextInputCompositionEvent::Preedit {
            text,
            cursor: TextInputCompositionCursor::Hidden,
        }),
        Ime::Commit(text) => Some(TextInputCompositionEvent::Commit(text)),
        Ime::Disabled => Some(TextInputCompositionEvent::Cancel),
        Ime::Enabled => None,
    }
}

fn encode_terminal_ime_event(terminal: &TerminalCore, event: &Ime) -> Vec<u8> {
    match event {
        Ime::Commit(text) => terminal.encode_key(TerminalKey::Text(text), KeyModifiers::NONE),
        Ime::Enabled | Ime::Preedit(_, _) | Ime::Disabled => Vec::new(),
    }
}

#[cfg(test)]
#[path = "input_method_tests.rs"]
mod tests;
