//! Workspace-file mention query and chat_input completion state.

mod input;
mod popup;

use super::super::editor::TextArea;
use popup::MentionPopup;
use zeta_file_search::PathSearchSnapshot;

pub(crate) use popup::MentionMatchKind;
pub(crate) use popup::MentionPluginItem;
pub(crate) use popup::MentionPopupView;

/// Owns the active `@file` token and completion popup state.
#[derive(Debug, Default, Eq, PartialEq)]
pub(in crate::components::chat_input) struct Mentions {
    popup: MentionPopup,
}

impl Mentions {
    pub(in crate::components::chat_input) fn sync_textarea(&mut self, textarea: &TextArea) {
        self.popup
            .sync(input::active_mention(textarea.text(), textarea.cursor()));
    }

    pub(in crate::components::chat_input) fn query(&self) -> Option<&str> {
        self.popup.query()
    }

    pub(in crate::components::chat_input) fn apply_search_snapshot(
        &mut self,
        snapshot: PathSearchSnapshot,
    ) {
        self.popup.apply_search_snapshot(snapshot);
    }

    pub(in crate::components::chat_input) fn replace_plugin_catalog(
        &mut self,
        catalog: Vec<MentionPluginItem>,
    ) {
        self.popup.replace_plugin_catalog(catalog);
    }

    pub(in crate::components::chat_input) fn view(&self) -> Option<MentionPopupView<'_>> {
        self.popup.view()
    }

    pub(in crate::components::chat_input) fn select_previous(&mut self) {
        self.popup.select_previous();
    }

    pub(in crate::components::chat_input) fn select_next(&mut self) {
        self.popup.select_next();
    }

    pub(in crate::components::chat_input) fn select(&mut self, index: usize) -> bool {
        self.popup.select(index)
    }

    pub(in crate::components::chat_input) fn dismiss(&mut self) {
        self.popup.dismiss();
    }

    pub(in crate::components::chat_input) fn complete_selected(
        &mut self,
        textarea: &mut TextArea,
    ) -> bool {
        let Some((range, path)) = self.popup.selected_completion() else {
            return false;
        };
        insert_mention(textarea, range, &path);
        self.popup.clear();
        true
    }

    pub(in crate::components::chat_input) fn complete_at(
        &mut self,
        textarea: &mut TextArea,
        index: usize,
    ) -> bool {
        let Some((range, path)) = self.popup.completion_at(index) else {
            return false;
        };
        insert_mention(textarea, range, &path);
        self.popup.clear();
        true
    }

    pub(in crate::components::chat_input) fn clear(&mut self) {
        self.popup.clear();
    }
}

fn insert_mention(textarea: &mut TextArea, range: std::ops::Range<usize>, value: &str) {
    textarea.replace_range(range, "");
    textarea.insert_element(value);
    if !textarea.text()[textarea.cursor()..]
        .chars()
        .next()
        .is_some_and(char::is_whitespace)
    {
        textarea.insert_text(" ");
    }
}

#[cfg(test)]
#[path = "mention/mention_tests.rs"]
mod tests;
