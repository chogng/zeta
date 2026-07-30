use std::ops::Range;

use unicode_segmentation::UnicodeSegmentation;
use zeta_ui::{TextInputCompositionCursor, TextInputCompositionEvent, TextInputSelectionMode};

use super::CodeEditorDocument;

/// Whether navigation collapses or extends the current CodeEditor selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeEditorSelectionMode {
    Move,
    Extend,
}

impl From<CodeEditorSelectionMode> for TextInputSelectionMode {
    fn from(mode: CodeEditorSelectionMode) -> Self {
        match mode {
            CodeEditorSelectionMode::Move => Self::Move,
            CodeEditorSelectionMode::Extend => Self::Extend,
        }
    }
}

/// Platform-independent editing commands accepted by a multiline CodeEditor document.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodeEditorCommand {
    Insert(String),
    Newline,
    MoveLeft(CodeEditorSelectionMode),
    MoveRight(CodeEditorSelectionMode),
    MoveUp(CodeEditorSelectionMode),
    MoveDown(CodeEditorSelectionMode),
    MoveToLineStart(CodeEditorSelectionMode),
    MoveToLineEnd(CodeEditorSelectionMode),
    SelectAll,
    Backspace,
    DeleteForward,
    Undo,
    Redo,
}

impl CodeEditorDocument {
    pub fn apply(&mut self, command: CodeEditorCommand) {
        match command {
            CodeEditorCommand::Insert(text) => self.insert(&editable_text(&text)),
            CodeEditorCommand::Newline => self.insert("\n"),
            CodeEditorCommand::MoveLeft(mode) => self.move_left(mode),
            CodeEditorCommand::MoveRight(mode) => self.move_right(mode),
            CodeEditorCommand::MoveUp(mode) => self.move_vertical(-1, mode),
            CodeEditorCommand::MoveDown(mode) => self.move_vertical(1, mode),
            CodeEditorCommand::MoveToLineStart(mode) => {
                self.move_cursor(self.current_line_range().start, mode)
            }
            CodeEditorCommand::MoveToLineEnd(mode) => {
                self.move_cursor(self.current_line_range().end, mode)
            }
            CodeEditorCommand::SelectAll => {
                self.cancel_composition();
                self.anchor = 0;
                self.cursor = self.text.len();
            }
            CodeEditorCommand::Backspace => self.backspace(),
            CodeEditorCommand::DeleteForward => self.delete_forward(),
            CodeEditorCommand::Undo => self.undo(),
            CodeEditorCommand::Redo => self.redo(),
        }
    }

    pub fn apply_composition(&mut self, event: TextInputCompositionEvent) {
        match event {
            TextInputCompositionEvent::Preedit { text, cursor } => {
                let text = editable_text(&text);
                if text.is_empty() {
                    self.composition = None;
                } else {
                    self.composition = Some(super::document::Composition {
                        cursor: clamp_composition_cursor(&text, cursor),
                        text,
                    });
                }
            }
            TextInputCompositionEvent::Commit(text) => {
                self.composition = None;
                self.insert(&editable_text(&text));
            }
            TextInputCompositionEvent::Cancel => self.cancel_composition(),
        }
    }

