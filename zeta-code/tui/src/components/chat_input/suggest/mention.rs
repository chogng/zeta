//! Workspace-file mention query and composer completion state.

mod input;
mod popup;

use super::editor::TextArea;
use popup::MentionPopup;
use zeta_file_search::PathSearchSnapshot;

pub(crate) use popup::MentionPopupView;

/// Owns the active `@file` token and completion popup state.
#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct Mentions {
    popup: MentionPopup,
}

impl Mentions {
    pub(super) fn sync_textarea(&mut self, textarea: &TextArea) {
        self.popup
            .sync(input::active_mention(textarea.text(), textarea.cursor()));
    }

    pub(super) fn query(&self) -> Option<&str> {
        self.popup.query()
    }

    pub(super) fn apply_search_snapshot(&mut self, snapshot: PathSearchSnapshot) {
        self.popup.apply_search_snapshot(snapshot);
    }

    pub(super) fn view(&self) -> Option<MentionPopupView<'_>> {
        self.popup.view()
    }

    pub(super) fn select_previous(&mut self) {
        self.popup.select_previous();
    }

    pub(super) fn select_next(&mut self) {
        self.popup.select_next();
    }

    pub(super) fn select(&mut self, index: usize) -> bool {
        self.popup.select(index)
    }

    pub(super) fn dismiss(&mut self) {
        self.popup.dismiss();
    }

    pub(super) fn complete_selected(&mut self, textarea: &mut TextArea) -> bool {
        let Some((range, path)) = self.popup.selected_completion() else {
            return false;
        };
        insert_path(textarea, range, &path);
        self.popup.clear();
        true
    }

    pub(super) fn complete_at(&mut self, textarea: &mut TextArea, index: usize) -> bool {
        let Some((range, path)) = self.popup.completion_at(index) else {
            return false;
        };
        insert_path(textarea, range, &path);
        self.popup.clear();
        true
    }

    pub(super) fn clear(&mut self) {
        self.popup.clear();
    }
}

fn insert_path(textarea: &mut TextArea, range: std::ops::Range<usize>, path: &str) {
    textarea.replace_range(range, "");
    textarea.insert_element(path);
    if !textarea.text()[textarea.cursor()..]
        .chars()
        .next()
        .is_some_and(char::is_whitespace)
    {
        textarea.insert_text(" ");
    }
}

#[cfg(test)]
#[path = "mentions/mentions_tests.rs"]
mod tests;
