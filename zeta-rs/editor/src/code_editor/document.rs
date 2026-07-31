//! Owned text snapshot used by the ordinary CodeEditor projection.

use std::ops::Range;

use zeta_ui::TextInputCompositionCursor;

use super::{
    CodeEditorComposition, CodeEditorPosition, CodeEditorRow, CodeEditorRowSource,
    CodeEditorSelection, CodeEditorSyntaxHighlighter, CodeEditorSyntaxToken,
};

pub(super) const HISTORY_LIMIT: usize = 100;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Composition {
    pub(super) text: String,
    pub(super) cursor: TextInputCompositionCursor,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DocumentSnapshot {
    text: String,
    anchor: usize,
    cursor: usize,
}

/// Owned text snapshot projected as numbered CodeEditor rows.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CodeEditorDocument {
    pub(super) text: String,
    pub(super) line_ranges: Vec<Range<usize>>,
    pub(super) anchor: usize,
    pub(super) cursor: usize,
    pub(super) preferred_column: Option<usize>,
    pub(super) composition: Option<Composition>,
    pub(super) undo: Vec<DocumentSnapshot>,
    pub(super) redo: Vec<DocumentSnapshot>,
    pub(super) syntax: Vec<Vec<CodeEditorSyntaxToken>>,
}

impl CodeEditorDocument {
    pub fn from_text(text: impl Into<String>) -> Self {
        let mut document = Self {
            text: text.into(),
            line_ranges: Vec::new(),
            anchor: 0,
            cursor: 0,
            preferred_column: None,
            composition: None,
            undo: Vec::new(),
            redo: Vec::new(),
            syntax: Vec::new(),
        };
        document.reindex_lines();
        document
    }

    pub fn replace_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.anchor = 0;
        self.cursor = 0;
        self.preferred_column = None;
        self.composition = None;
        self.undo.clear();
        self.redo.clear();
        self.syntax.clear();
        self.reindex_lines();
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn anchor(&self) -> usize {
        self.anchor
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    pub fn selected_text(&self) -> Option<&str> {
        (self.anchor != self.cursor).then(|| &self.text[self.selection_range()])
    }

    pub fn set_selection(&mut self, anchor: CodeEditorPosition, cursor: CodeEditorPosition) {
        self.cancel_composition();
        self.anchor = self.offset_for_position(anchor);
        self.cursor = self.offset_for_position(cursor);
        self.preferred_column = None;
    }

    /// Moves or extends the committed selection to a projected editor position.
    pub fn move_to(&mut self, position: CodeEditorPosition, mode: super::CodeEditorSelectionMode) {
        self.cancel_composition();
        self.cursor = self.offset_for_position(position);
        if mode == super::CodeEditorSelectionMode::Move {
            self.anchor = self.cursor;
        }
        self.preferred_column = None;
    }

    pub fn apply_syntax(&mut self, highlighter: &dyn CodeEditorSyntaxHighlighter) {
        self.syntax = self
            .line_ranges
            .iter()
            .enumerate()
            .map(|(index, range)| {
                let text = &self.text[range.clone()];
                highlighter
                    .highlight_line(index + 1, text)
                    .into_iter()
                    .filter(|token| valid_token(text, token))
                    .collect()
            })
            .collect();
    }

    pub(super) fn reindex_lines(&mut self) {
        self.line_ranges.clear();
        let bytes = self.text.as_bytes();
        let mut start = 0;
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'\n' => {
                    let end = if index > start && bytes[index - 1] == b'\r' {
                        index - 1
                    } else {
                        index
                    };
                    self.line_ranges.push(start..end);
                    start = index + 1;
                }
                b'\r' if bytes.get(index + 1) != Some(&b'\n') => {
                    self.line_ranges.push(start..index);
                    start = index + 1;
                }
                _ => {}
            }
            index += 1;
        }
        if start <= self.text.len() {
            self.line_ranges.push(start..self.text.len());
        }
        self.cursor = self.cursor.min(self.text.len());
        self.anchor = self.anchor.min(self.text.len());
        while !self.text.is_char_boundary(self.cursor) {
            self.cursor -= 1;
        }
        while !self.text.is_char_boundary(self.anchor) {
            self.anchor -= 1;
        }
    }

    pub(super) fn snapshot(&self) -> DocumentSnapshot {
        DocumentSnapshot {
            text: self.text.clone(),
            anchor: self.anchor,
            cursor: self.cursor,
        }
    }

    pub(super) fn restore(&mut self, snapshot: DocumentSnapshot) {
        self.text = snapshot.text;
        self.anchor = snapshot.anchor;
        self.cursor = snapshot.cursor;
        self.preferred_column = None;
        self.composition = None;
        self.syntax.clear();
        self.reindex_lines();
    }

    pub(super) fn current_line_range(&self) -> Range<usize> {
        self.line_ranges
            .get(self.row_index_for_offset(self.cursor))
            .cloned()
            .unwrap_or(self.text.len()..self.text.len())
    }

    pub(super) fn row_index_for_offset(&self, offset: usize) -> usize {
        self.line_ranges
            .iter()
            .position(|range| offset <= range.end)
            .unwrap_or_else(|| self.line_ranges.len().saturating_sub(1))
    }

    fn position_for_offset(&self, offset: usize) -> CodeEditorPosition {
        let row_index = self.row_index_for_offset(offset);
        let start = self
            .line_ranges
            .get(row_index)
            .map(|range| range.start)
            .unwrap_or(0);
        CodeEditorPosition {
            row_index,
            byte_offset: offset.saturating_sub(start),
        }
    }

    fn offset_for_position(&self, position: CodeEditorPosition) -> usize {
        let Some(range) = self.line_ranges.get(position.row_index) else {
            return self.text.len();
        };
        let relative = position.byte_offset.min(range.end - range.start);
        let mut offset = range.start + relative;
        while !self.text.is_char_boundary(offset) {
            offset -= 1;
        }
        offset
    }
}

impl CodeEditorRowSource for CodeEditorDocument {
    fn row_count(&self) -> usize {
        self.line_ranges.len()
    }

    fn largest_line_number(&self) -> usize {
        self.line_ranges.len()
    }

    fn row(&self, index: usize) -> Option<CodeEditorRow<'_>> {
        let range = self.line_ranges.get(index)?.clone();
        let syntax = self.syntax.get(index).cloned().unwrap_or_default();
        Some(CodeEditorRow::new(index + 1, &self.text[range]).with_syntax_tokens(syntax))
    }

    fn caret(&self) -> Option<CodeEditorPosition> {
        Some(self.position_for_offset(self.cursor))
    }

    fn selection(&self) -> Option<CodeEditorSelection> {
        let selection = self.selection_range();
        (selection.start != selection.end).then(|| CodeEditorSelection {
            start: self.position_for_offset(selection.start),
            end: self.position_for_offset(selection.end),
        })
    }

    fn composition(&self) -> Option<CodeEditorComposition<'_>> {
        self.composition
            .as_ref()
            .map(|composition| CodeEditorComposition {
                text: &composition.text,
                cursor: &composition.cursor,
            })
    }
}

fn valid_token(text: &str, token: &CodeEditorSyntaxToken) -> bool {
    token.range.start < token.range.end
        && token.range.end <= text.len()
        && text.is_char_boundary(token.range.start)
        && text.is_char_boundary(token.range.end)
}
