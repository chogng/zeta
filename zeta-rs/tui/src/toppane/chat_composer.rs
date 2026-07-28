use super::textarea::TextArea;
use super::textarea::TextAreaOutcome;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ComposerOutcome {
    Consumed,
    Submit(String),
    Unhandled,
}

/// Owns chat-input semantics around an editing-oriented [`TextArea`].
///
/// Slash discovery, completion, and command selection belong here as they are added. The text
/// area remains responsible only for editing state and keymap behavior.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct ChatComposer {
    textarea: TextArea,
}

impl ChatComposer {
    pub(super) fn new() -> Self {
        Self {
            textarea: TextArea::new(),
        }
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> ComposerOutcome {
        if key.code == KeyCode::Enter {
            return self.submit();
        }

        match self.textarea.handle_key(key) {
            TextAreaOutcome::Consumed => ComposerOutcome::Consumed,
            TextAreaOutcome::Unhandled => ComposerOutcome::Unhandled,
        }
    }

    pub(super) fn insert_text(&mut self, text: &str) {
        self.textarea.insert_text(text);
    }

    pub(super) fn text(&self) -> &str {
        self.textarea.text()
    }

    pub(super) fn cursor_display_width(&self) -> usize {
        self.textarea.cursor_display_width()
    }

    fn submit(&mut self) -> ComposerOutcome {
        let prompt = self.textarea.text().trim().to_owned();
        if prompt.is_empty() {
            return ComposerOutcome::Consumed;
        }
        self.textarea.clear();
        ComposerOutcome::Submit(prompt)
    }
}
