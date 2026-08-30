//! `$skill` query, completion, and exact Skill binding state.

use super::super::editor::TextArea;
use super::super::editor::TextElementId;
use std::ops::Range;
use zeta_protocol::SkillRef;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SkillSelectorItem {
    name: String,
    description: String,
    skill: SkillRef,
}

impl SkillSelectorItem {
    pub(crate) fn new(name: String, description: String, skill: SkillRef) -> Self {
        Self {
            name,
            description,
            skill,
        }
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn description(&self) -> &str {
        &self.description
    }

    #[cfg(test)]
    pub(crate) fn skill(&self) -> &SkillRef {
        &self.skill
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SkillSelectorView<'a> {
    pub(crate) items: &'a [SkillSelectorItem],
    pub(crate) selected: usize,
}

#[derive(Debug, Default, Eq, PartialEq)]
pub(in crate::components::chat_input) struct SkillSelector {
    catalog: Vec<SkillSelectorItem>,
    token_range: Option<Range<usize>>,
    query: Option<String>,
    matches: Vec<SkillSelectorItem>,
    selected: usize,
    dismissed: bool,
    bindings: Vec<(TextElementId, SkillRef)>,
}

impl SkillSelector {
    pub(in crate::components::chat_input) fn replace_catalog(
        &mut self,
        catalog: Vec<SkillSelectorItem>,
    ) {
        self.catalog = catalog;
        self.refresh_matches();
    }

    pub(in crate::components::chat_input) fn sync_textarea(&mut self, textarea: &TextArea) {
        self.bindings
            .retain(|(element_id, _)| textarea.has_element(*element_id));
        let Some((range, query)) = active_skill(textarea.text(), textarea.cursor()) else {
            self.close_popup();
            return;
        };
        if self.token_range.as_ref() == Some(&range) && self.query.as_deref() == Some(query) {
            return;
        }
        self.token_range = Some(range);
        self.query = Some(query.to_owned());
        self.selected = 0;
        self.dismissed = false;
        self.refresh_matches();
    }

    pub(in crate::components::chat_input) fn view(&self) -> Option<SkillSelectorView<'_>> {
        (!self.dismissed && self.query.is_some()).then_some(SkillSelectorView {
            items: &self.matches,
            selected: self.selected,
        })
    }

    pub(in crate::components::chat_input) fn select_previous(&mut self) {
        if self.matches.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.matches.len() - 1
        } else {
            self.selected - 1
        };
    }

    pub(in crate::components::chat_input) fn select_next(&mut self) {
        if !self.matches.is_empty() {
            self.selected = (self.selected + 1) % self.matches.len();
        }
    }

    pub(in crate::components::chat_input) fn select(&mut self, index: usize) -> bool {
        if !self.view().is_some_and(|view| index < view.items.len()) {
            return false;
        }
        self.selected = index;
        true
    }

    pub(in crate::components::chat_input) fn dismiss(&mut self) {
        self.dismissed = true;
    }

    pub(in crate::components::chat_input) fn complete_selected(
        &mut self,
        textarea: &mut TextArea,
    ) -> bool {
        self.complete_at(textarea, self.selected)
    }

    pub(in crate::components::chat_input) fn complete_at(
        &mut self,
        textarea: &mut TextArea,
        index: usize,
    ) -> bool {
        let Some(item) = self.view().and_then(|view| view.items.get(index)).cloned() else {
            return false;
        };
        let Some(range) = self.token_range.clone() else {
            return false;
        };
        textarea.replace_range(range, "");
        let element_id = textarea.insert_element(&format!("${}", item.name));
        self.bindings.push((element_id, item.skill));
        if !textarea.text()[textarea.cursor()..]
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            textarea.insert_text(" ");
        }
        self.close_popup();
        true
    }

    pub(in crate::components::chat_input) fn skill_for(
        &self,
        element_id: TextElementId,
    ) -> Option<&SkillRef> {
        self.bindings
            .iter()
            .find(|(candidate, _)| *candidate == element_id)
            .map(|(_, skill)| skill)
    }

    pub(in crate::components::chat_input) fn take_bindings(
        &mut self,
    ) -> Vec<(TextElementId, SkillRef)> {
        self.close_popup();
        std::mem::take(&mut self.bindings)
    }

    pub(in crate::components::chat_input) fn restore_bindings(
        &mut self,
        bindings: Vec<(TextElementId, SkillRef)>,
    ) {
        self.close_popup();
        self.bindings = bindings;
    }

    pub(in crate::components::chat_input) fn clear(&mut self) {
        self.close_popup();
        self.bindings.clear();
    }

    fn refresh_matches(&mut self) {
        let Some(query) = self.query.as_deref() else {
            self.matches.clear();
            return;
        };
        self.matches = self
            .catalog
            .iter()
            .filter(|item| item.name.starts_with(query))
            .cloned()
            .collect();
        self.selected = self.selected.min(self.matches.len().saturating_sub(1));
    }

    fn close_popup(&mut self) {
        self.token_range = None;
        self.query = None;
        self.matches.clear();
        self.selected = 0;
        self.dismissed = false;
    }
}

fn active_skill(text: &str, cursor: usize) -> Option<(Range<usize>, &str)> {
    if cursor > text.len() || !text.is_char_boundary(cursor) {
        return None;
    }
    let start = text[..cursor]
        .char_indices()
        .rfind(|(_, character)| character.is_whitespace())
        .map(|(index, character)| index + character.len_utf8())
        .unwrap_or(0);
    let end = text[cursor..]
        .char_indices()
        .find(|(_, character)| character.is_whitespace())
        .map(|(offset, _)| cursor + offset)
        .unwrap_or(text.len());
    let token = text.get(start..end)?;
    let query = token.strip_prefix('$')?;
    query
        .chars()
        .all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '-'
        })
        .then_some((start..end, query))
}

#[cfg(test)]
#[path = "skill_tests.rs"]
mod tests;