    pub fn cancel_composition(&mut self) {
        self.composition = None;
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    fn insert(&mut self, text: &str) {
        self.cancel_composition();
        if text.is_empty() {
            return;
        }
        self.checkpoint();
        let selection = self.selection_range();
        self.text.replace_range(selection.clone(), text);
        self.collapse(selection.start + text.len());
        self.after_edit();
    }

    fn move_left(&mut self, mode: CodeEditorSelectionMode) {
        self.cancel_composition();
        if mode == CodeEditorSelectionMode::Move && self.has_selection() {
            self.collapse(self.selection_range().start);
            return;
        }
        let cursor = previous_boundary(&self.text, self.cursor);
        self.move_cursor(cursor, mode);
    }

    fn move_right(&mut self, mode: CodeEditorSelectionMode) {
        self.cancel_composition();
        if mode == CodeEditorSelectionMode::Move && self.has_selection() {
            self.collapse(self.selection_range().end);
            return;
        }
        let cursor = next_boundary(&self.text, self.cursor);
        self.move_cursor(cursor, mode);
    }

    fn move_vertical(&mut self, direction: isize, mode: CodeEditorSelectionMode) {
        self.cancel_composition();
        let row = self.row_index_for_offset(self.cursor);
        let target = if direction.is_negative() {
            row.checked_sub(direction.unsigned_abs())
        } else {
            row.checked_add(direction as usize)
                .filter(|index| *index < self.line_ranges.len())
        };
        let Some(target) = target else {
            return;
        };
        let current = self.current_line_range();
        let requested_column = self.preferred_column.unwrap_or_else(|| {
            super::text_metrics::display_columns_until(
                &self.text[current.clone()],
                self.cursor.saturating_sub(current.start),
            )
        });
        let target_range = self.line_ranges[target].clone();
        let relative = super::text_metrics::byte_offset_for_column(
            &self.text[target_range.clone()],
            requested_column,
        );
        self.preferred_column = Some(requested_column);
        self.move_cursor_preserving_column(target_range.start + relative, mode);
    }

    fn backspace(&mut self) {
        self.cancel_composition();
        if self.delete_selection() {
            return;
        }
        let start = previous_boundary(&self.text, self.cursor);
        if start == self.cursor {
            return;
        }
        self.checkpoint();
        self.text.replace_range(start..self.cursor, "");
        self.collapse(start);
        self.after_edit();
    }

    fn delete_forward(&mut self) {
        self.cancel_composition();
        if self.delete_selection() {
            return;
        }
        let end = next_boundary(&self.text, self.cursor);
        if end == self.cursor {
            return;
        }
        self.checkpoint();
        self.text.replace_range(self.cursor..end, "");
        self.after_edit();
    }

    fn delete_selection(&mut self) -> bool {
        if !self.has_selection() {
            return false;
        }
        self.checkpoint();
        let selection = self.selection_range();
        self.text.replace_range(selection.clone(), "");
        self.collapse(selection.start);
        self.after_edit();
        true
    }

    fn checkpoint(&mut self) {
        if self.undo.len() == super::document::HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.undo.push(self.snapshot());
        self.redo.clear();
    }

    fn undo(&mut self) {
        let Some(snapshot) = self.undo.pop() else {
            return;
        };
        self.redo.push(self.snapshot());
        self.restore(snapshot);
    }

    fn redo(&mut self) {
        let Some(snapshot) = self.redo.pop() else {
            return;
        };
        self.undo.push(self.snapshot());
        self.restore(snapshot);
    }

    fn after_edit(&mut self) {
        self.preferred_column = None;
        self.syntax.clear();
        self.reindex_lines();
    }

    fn move_cursor(&mut self, cursor: usize, mode: CodeEditorSelectionMode) {
        self.preferred_column = None;
        self.move_cursor_preserving_column(cursor, mode);
    }

    fn move_cursor_preserving_column(&mut self, cursor: usize, mode: CodeEditorSelectionMode) {
        self.cursor = clamp_boundary(&self.text, cursor);
        if mode == CodeEditorSelectionMode::Move {
            self.anchor = self.cursor;
        }
    }

    fn collapse(&mut self, cursor: usize) {
        self.anchor = cursor;
        self.cursor = cursor;
        self.preferred_column = None;
    }

    fn has_selection(&self) -> bool {
        self.anchor != self.cursor
    }

    pub(super) fn selection_range(&self) -> Range<usize> {
        self.anchor.min(self.cursor)..self.anchor.max(self.cursor)
    }
}

fn previous_boundary(text: &str, cursor: usize) -> usize {
    let cursor = clamp_boundary(text, cursor);
    if cursor >= 2 && &text.as_bytes()[cursor - 2..cursor] == b"\r\n" {
        return cursor - 2;
    }
    text[..cursor]
        .grapheme_indices(true)
        .next_back()
        .map(|(index, _)| index)
        .unwrap_or(0)
}

fn next_boundary(text: &str, cursor: usize) -> usize {
    let cursor = clamp_boundary(text, cursor);
    if text.as_bytes().get(cursor..cursor + 2) == Some(b"\r\n") {
        return cursor + 2;
    }
    text[cursor..]
        .grapheme_indices(true)
        .nth(1)
        .map(|(index, _)| cursor + index)
        .unwrap_or(text.len())
}

fn clamp_boundary(text: &str, requested: usize) -> usize {
    let mut index = requested.min(text.len());
    while !text.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn editable_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .chars()
        .filter(|character| matches!(character, '\n' | '\t') || !character.is_control())
        .collect()
}

fn clamp_composition_cursor(
    text: &str,
    cursor: TextInputCompositionCursor,
) -> TextInputCompositionCursor {
    match cursor {
        TextInputCompositionCursor::Visible(cursor) => {
            let start = clamp_boundary(text, cursor.start);
            let end = clamp_boundary(text, cursor.end);
            TextInputCompositionCursor::Visible(start.min(end)..start.max(end))
        }
        TextInputCompositionCursor::Hidden => TextInputCompositionCursor::Hidden,
    }
}
