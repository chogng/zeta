//! Unicode-safe composer editor state and atomic element handling.

use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use std::ops::Range;
use unicode_width::UnicodeWidthStr;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TextAreaOutcome {
    Consumed,
    Unhandled,
}

/// Stable identity for one atomic element during the lifetime of a draft.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TextElementId(u64);

#[derive(Debug, Eq, PartialEq)]
struct TextElement {
    id: TextElementId,
    range: Range<usize>,
}

/// Owns the editable text buffer, cursor, and editor keymap boundary.
///
/// Vim modes, motions, and operators belong in this component when that capability is added.
/// Chat-level submission stays in the parent composer; slash parsing stays in `SlashInput`.
#[derive(Debug, Eq, PartialEq)]
pub(super) struct TextArea {
    text: String,
    cursor: usize,
    elements: Vec<TextElement>,
    next_element_id: u64,
}

impl TextArea {
    pub(super) fn new() -> Self {
        Self {
            text: String::new(),
            cursor: 0,
            elements: Vec::new(),
            next_element_id: 0,
        }
    }

    pub(super) fn handle_key(&mut self, key: KeyEvent) -> TextAreaOutcome {
        if key
            .modifiers
            .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
        {
            return TextAreaOutcome::Unhandled;
        }

        match key.code {
            KeyCode::Backspace => {
                self.backspace();
                TextAreaOutcome::Consumed
            }
            KeyCode::Delete => {
                self.delete();
                TextAreaOutcome::Consumed
            }
            KeyCode::Left => {
                self.move_left();
                TextAreaOutcome::Consumed
            }
            KeyCode::Right => {
                self.move_right();
                TextAreaOutcome::Consumed
            }
            KeyCode::Home => {
                self.cursor = 0;
                TextAreaOutcome::Consumed
            }
            KeyCode::End => {
                self.cursor = self.text.len();
                TextAreaOutcome::Consumed
            }
            KeyCode::Char(character) => {
                let mut encoded = [0; 4];
                self.insert_text(character.encode_utf8(&mut encoded));
                TextAreaOutcome::Consumed
            }
            _ => TextAreaOutcome::Unhandled,
        }
    }

    pub(super) fn insert_text(&mut self, text: &str) {
        self.shift_elements_for_insertion(self.cursor, text.len());
        self.text.insert_str(self.cursor, text);
        self.cursor += text.len();
    }

    pub(super) fn insert_element(&mut self, text: &str) -> TextElementId {
        let start = self.cursor;
        self.insert_text(text);
        let range = start..self.cursor;
        self.mark_element(range)
    }

    pub(super) fn text(&self) -> &str {
        &self.text
    }

    pub(super) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(super) fn has_element(&self, expected: TextElementId) -> bool {
        self.elements.iter().any(|element| element.id == expected)
    }

