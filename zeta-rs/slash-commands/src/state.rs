use std::ops::Range;

use crate::{
    SlashCommandCatalog, SlashCommandCompletion, SlashCommandDefinition, SlashCommandInput,
    SlashCommandInvocation,
};

/// Immutable render projection for one visible Slash Commands list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SlashCommandsView<'a> {
    pub commands: &'a [SlashCommandDefinition],
    pub selected: usize,
}

/// Headless query, selection, dismissal, completion, and submission state.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SlashCommandsState {
    catalog: SlashCommandCatalog,
    input: Option<String>,
    cursor: usize,
    query: Option<String>,
    commands: Vec<SlashCommandDefinition>,
    selected: usize,
    dismissed_input: Option<String>,
}

impl SlashCommandsState {
    pub fn new(catalog: SlashCommandCatalog) -> Self {
        Self {
            catalog,
            ..Self::default()
        }
    }

    pub fn catalog(&self) -> &SlashCommandCatalog {
        &self.catalog
    }

    pub fn set_catalog(&mut self, catalog: SlashCommandCatalog) {
        self.catalog = catalog;
        self.refresh();
    }

    pub fn sync_input(&mut self, input: &str, cursor: usize) {
        self.input = Some(input.to_owned());
        self.cursor = cursor;
        self.refresh();
    }

    pub fn view(&self) -> Option<SlashCommandsView<'_>> {
        (self.query.is_some() && self.dismissed_input.as_deref() != self.input.as_deref())
            .then_some(SlashCommandsView {
                commands: &self.commands,
                selected: self.selected,
            })
    }

    pub fn selected_command(&self) -> Option<&SlashCommandDefinition> {
        self.view()
            .and_then(|view| view.commands.get(view.selected))
    }

    pub fn command_at(&self, index: usize) -> Option<&SlashCommandDefinition> {
        self.view().and_then(|view| view.commands.get(index))
    }

    pub fn select_previous(&mut self) {
        if self.commands.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.commands.len() - 1
        } else {
            self.selected - 1
        };
    }

    pub fn select_next(&mut self) {
        if !self.commands.is_empty() {
            self.selected = (self.selected + 1) % self.commands.len();
        }
    }

    pub fn select(&mut self, index: usize) -> bool {
        if index >= self.commands.len() || self.view().is_none() {
            return false;
        }
        self.selected = index;
        true
    }

    pub fn completion(&self, command: &SlashCommandDefinition) -> Option<SlashCommandCompletion> {
        SlashCommandInput::at_cursor(self.input.as_deref()?, self.cursor, &self.catalog)
            .completion(command)
    }

    pub fn selected_completion(&self) -> Option<SlashCommandCompletion> {
        self.completion(self.selected_command()?)
    }

    pub fn invocation(&self, input: &str) -> Option<SlashCommandInvocation> {
        SlashCommandInput::for_submission(input, &self.catalog).invocation()
    }

    pub fn command_element_range(&self, input: &str, cursor: usize) -> Option<Range<usize>> {
        SlashCommandInput::at_cursor(input, cursor, &self.catalog).command_element_range()
    }

    pub fn dismiss(&mut self) {
        self.dismissed_input.clone_from(&self.input);
    }

    pub fn clear(&mut self) {
        self.input = None;
        self.cursor = 0;
        self.query = None;
        self.commands.clear();
        self.selected = 0;
        self.dismissed_input = None;
    }

    fn refresh(&mut self) {
        let Some(input) = self.input.as_deref() else {
            self.query = None;
            self.commands.clear();
            return;
        };
        let slash_input = SlashCommandInput::at_cursor(input, self.cursor, &self.catalog);
        let Some(query) = slash_input.query() else {
            self.query = None;
            self.commands.clear();
            self.selected = 0;
            self.dismissed_input = None;
            return;
        };
        let query_changed = self.query.as_deref() != Some(query.text);
        self.query = Some(query.text.to_owned());
        self.commands = slash_input
            .matching_commands()
            .expect("a Slash Command query has matches");
        if query_changed {
            self.selected = 0;
        } else {
            self.selected = self.selected.min(self.commands.len().saturating_sub(1));
        }
        if self.dismissed_input.as_deref() != Some(input) {
            self.dismissed_input = None;
        }
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
