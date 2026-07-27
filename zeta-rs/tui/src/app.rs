use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Quit,
    Interrupt,
    Submit(String),
}

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
pub(crate) enum Status {
    Ready,
    Working,
    WaitingForApproval,
    WaitingForUserInput,
    WaitingForCapability,
    Cancelling,
    Error(String),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct App {
    input: String,
    messages: Vec<Message>,
    status: Status,
}

impl App {
    pub(crate) fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            status: Status::Ready,
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') => self.quit_or_interrupt(),
                KeyCode::Char('d') if self.input.is_empty() => self.quit_or_interrupt(),
                _ => None,
            };
        }
        if key.code == KeyCode::Esc {
            return self.quit_or_interrupt();
        }
        if !self.accepts_input() {
            return None;
        }
        match key.code {
            KeyCode::Enter => self.submit(),
            KeyCode::Backspace => {
                self.input.pop();
                None
            }
            KeyCode::Char(character) => {
                self.input.push(character);
                None
            }
            _ => None,
        }
    }

    pub(crate) fn insert_text(&mut self, text: &str) {
        if self.accepts_input() {
            self.input.push_str(text);
        }
    }

    pub(crate) fn input(&self) -> &str {
        &self.input
    }

    pub(crate) fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub(crate) fn status(&self) -> &Status {
        &self.status
    }

    pub(crate) fn record_response(&mut self, response: String) {
        self.messages.push(Message {
            role: MessageRole::Agent,
            text: response,
        });
        self.status = Status::Ready;
    }

    pub(crate) fn record_interrupted(&mut self) {
        self.messages.push(Message {
            role: MessageRole::Notice,
            text: "turn interrupted".into(),
        });
        self.status = Status::Ready;
    }

    pub(crate) fn record_interrupt_failure(&mut self, error: String) {
        self.messages.push(Message {
            role: MessageRole::Error,
            text: format!("could not interrupt turn: {error}"),
        });
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
        self.messages.push(Message {
            role: MessageRole::Error,
            text: error.clone(),
        });
        self.status = Status::Error(error);
    }

    fn submit(&mut self) -> Option<Action> {
        let prompt = self.input.trim().to_owned();
        if prompt.is_empty() {
            return None;
        }
        self.input.clear();
        self.messages.push(Message {
            role: MessageRole::User,
            text: prompt.clone(),
        });
        self.status = Status::Working;
        Some(Action::Submit(prompt))
    }

    fn accepts_input(&self) -> bool {
        matches!(&self.status, Status::Ready | Status::Error(_))
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
            Status::Ready | Status::Error(_) => Some(Action::Quit),
        }
    }
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
