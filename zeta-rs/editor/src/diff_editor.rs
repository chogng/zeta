//! Side-by-side diff projection composed from shared CodeEditor viewports.

use std::ops::Range;

use zeta_diff::{DiffDocument, DiffRowKind};
use zeta_ui::{Component, PaintRect, Point, Rect, UiScene};

use self::layout::{DiffEditorLayout, build_layout};
pub use self::style::DiffEditorStyle;
use crate::code_editor::{
    CodeEditor, CodeEditorHeader, CodeEditorInlineHighlight, CodeEditorRow, CodeEditorRowSource,
    CodeEditorViewport,
};

const DIVIDER_WIDTH: f32 = 1.0;

mod layout;
mod style;

/// The source side represented by a DiffEditor pane.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiffEditorSide {
    Original,
    Modified,
}

/// Persistent viewport state retained by the editor host while this document is inactive.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DiffEditorState {
    first_visible_row: usize,
    original_horizontal_column: usize,
    modified_horizontal_column: usize,
}

impl DiffEditorState {
    pub const fn first_visible_row(self) -> usize {
        self.first_visible_row
    }

    pub const fn horizontal_column(self, side: DiffEditorSide) -> usize {
        match side {
            DiffEditorSide::Original => self.original_horizontal_column,
            DiffEditorSide::Modified => self.modified_horizontal_column,
        }
    }

    pub fn set_horizontal_column(&mut self, side: DiffEditorSide, column: usize) {
        match side {
            DiffEditorSide::Original => self.original_horizontal_column = column,
            DiffEditorSide::Modified => self.modified_horizontal_column = column,
        }
    }

    pub fn scroll_rows(&mut self, delta: isize, row_count: usize, visible_row_capacity: usize) {
        let mut viewport = CodeEditorViewport::new(self.first_visible_row);
        viewport.scroll_rows(delta, row_count, visible_row_capacity);
        self.first_visible_row = viewport.first_visible_row();
    }

    pub fn clamp(&mut self, row_count: usize, visible_row_capacity: usize) {
        let mut viewport = CodeEditorViewport::new(self.first_visible_row);
        viewport.clamp(row_count, visible_row_capacity);
        self.first_visible_row = viewport.first_visible_row();
    }

    fn viewport(self, side: DiffEditorSide) -> CodeEditorViewport {
        CodeEditorViewport::new(self.first_visible_row)
            .with_horizontal_column(self.horizontal_column(side))
    }
}

/// Labels rendered above the original and modified CodeEditor panes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiffEditorLabels<'a> {
    original: &'a str,
    modified: &'a str,
}

impl<'a> DiffEditorLabels<'a> {
    pub const fn new(original: &'a str, modified: &'a str) -> Self {
        Self { original, modified }
    }

    const fn for_side(self, side: DiffEditorSide) -> &'a str {
        match side {
            DiffEditorSide::Original => self.original,
            DiffEditorSide::Modified => self.modified,
        }
    }
}

/// A mapped source location under a point in the visible DiffEditor body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiffEditorLocation {
    pub side: DiffEditorSide,
    pub row_index: usize,
    pub line_number: Option<usize>,
}

struct DiffSideRows<'a> {
    document: &'a DiffDocument,
    side: DiffEditorSide,
    style: &'a DiffEditorStyle,
}

impl CodeEditorRowSource for DiffSideRows<'_> {
    fn row_count(&self) -> usize {
        self.document.rows().len()
    }

    fn largest_line_number(&self) -> usize {
        match self.side {
            DiffEditorSide::Original => self.document.old_line_count(),
            DiffEditorSide::Modified => self.document.new_line_count(),
        }
    }

    fn row(&self, index: usize) -> Option<CodeEditorRow<'_>> {
        let row = self.document.rows().get(index)?;
        let line = match self.side {
            DiffEditorSide::Original => row.old(),
            DiffEditorSide::Modified => row.new_line(),
        };
        let background = self
            .style
            .line_background(row.kind(), self.side, line.is_some());
        let Some(line) = line else {
            return Some(CodeEditorRow::placeholder().with_background(background));
        };
        let mut code_row =
            CodeEditorRow::new(line.number(), line.text()).with_background(background);
        if let Some(marker) = change_marker(row.kind(), self.side) {
            code_row = code_row.with_marker(marker, self.style.marker_color(self.side));
        }
        let inline_color = self.style.inline_color(self.side);
        let highlights = row
            .inline_changes()
            .iter()
            .filter_map(|change| {
                let range = match self.side {
                    DiffEditorSide::Original => change.old_range(),
                    DiffEditorSide::Modified => change.new_range(),
                };
                (!range.is_empty()).then(|| CodeEditorInlineHighlight::new(range, inline_color))
            })
            .collect();
        Some(code_row.with_inline_highlights(highlights))
    }
}

