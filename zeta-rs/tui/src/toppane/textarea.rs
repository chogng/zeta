use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TextAreaOutcome {
    Consumed,
    Unhandled,
}

/// Owns the editable text buffer, cursor, and editor keymap boundary.
///
/// Vim modes, motions, and operators belong in this component when that capability is added.
/// Chat-level submission and slash semantics stay in the parent composer.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct TextArea {
    text: String,
    cursor: usize,
}

impl TextArea {
    pub(super) fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
        }
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> TextAreaOutcome {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return TextAreaOutcome::Unhandled;
        }

        match key.code {
            KeyCode::Backspace => {
                self.backspace();
                TextAreaOutcome::Consumed
            }
            KeyCode::Delete => {
                self.delete();
                TextAreaOutcome::Consumed
            }
            KeyCode::Left => {
                self.move_left();
                TextAreaOutcome::Consumed
            }
            KeyCode::Right => {
                self.move_right();
                TextAreaOutcome::Consumed
            }
            KeyCode::Home => {
                self.cursor = 0;
                TextAreaOutcome::Consumed
            }
            KeyCode::End => {
                self.cursor = self.text.len();
                TextAreaOutcome::Consumed
            }
            KeyCode::Char(character) => {
                self.text.insert(self.cursor, character);
                self.cursor += character.len_utf8();
                TextAreaOutcome::Consumed
            }
            _ => TextAreaOutcome::Unhandled,
        }
    }

    pub(super) fn insert_text(&mut self, text: &str) {
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
    }

    pub(super) fn text(&self) -> &str {
        &self.text
    }

    pub(super) fn cursor_display_width(&self) -> usize {
        self.text[..self.cursor].width()
    }

    pub(super) fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
    }

    fn backspace(&mut self) {
        let Some(previous) = self.previous_boundary() else {
            return;
        };
        self.text.replace_range(previous..self.cursor, "");
        self.cursor = previous;
    }

    fn delete(&mut self) {
        let Some(next) = self.next_boundary() else {
            return;
        };
        self.text.replace_range(self.cursor..next, "");
    }

    fn move_left(&mut self) {
        if let Some(previous) = self.previous_boundary() {
            self.cursor = previous;
        }
    }

    fn move_right(&mut self) {
        if let Some(next) = self.next_boundary() {
            self.cursor = next;
        }
    }

    fn previous_boundary(&self) -> Option<usize> {
        self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
    }

    fn next_boundary(&self) -> Option<usize> {
        self.text[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| self.cursor + offset)
            .or_else(|| (self.cursor < self.text.len()).then_some(self.text.len()))
    }
}

#[cfg(test)]
#[path = "textarea_tests.rs"]
mod tests;
