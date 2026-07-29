//! Slash-command discovery popup state.

use super::slash_commands::SlashCommandItem;
use super::slash_commands::SlashCommandRegistry;
use super::slash_input::SlashInput;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SlashPopupView<'a> {
    pub(crate) commands: &'a [SlashCommandItem],
    pub(crate) selected: usize,
}

/// Owns slash-command discovery state associated with the current composer input.
///
/// Matches are derived from the current validated command registry whenever the cursor query or
/// registry snapshot changes. Rendering consumes [`SlashPopupView`] and does not mutate selection
/// or dismissal state.
#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct SlashCommandPopup {
    query: Option<String>,
    commands: Vec<SlashCommandItem>,
    selected: usize,
    dismissed: bool,
}

impl SlashCommandPopup {
    pub(super) fn sync_input(
        &mut self,
        input: &str,
        cursor: usize,
        registry: &SlashCommandRegistry,
    ) {
        let slash_input = SlashInput::at_cursor(input, cursor, registry);
        let Some(query) = slash_input.popup_query() else {
            self.clear();
            return;
        };
        let commands = slash_input
            .matching_commands()
            .expect("a slash query must have command matches");
        if self.query.as_deref() == Some(query.text) && self.commands == commands {
            return;
        }

        self.query = Some(query.text.to_owned());
        self.commands = commands;
        self.selected = 0;
        self.dismissed = false;
    }

    pub(super) fn view(&self) -> Option<SlashPopupView<'_>> {
        (!self.dismissed && self.query.is_some()).then_some(SlashPopupView {
            commands: &self.commands,
            selected: self.selected,
        })
    }

    pub(super) fn selected_command(&self) -> Option<SlashCommandItem> {
        self.view()
            .and_then(|view| view.commands.get(view.selected).cloned())
    }

    pub(super) fn command_at(&self, index: usize) -> Option<SlashCommandItem> {
        self.view()
            .and_then(|view| view.commands.get(index).cloned())
    }

    pub(super) fn select_previous(&mut self) {
        if self.commands.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.commands.len() - 1
        } else {
            self.selected - 1
        };
    }

    pub(super) fn select_next(&mut self) {
        if !self.commands.is_empty() {
            self.selected = (self.selected + 1) % self.commands.len();
        }
    }

    pub(super) fn dismiss(&mut self) {
        self.dismissed = true;
    }

    pub(super) fn clear(&mut self) {
        self.query = None;
        self.commands.clear();
        self.selected = 0;
        self.dismissed = false;
    }
}

#[cfg(test)]
#[path = "slash_command_popup_tests.rs"]
mod tests;
