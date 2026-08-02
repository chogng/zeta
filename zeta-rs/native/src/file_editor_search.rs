use zeta_editor::CodeEditorSearchQuery;
use zeta_ui::{TextInput, TextInputCommand, TextInputCompositionEvent};

/// Which Native find widget fields are visible above the shared CodeEditor.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum FileEditorSearchMode {
    #[default]
    Hidden,
    Find,
    Replace,
}

/// Ephemeral Native input state for the file editor find/replace presentation.
#[derive(Default)]
pub(crate) struct FileEditorSearchState {
    mode: FileEditorSearchMode,
    query: TextInput,
    replacement: TextInput,
}

impl FileEditorSearchState {
    pub(crate) const fn mode(&self) -> FileEditorSearchMode {
        self.mode
    }

    pub(crate) const fn query_input(&self) -> &TextInput {
        &self.query
    }

    pub(crate) const fn replacement_input(&self) -> &TextInput {
        &self.replacement
    }

    pub(crate) fn query(&self) -> CodeEditorSearchQuery {
        CodeEditorSearchQuery::new(self.query.text())
    }

    pub(crate) fn replacement(&self) -> &str {
        self.replacement.text()
    }

    pub(crate) fn selected_query_text(&self) -> Option<&str> {
        self.query.selected_text()
    }

    pub(crate) fn selected_replacement_text(&self) -> Option<&str> {
        self.replacement.selected_text()
    }

    pub(crate) fn show_find(&mut self) {
        self.mode = FileEditorSearchMode::Find;
        self.query.apply(TextInputCommand::SelectAll);
    }

    pub(crate) fn show_replace(&mut self) {
        self.mode = FileEditorSearchMode::Replace;
        self.query.apply(TextInputCommand::SelectAll);
    }

    pub(crate) fn hide(&mut self) {
        self.mode = FileEditorSearchMode::Hidden;
        self.cancel_composition();
    }

    pub(crate) fn apply_query(&mut self, command: TextInputCommand) {
        self.query.apply(command);
    }

    pub(crate) fn apply_replacement(&mut self, command: TextInputCommand) {
        self.replacement.apply(command);
    }

    pub(crate) fn apply_query_composition(&mut self, event: TextInputCompositionEvent) {
        self.query.apply_composition(event);
    }

    pub(crate) fn apply_replacement_composition(&mut self, event: TextInputCompositionEvent) {
        self.replacement.apply_composition(event);
    }

    pub(crate) fn cancel_composition(&mut self) {
        self.query.cancel_composition();
        self.replacement.cancel_composition();
    }
}

#[cfg(test)]
#[path = "file_editor_search_tests.rs"]
mod tests;
