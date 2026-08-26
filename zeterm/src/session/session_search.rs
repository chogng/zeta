//! Host-owned search editor and matching policy for the Sessions sidebar.

use zeta_ui::{TextInput, TextInputCommand, TextInputCompositionEvent};

#[derive(Default)]
pub(crate) struct SessionSearch {
    input: TextInput,
}

impl SessionSearch {
    pub(crate) const fn input(&self) -> &TextInput {
        &self.input
    }

    pub(crate) fn apply(&mut self, command: TextInputCommand) {
        self.input.apply(command);
    }

    pub(crate) fn apply_composition(&mut self, event: TextInputCompositionEvent) {
        self.input.apply_composition(event);
    }

    pub(crate) fn cancel_composition(&mut self) {
        self.input.cancel_composition();
    }

    pub(crate) fn clear(&mut self) {
        self.input.take_text();
    }

    pub(crate) fn selected_text(&self) -> Option<&str> {
        self.input.selected_text()
    }

    pub(crate) fn matches_session_name(&self, name: &str) -> bool {
        let query = self.input.text().trim();
        query.is_empty() || name.to_lowercase().contains(&query.to_lowercase())
    }
}

#[cfg(test)]
#[path = "session_search_tests.rs"]
mod tests;
