use crate::chatwidget::ChatWidget;
use crate::chatwidget::ChatWidgetOutcome;
pub(crate) use crate::chatwidget::Message;
pub(crate) use crate::chatwidget::MessageRole;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Quit,
    Interrupt,
    Submit(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Status {
    Ready,
    Working,
    WaitingForApproval,
    WaitingForUserInput,
    WaitingForCapability,
    Cancelling,
    Error,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct App {
    chat_widget: ChatWidget,
    status: Status,
}

impl App {
    pub(crate) fn new() -> Self {
        Self {
            chat_widget: ChatWidget::new(),
            status: Status::Ready,
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.accepts_input() {
            return self.handle_global_key(key);
        }

        match self.chat_widget.handle_key(key) {
            ChatWidgetOutcome::Submit(prompt) => {
                self.status = Status::Working;
                Some(Action::Submit(prompt))
            }
            ChatWidgetOutcome::Consumed => None,
            ChatWidgetOutcome::Unhandled => self.handle_global_key(key),
        }
    }

    pub(crate) fn insert_text(&mut self, text: &str) {
        if self.accepts_input() {
            self.chat_widget.insert_text(text);
        }
    }

    pub(crate) fn input(&self) -> &str {
        self.chat_widget.draft()
    }

    pub(crate) fn input_cursor_width(&self) -> usize {
        self.chat_widget.draft_cursor_width()
    }

    pub(crate) fn messages(&self) -> &[Message] {
        self.chat_widget.messages()
    }

    pub(crate) fn status(&self) -> &Status {
        &self.status
    }

    pub(crate) fn accepts_input(&self) -> bool {
        matches!(&self.status, Status::Ready | Status::Error)
    }

    pub(crate) fn record_response(&mut self, response: String) {
        self.chat_widget.push_message(MessageRole::Agent, response);
        self.status = Status::Ready;
    }

    pub(crate) fn record_interrupted(&mut self) {
        self.chat_widget
            .push_message(MessageRole::Notice, "turn interrupted".into());
        self.status = Status::Ready;
    }

    pub(crate) fn record_interrupt_failure(&mut self, error: String) {
        self.chat_widget.push_message(
            MessageRole::Error,
            format!("could not interrupt turn: {error}"),
        );
        self.status = Status::Working;
    }

    pub(crate) fn record_working(&mut self) {
        self.status = Status::Working;
    }

    pub(crate) fn record_cancelling(&mut self) {
        self.status = Status::Cancelling;
    }

    pub(crate) fn wait_for_approval(&mut self) {
        self.status = Status::WaitingForApproval;
    }

    pub(crate) fn wait_for_user_input(&mut self) {
        self.status = Status::WaitingForUserInput;
    }

    pub(crate) fn wait_for_capability(&mut self) {
        self.status = Status::WaitingForCapability;
    }

    pub(crate) fn record_error(&mut self, error: String) {
        self.chat_widget.push_message(MessageRole::Error, error);
        self.status = Status::Error;
    }

    fn handle_global_key(&mut self, key: KeyEvent) -> Option<Action> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') => self.quit_or_interrupt(),
                KeyCode::Char('d') if self.input().is_empty() => self.quit_or_interrupt(),
                _ => None,
            };
        }
        if key.code == KeyCode::Esc {
            return self.quit_or_interrupt();
        }
        None
    }

    fn quit_or_interrupt(&mut self) -> Option<Action> {
        match &self.status {
            Status::Working
            | Status::WaitingForApproval
            | Status::WaitingForUserInput
            | Status::WaitingForCapability => {
                self.status = Status::Cancelling;
                Some(Action::Interrupt)
            }
            Status::Cancelling => None,
            Status::Ready | Status::Error => Some(Action::Quit),
        }
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
