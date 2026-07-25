use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Action {
    Quit,
    Submit(String),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MessageRole {
    User,
    Agent,
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
                KeyCode::Char('c') => Some(Action::Quit),
                KeyCode::Char('d') if self.input.is_empty() => Some(Action::Quit),
                _ => None,
            };
        }
        match key.code {
            KeyCode::Esc => Some(Action::Quit),
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
        self.input.push_str(text);
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
}

#[cfg(test)]
#[path = "app_tests.rs"]
mod tests;
