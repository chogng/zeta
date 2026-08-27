use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use std::fmt;

pub(crate) const SEARCH_BOX_HEIGHT: u16 = 3;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SearchBoxModel {
    placeholder: String,
    initially_active: bool,
    masked: bool,
}

impl SearchBoxModel {
    pub(crate) fn new(placeholder: impl Into<String>) -> Self {
        Self {
            placeholder: placeholder.into(),
            initially_active: false,
            masked: false,
        }
    }

    pub(crate) fn initially_active(mut self) -> Self {
        self.initially_active = true;
        self
    }

    pub(crate) fn masked(mut self) -> Self {
        self.masked = true;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SearchBoxInputOutcome {
    Consumed,
    Ignored,
    QueryChanged,
}

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct SearchBoxState {
    model: SearchBoxModel,
    query: String,
    input_active: bool,
}

impl SearchBoxState {
    pub(crate) fn new(model: SearchBoxModel) -> Self {
        Self {
            input_active: model.initially_active,
            model,
            query: String::new(),
        }
    }

    pub(crate) fn replace_model(&mut self, model: SearchBoxModel) {
        self.model = model;
    }

    pub(crate) fn placeholder(&self) -> &str {
        &self.model.placeholder
    }

    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    pub(crate) fn input_active(&self) -> bool {
        self.input_active
    }

    pub(crate) fn masked(&self) -> bool {
        self.model.masked
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> SearchBoxInputOutcome {
        if key.code == KeyCode::Esc && key.kind == KeyEventKind::Repeat {
            return SearchBoxInputOutcome::Consumed;
        }
        if !self.input_active {
            if key.code == KeyCode::Char(' ') {
                self.input_active = true;
                return SearchBoxInputOutcome::Consumed;
            }
            return SearchBoxInputOutcome::Ignored;
        }

        match key.code {
            KeyCode::Esc => {
                self.query.clear();
                self.input_active = false;
                SearchBoxInputOutcome::QueryChanged
            }
            KeyCode::Backspace => {
                self.query.pop();
                SearchBoxInputOutcome::QueryChanged
            }
            KeyCode::Char(character) if !character.is_ascii_control() => {
                self.query.push(character);
                SearchBoxInputOutcome::QueryChanged
            }
            _ => SearchBoxInputOutcome::Ignored,
        }
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) -> SearchBoxInputOutcome {
        if !self.input_active {
            return SearchBoxInputOutcome::Ignored;
        }
        let normalized = pasted.split_whitespace().collect::<Vec<_>>().join(" ");
        self.query.push_str(&normalized);
        SearchBoxInputOutcome::QueryChanged
    }
}

impl fmt::Debug for SearchBoxState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SearchBoxState")
            .field("model", &self.model)
            .field(
                "query",
                &if self.masked() {
                    "[REDACTED]"
                } else {
                    self.query.as_str()
                },
            )
            .field("input_active", &self.input_active)
            .finish()
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
