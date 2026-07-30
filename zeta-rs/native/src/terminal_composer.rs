use zeta_ui::{TextInput, TextInputCommand, TextInputCompositionEvent};

/// Host-owned command editor shown at the bottom of a primary terminal session.
#[derive(Default)]
pub(crate) struct TerminalComposer {
    input: TextInput,
}

impl TerminalComposer {
    pub(crate) const fn input(&self) -> &TextInput {
        &self.input
    }

    pub(crate) fn command(&self) -> Option<&str> {
        (!self.input.text().trim().is_empty()).then(|| self.input.text())
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

    pub(crate) fn clear_after_submit(&mut self) {
        self.input.take_text();
    }
}

#[cfg(test)]
#[path = "terminal_composer_tests.rs"]
mod tests;