/// Read-only, side-by-side composition of two shared [`CodeEditor`] viewports.
///
/// The host owns file identity, diff computation, active-tab selection, input routing, and the
/// retained [`DiffEditorState`]. DiffEditor owns only row-pair projection, synchronized vertical
/// position, side-specific decorations, and the divider between the two CodeEditor panes.
pub struct DiffEditor<'a> {
    bounds: Rect,
    document: &'a DiffDocument,
    state: DiffEditorState,
    labels: DiffEditorLabels<'a>,
    style: DiffEditorStyle,
}

impl<'a> DiffEditor<'a> {
    pub fn new(
        bounds: Rect,
        document: &'a DiffDocument,
        state: DiffEditorState,
        labels: DiffEditorLabels<'a>,
        style: DiffEditorStyle,
    ) -> Self {
        Self {
            bounds,
            document,
            state,
            labels,
            style,
        }
    }

    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn visible_row_capacity(&self) -> usize {
        let rows = self.rows(DiffEditorSide::Original);
        self.code_editor(DiffEditorSide::Original, &rows)
            .visible_row_capacity()
    }

    pub fn content_height(&self) -> f32 {
        let rows = self.rows(DiffEditorSide::Original);
        self.code_editor(DiffEditorSide::Original, &rows)
            .content_height()
    }

    pub fn visible_row_range(&self) -> Range<usize> {
        let rows = self.rows(DiffEditorSide::Original);
        self.code_editor(DiffEditorSide::Original, &rows)
            .visible_row_range()
    }

    pub fn location_at(&self, point: Point) -> Option<DiffEditorLocation> {
        let layout = self.layout();
        let side = if layout.original.contains(point) {
            DiffEditorSide::Original
        } else if layout.modified.contains(point) {
            DiffEditorSide::Modified
        } else {
            return None;
        };
        let rows = self.rows(side);
        let location = self.code_editor(side, &rows).location_at(point)?;
        Some(DiffEditorLocation {
            side,
            row_index: location.row_index,
            line_number: location.line_number,
        })
    }

    fn layout(&self) -> DiffEditorLayout {
        build_layout(self.bounds)
    }

    fn rows(&self, side: DiffEditorSide) -> DiffSideRows<'_> {
        DiffSideRows {
            document: self.document,
            side,
            style: &self.style,
        }
    }

    fn code_editor<'rows>(
        &'rows self,
        side: DiffEditorSide,
        rows: &'rows DiffSideRows<'_>,
    ) -> CodeEditor<'rows> {
        let layout = self.layout();
        let bounds = match side {
            DiffEditorSide::Original => layout.original,
            DiffEditorSide::Modified => layout.modified,
        };
        CodeEditor::new(
            bounds,
            rows,
            self.state.viewport(side),
            CodeEditorHeader::Label(self.labels.for_side(side)),
            self.style.code_editor().clone(),
        )
    }
}

impl Component for DiffEditor<'_> {
    fn paint(&self, scene: &mut UiScene) {
        if self.bounds.is_empty() {
            return;
        }
        let layout = self.layout();
        let original_rows = self.rows(DiffEditorSide::Original);
        let modified_rows = self.rows(DiffEditorSide::Modified);
        scene.with_clip(self.bounds, |scene| {
            scene.draw_component(&self.code_editor(DiffEditorSide::Original, &original_rows));
            scene.draw_component(&self.code_editor(DiffEditorSide::Modified, &modified_rows));
            scene.draw_rect(PaintRect::new(layout.divider, self.style.divider()));
        });
    }
}

fn change_marker(kind: DiffRowKind, side: DiffEditorSide) -> Option<&'static str> {
    match (kind, side) {
        (DiffRowKind::Added, DiffEditorSide::Modified)
        | (DiffRowKind::Modified, DiffEditorSide::Modified) => Some("+"),
        (DiffRowKind::Removed, DiffEditorSide::Original)
        | (DiffRowKind::Modified, DiffEditorSide::Original) => Some("−"),
        _ => None,
    }
}

#[cfg(test)]
#[path = "diff_editor_tests.rs"]
mod tests;
