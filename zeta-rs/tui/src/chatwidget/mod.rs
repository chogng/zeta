use crate::toppane::TopPane;
use crate::toppane::TopPaneOutcome;
use crossterm::event::KeyEvent;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MessageRole {
    User,
    Agent,
    Notice,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Message {
    pub(crate) role: MessageRole,
    pub(crate) text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ChatWidgetOutcome {
    Consumed,
    Submit(String),
    Unhandled,
}

/// Coordinates conversation presentation and delegates interaction to the top pane.
///
/// The widget owns transcript state, but callers remain responsible for global lifecycle,
/// product status, and executing submitted actions. Editing and composer behavior remain owned by
/// the sibling `toppane` module.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct ChatWidget {
    messages: Vec<Message>,
    top_pane: TopPane,
}

impl ChatWidget {
    pub(crate) fn new() -> Self {
        Self {
            messages: Vec::new(),
            top_pane: TopPane::new(),
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> ChatWidgetOutcome {
        match self.top_pane.handle_key(key) {
            TopPaneOutcome::Consumed => ChatWidgetOutcome::Consumed,
            TopPaneOutcome::Submit(prompt) => {
                self.messages.push(Message {
                    role: MessageRole::User,
                    text: prompt.clone(),
                });
                ChatWidgetOutcome::Submit(prompt)
            }
            TopPaneOutcome::Unhandled => ChatWidgetOutcome::Unhandled,
        }
    }

    pub(crate) fn insert_text(&mut self, text: &str) {
        self.top_pane.insert_text(text);
    }

    pub(crate) fn draft(&self) -> &str {
        self.top_pane.text()
    }

    pub(crate) fn draft_cursor_width(&self) -> usize {
        self.top_pane.cursor_display_width()
    }

    pub(crate) fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub(crate) fn push_message(&mut self, role: MessageRole, text: String) {
        self.messages.push(Message { role, text });
    }
}

#[cfg(test)]
#[path = "chatwidget_tests.rs"]
mod tests;
