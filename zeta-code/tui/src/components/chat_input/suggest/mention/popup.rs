//! Mention popup selection and stale-result handling.

use std::ops::Range;

use super::input::ActiveMention;
use zeta_file_search::PathSearchSnapshot;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MentionMatchKind {
    File,
    Plugin,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MentionMatch {
    pub(crate) label: String,
    pub(crate) completion: String,
    pub(crate) kind: MentionMatchKind,
    pub(crate) indices: Vec<usize>,
    pub(crate) score: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MentionPluginItem {
    id: String,
}

impl MentionPluginItem {
    pub(crate) fn new(id: String) -> Self {
        Self { id }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MentionPopupView<'a> {
    pub(crate) matches: &'a [MentionMatch],
    pub(crate) selected: usize,
    pub(crate) searching: bool,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(super) struct MentionPopup {
    token_range: Option<Range<usize>>,
    query: Option<String>,
    plugin_catalog: Vec<MentionPluginItem>,
    file_matches: Vec<MentionMatch>,
    matches: Vec<MentionMatch>,
    selected: usize,
    dismissed: bool,
    searching: bool,
}

impl MentionPopup {
    pub(super) fn sync(&mut self, active: Option<ActiveMention<'_>>) {
        let Some(active) = active else {
            self.clear();
            return;
        };
        if self.token_range.as_ref() == Some(&active.range)
            && self.query.as_deref() == Some(active.query)
        {
            return;
        }

        self.token_range = Some(active.range);
        self.query = Some(active.query.to_owned());
        self.file_matches.clear();
        self.selected = 0;
        self.dismissed = false;
        self.searching = true;
        self.refresh_matches();
    }

    pub(super) fn query(&self) -> Option<&str> {
        self.query.as_deref()
    }

    pub(super) fn apply_search_snapshot(&mut self, snapshot: PathSearchSnapshot) {
        if self.query.as_deref() != Some(snapshot.query.as_str()) {
            return;
        }
        self.file_matches = snapshot
            .matches
            .into_iter()
            .filter_map(|matched| {
                let path = matched.path.to_str()?.to_owned();
                Some(MentionMatch {
                    label: path.clone(),
                    completion: path,
                    kind: MentionMatchKind::File,
                    indices: matched
                        .indices
                        .into_iter()
                        .map(|index| index as usize)
                        .collect(),
                    score: matched.score,
                })
            })
            .collect();
        self.searching = !snapshot.search_complete;
        self.refresh_matches();
    }

    pub(super) fn replace_plugin_catalog(&mut self, catalog: Vec<MentionPluginItem>) {
        self.plugin_catalog = catalog;
        self.refresh_matches();
    }

    pub(super) fn view(&self) -> Option<MentionPopupView<'_>> {
        (!self.dismissed && self.query.is_some()).then_some(MentionPopupView {
            matches: &self.matches,
            selected: self.selected,
            searching: self.searching,
        })
    }

    pub(super) fn select_previous(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.matches.len() - 1
        } else {
            self.selected - 1
        };
    }

    pub(super) fn select_next(&mut self) {
        if !self.matches.is_empty() {
            self.selected = (self.selected + 1) % self.matches.len();
        }
    }

    pub(super) fn select(&mut self, index: usize) -> bool {
        if !self.view().is_some_and(|view| index < view.matches.len()) {
            return false;
        }
        self.selected = index;
        true
    }

    pub(super) fn dismiss(&mut self) {
        self.dismissed = true;
    }

    pub(super) fn selected_completion(&self) -> Option<(Range<usize>, String)> {
        self.completion_at(self.selected)
    }

    pub(super) fn completion_at(&self, index: usize) -> Option<(Range<usize>, String)> {
        let view = self.view()?;
        let completion = view.matches.get(index)?.completion.clone();
        Some((self.token_range.clone()?, completion))
    }

    pub(super) fn clear(&mut self) {
        self.token_range = None;
        self.query = None;
        self.file_matches.clear();
        self.matches.clear();
        self.selected = 0;
        self.dismissed = false;
        self.searching = false;
    }

    fn refresh_matches(&mut self) {
        let Some(query) = self.query.as_deref() else {
            self.matches.clear();
            return;
        };
        let query = query.to_ascii_lowercase();
        self.matches = self
            .plugin_catalog
            .iter()
            .filter(|plugin| plugin.id.to_ascii_lowercase().starts_with(&query))
            .map(|plugin| MentionMatch {
                label: plugin.id.clone(),
                completion: format!("@{}", plugin.id),
                kind: MentionMatchKind::Plugin,
                indices: (0..query.chars().count()).collect(),
                score: u32::MAX,
            })
            .chain(self.file_matches.iter().cloned())
            .collect();
        self.selected = self.selected.min(self.matches.len().saturating_sub(1));
    }
}