    pub(super) fn elements(&self) -> impl Iterator<Item = (TextElementId, Range<usize>)> + '_ {
        self.elements
            .iter()
            .map(|element| (element.id, element.range.clone()))
    }

    pub(super) fn element_range(&self, expected: TextElementId) -> Option<Range<usize>> {
        self.elements
            .iter()
            .find(|element| element.id == expected)
            .map(|element| element.range.clone())
    }

    pub(super) fn mark_element(&mut self, range: Range<usize>) -> TextElementId {
        assert!(
            !range.is_empty()
                && range.end <= self.text.len()
                && self.text.is_char_boundary(range.start)
                && self.text.is_char_boundary(range.end),
            "element range must be non-empty and lie on text boundaries"
        );
        assert!(
            self.elements.iter().all(|element| {
                element.range.end <= range.start || element.range.start >= range.end
            }),
            "element range must not overlap another atomic element"
        );
        let id = TextElementId(self.next_element_id);
        self.next_element_id = self
            .next_element_id
            .checked_add(1)
            .expect("text element ID overflow");
        let index = self
            .elements
            .partition_point(|element| element.range.start < range.start);
        self.elements.insert(index, TextElement { id, range });
        id
    }

    pub(super) fn unmark_element(&mut self, expected: TextElementId) {
        self.elements.retain(|element| element.id != expected);
    }

    pub(super) fn replace_element(&mut self, element_id: TextElementId, text: &str) {
        let index = self
            .elements
            .iter()
            .position(|element| element.id == element_id)
            .expect("text element must exist before replacement");
        let old_range = self.elements[index].range.clone();
        let old_len = old_range.len();
        self.text.replace_range(old_range.clone(), text);
        self.elements[index].range.end = old_range.start + text.len();

        if text.len() >= old_len {
            let delta = text.len() - old_len;
            for element in &mut self.elements[index + 1..] {
                element.range.start += delta;
                element.range.end += delta;
            }
            if self.cursor >= old_range.end {
                self.cursor += delta;
            }
        } else {
            let delta = old_len - text.len();
            for element in &mut self.elements[index + 1..] {
                element.range.start -= delta;
                element.range.end -= delta;
            }
            if self.cursor >= old_range.end {
                self.cursor -= delta;
            }
        }
    }

    pub(super) fn replace_range(&mut self, range: Range<usize>, text: &str) {
        assert!(
            range.start <= range.end
                && range.end <= self.text.len()
                && self.text.is_char_boundary(range.start)
                && self.text.is_char_boundary(range.end),
            "replacement range must lie on text boundaries"
        );
        assert!(
            self.elements.iter().all(|element| {
                element.range.end <= range.start || element.range.start >= range.end
            }),
            "editable replacement must not overlap an atomic element"
        );
        self.cursor = range.start;
        self.remove_range(range);
        self.insert_text(text);
    }

    pub(super) fn cursor_display_width(&self) -> usize {
        self.text[..self.cursor].width()
    }

    pub(super) fn clear(&mut self) {
        self.text.clear();
        self.cursor = 0;
        self.elements.clear();
        self.next_element_id = 0;
    }

    fn backspace(&mut self) {
        if let Some(range) = self
            .elements
            .iter()
            .find(|element| element.range.end == self.cursor)
            .map(|element| element.range.clone())
        {
            self.cursor = range.start;
            self.remove_range(range);
            return;
        }

        let Some(previous) = self.previous_boundary() else {
            return;
        };
        self.remove_range(previous..self.cursor);
        self.cursor = previous;
    }

    fn delete(&mut self) {
        if let Some(range) = self
            .elements
            .iter()
            .find(|element| element.range.start == self.cursor)
            .map(|element| element.range.clone())
        {
            self.remove_range(range);
            return;
        }

        let Some(next) = self.next_boundary() else {
            return;
        };
        self.remove_range(self.cursor..next);
    }

    fn move_left(&mut self) {
        if let Some(element) = self
            .elements
            .iter()
            .find(|element| element.range.end == self.cursor)
        {
            self.cursor = element.range.start;
            return;
        }

        if let Some(previous) = self.previous_boundary() {
            self.cursor = previous;
        }
    }

    fn move_right(&mut self) {
        if let Some(element) = self
            .elements
            .iter()
            .find(|element| element.range.start == self.cursor)
        {
            self.cursor = element.range.end;
            return;
        }

        if let Some(next) = self.next_boundary() {
            self.cursor = next;
        }
    }

    fn shift_elements_for_insertion(&mut self, at: usize, inserted_len: usize) {
        for element in &mut self.elements {
            if element.range.start >= at {
                element.range.start += inserted_len;
                element.range.end += inserted_len;
            }
        }
    }

    fn remove_range(&mut self, removed: Range<usize>) {
        self.text.replace_range(removed.clone(), "");
        let removed_len = removed.end - removed.start;
        self.elements.retain_mut(|element| {
            if element.range.end <= removed.start {
                true
            } else if element.range.start >= removed.end {
                element.range.start -= removed_len;
                element.range.end -= removed_len;
                true
            } else {
                debug_assert_eq!(element.range, removed);
                false
            }
        });
    }

    fn previous_boundary(&self) -> Option<usize> {
        self.text[..self.cursor]
            .char_indices()
            .next_back()
            .map(|(index, _)| index)
    }

    fn next_boundary(&self) -> Option<usize> {
        self.text[self.cursor..]
            .char_indices()
            .nth(1)
            .map(|(offset, _)| self.cursor + offset)
            .or_else(|| (self.cursor < self.text.len()).then_some(self.text.len()))
    }
}

#[cfg(test)]
#[path = "editor_tests.rs"]
mod tests;
