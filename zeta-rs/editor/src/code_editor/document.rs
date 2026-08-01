//! Owned text snapshot used by the ordinary CodeEditor projection.

use std::{fmt, ops::Range};

use zeta_ui::TextInputCompositionCursor;

use super::analysis::CodeEditorAnalysis;
use super::{
    CodeEditorComposition, CodeEditorLanguage, CodeEditorPosition, CodeEditorRow,
    CodeEditorRowSource, CodeEditorSelection, CodeEditorSyntaxToken,
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
pub struct CodeEditorDocument {
    pub(super) text: String,
    pub(super) line_ranges: Vec<Range<usize>>,
    pub(super) anchor: usize,
    pub(super) cursor: usize,
    pub(super) preferred_column: Option<usize>,
    pub(super) composition: Option<Composition>,
    pub(super) undo: Vec<DocumentSnapshot>,
    pub(super) redo: Vec<DocumentSnapshot>,
    pub(super) syntax_tokens: Vec<Vec<CodeEditorSyntaxToken>>,
    pub(super) analysis: CodeEditorAnalysis,
}

impl Clone for CodeEditorDocument {
    fn clone(&self) -> Self {
        let mut clone = Self::from_text_with_language(&self.text, self.language());
        clone.anchor = self.anchor;
        clone.cursor = self.cursor;
        clone.preferred_column = self.preferred_column;
        clone.composition = self.composition.clone();
        clone.undo = self.undo.clone();
        clone.redo = self.redo.clone();
        clone
    }
}

impl fmt::Debug for CodeEditorDocument {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CodeEditorDocument")
            .field("text", &self.text)
            .field("line_ranges", &self.line_ranges)
            .field("anchor", &self.anchor)
            .field("cursor", &self.cursor)
            .field("preferred_column", &self.preferred_column)
            .field("composition", &self.composition)
            .field("undo", &self.undo)
            .field("redo", &self.redo)
            .field("syntax_tokens", &self.syntax_tokens)
            .field("language", &self.language())
            .finish()
    }
}

impl PartialEq for CodeEditorDocument {
    fn eq(&self, other: &Self) -> bool {
        self.text == other.text
            && self.line_ranges == other.line_ranges
            && self.anchor == other.anchor
            && self.cursor == other.cursor
            && self.preferred_column == other.preferred_column
            && self.composition == other.composition
            && self.undo == other.undo
            && self.redo == other.redo
            && self.syntax_tokens == other.syntax_tokens
            && self.language() == other.language()
    }
}

impl Eq for CodeEditorDocument {}

impl Default for CodeEditorDocument {
    fn default() -> Self {
        Self::from_text("")
    }
}

impl CodeEditorDocument {
    pub fn from_text(text: impl Into<String>) -> Self {
        Self::from_text_with_language(text, CodeEditorLanguage::PlainText)
    }

    pub fn from_text_with_language(text: impl Into<String>, language: CodeEditorLanguage) -> Self {
        let mut document = Self {
            text: text.into(),
            line_ranges: Vec::new(),
            anchor: 0,
            cursor: 0,
            preferred_column: None,
            composition: None,
            undo: Vec::new(),
            redo: Vec::new(),
            syntax_tokens: Vec::new(),
            analysis: CodeEditorAnalysis::default(),
        };
        document.analysis.set_language(language);
        document.reindex_lines();
        document.refresh_syntax();
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
        self.reindex_lines();
        self.refresh_syntax();
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

    pub const fn language(&self) -> CodeEditorLanguage {
        self.analysis.language()
    }

    pub fn set_language(&mut self, language: CodeEditorLanguage) {
        self.analysis.set_language(language);
        self.refresh_syntax();
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
        self.reindex_lines();
        self.refresh_syntax();
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
        let syntax = self.syntax_tokens.get(index).cloned().unwrap_or_default();
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

impl CodeEditorDocument {
    pub(super) fn refresh_syntax(&mut self) {
        self.syntax_tokens = self.analysis.synchronize(&self.text, &self.line_ranges);
    }

    pub(crate) fn syntax_tokens_for_row(&self, row_index: usize) -> &[CodeEditorSyntaxToken] {
        self.syntax_tokens
            .get(row_index)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}
