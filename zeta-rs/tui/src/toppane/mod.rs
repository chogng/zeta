mod chat_composer;
mod textarea;

use chat_composer::ChatComposer;
use chat_composer::ComposerOutcome;
use crossterm::event::KeyEvent;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TopPaneOutcome {
    Consumed,
    Submit(String),
    Unhandled,
}

/// Owns the active interaction surface used by the sibling chat widget.
///
/// The top pane routes input to its composer today. Future interaction modes can be added here
/// without moving slash semantics into `ChatWidget` or editor behavior into the application
/// coordinator.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct TopPane {
    composer: ChatComposer,
}

impl TopPane {
    pub(crate) fn new() -> Self {
        Self {
            composer: ChatComposer::new(),
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> TopPaneOutcome {
        match self.composer.handle_key(key) {
            ComposerOutcome::Consumed => TopPaneOutcome::Consumed,
            ComposerOutcome::Submit(prompt) => TopPaneOutcome::Submit(prompt),
            ComposerOutcome::Unhandled => TopPaneOutcome::Unhandled,
        }
    }

    pub(crate) fn insert_text(&mut self, text: &str) {
        self.composer.insert_text(text);
    }

    pub(crate) fn text(&self) -> &str {
        self.composer.text()
    }

    pub(crate) fn cursor_display_width(&self) -> usize {
        self.composer.cursor_display_width()
    }
}

#[cfg(test)]
#[path = "toppane_tests.rs"]
mod tests;
