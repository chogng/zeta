//! Automatic newline indentation and line-oriented indent commands.

use std::ops::Range;

use super::CodeEditorDocument;

#[derive(Clone, Debug, Eq, PartialEq)]
enum IndentationKind {
    Tabs,
    Spaces(usize),
}

/// Editor-owned indentation unit used by newline, indent, and outdent commands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeEditorIndentation {
    kind: IndentationKind,
}

impl CodeEditorIndentation {
    /// Uses one tab character for each indentation level.
    pub const fn tabs() -> Self {
        Self {
            kind: IndentationKind::Tabs,
        }
    }

    /// Uses `width` spaces for each indentation level.
    ///
    /// A zero width is normalized to one so editing commands always make progress.
    pub const fn spaces(width: usize) -> Self {
        Self {
            kind: IndentationKind::Spaces(if width == 0 { 1 } else { width }),
        }
    }

    fn unit(&self) -> String {
        match self.kind {
            IndentationKind::Tabs => "\t".to_owned(),
            IndentationKind::Spaces(width) => " ".repeat(width),
        }
    }

    fn removable_prefix_len(&self, text: &str) -> usize {
        if text.starts_with('\t') {
            return 1;
        }
        let maximum = match self.kind {
            IndentationKind::Tabs => 1,
            IndentationKind::Spaces(width) => width,
        };
        text.bytes()
            .take(maximum)
            .take_while(|byte| *byte == b' ')
            .count()
    }
}

impl Default for CodeEditorIndentation {
    fn default() -> Self {
        Self::spaces(4)
    }
}

impl CodeEditorDocument {
    pub const fn indentation(&self) -> &CodeEditorIndentation {
        &self.indentation
    }

    pub fn set_indentation(&mut self, indentation: CodeEditorIndentation) {
        self.indentation = indentation;
    }

    pub(super) fn insert_newline_with_indentation(&mut self) {
        self.cancel_composition();
        let selection = self.selection_range();
        let insertion = selection.start;
        let row = self.row_index_for_offset(insertion);
        let line = self
            .line_ranges
            .get(row)
            .cloned()
            .unwrap_or(insertion..insertion);
        let before = &self.text[line.start..insertion.min(line.end)];
        let after = &self.text[insertion.min(line.end)..line.end];
        let leading = before
            .char_indices()
            .take_while(|(_, character)| matches!(character, ' ' | '\t'))
            .last()
            .map_or(0, |(offset, character)| offset + character.len_utf8());
        let base = &before[..leading];
        let opener = before.trim_end().chars().next_back();
        let matching_close = match opener {
            Some('{') => Some('}'),
            Some('[') => Some(']'),
            Some('(') => Some(')'),
            _ => None,
        };
        let closes_pair = matching_close.is_some_and(|close| after.trim_start().starts_with(close));
        let unit = self.indentation.unit();
        let mut inserted = format!("\n{base}");
        if opener.is_some_and(|character| matches!(character, '{' | '[' | '(')) {
            inserted.push_str(&unit);
        }
        let cursor = selection.start + inserted.len();
        if closes_pair {
            inserted.push('\n');
            inserted.push_str(base);
        }
        self.checkpoint();
        self.text.replace_range(selection, &inserted);
        self.collapse(cursor);
        self.after_edit();
    }

    pub(super) fn indent(&mut self) {
        self.cancel_composition();
        let selection = self.selection_range();
        let unit = self.indentation.unit();
        if selection.is_empty() {
            self.checkpoint();
            self.text.insert_str(self.cursor, &unit);
            self.collapse(self.cursor + unit.len());
            self.after_edit();
            return;
        }
        let starts = self.selected_line_starts(selection.clone());
        self.checkpoint();
        for start in starts.iter().rev().copied() {
            self.text.insert_str(start, &unit);
        }
        self.anchor = offset_after_insertions(self.anchor, &starts, unit.len());
        self.cursor = offset_after_insertions(self.cursor, &starts, unit.len());
        self.after_edit();
    }

    pub(super) fn outdent(&mut self) {
        self.cancel_composition();
        let selection = self.selection_range();
        let starts = if selection.is_empty() {
            vec![self.current_line_range().start]
        } else {
            self.selected_line_starts(selection)
        };
        let removals = starts
            .into_iter()
            .filter_map(|start| {
                let length = self.indentation.removable_prefix_len(&self.text[start..]);
                (length > 0).then_some(start..start + length)
            })
            .collect::<Vec<_>>();
        if removals.is_empty() {
            return;
        }
        self.checkpoint();
        for range in removals.iter().rev() {
            self.text.replace_range(range.clone(), "");
        }
        self.anchor = offset_after_removals(self.anchor, &removals);
        self.cursor = offset_after_removals(self.cursor, &removals);
        self.after_edit();
    }

    fn selected_line_starts(&self, selection: Range<usize>) -> Vec<usize> {
        let start_row = self.row_index_for_offset(selection.start);
        let mut end_row = self.row_index_for_offset(selection.end);
        if selection.end > selection.start
            && self
                .line_ranges
                .get(end_row)
                .is_some_and(|line| line.start == selection.end)
        {
            end_row = end_row.saturating_sub(1);
        }
        self.line_ranges[start_row..=end_row]
            .iter()
            .map(|line| line.start)
            .collect()
    }
}

fn offset_after_insertions(offset: usize, starts: &[usize], inserted_len: usize) -> usize {
    offset + starts.iter().filter(|start| **start <= offset).count() * inserted_len
}

fn offset_after_removals(offset: usize, removals: &[Range<usize>]) -> usize {
    let mut transformed = offset;
    for removal in removals {
        if offset <= removal.start {
            break;
        }
        transformed = transformed.saturating_sub((offset - removal.start).min(removal.len()));
    }
    transformed
}

#[cfg(test)]
#[path = "indentation_tests.rs"]
mod tests;
