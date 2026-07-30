//! Shared code viewport and lazy visual-row projection.

use std::ops::Range;

use zeta_ui::{Color, Component, PaintRect, Point, Rect, Size, TextBlock, UiScene};

pub use self::document::CodeEditorDocument;
pub use self::editing::{CodeEditorCommand, CodeEditorSelectionMode};
use self::layout::{CodeEditorLayout, build_layout};
pub use self::style::CodeEditorStyle;
pub use self::syntax::{CodeEditorSyntaxHighlighter, CodeEditorSyntaxToken};
use self::text_metrics::{display_columns, expand_tabs};

const HEADER_HEIGHT: f32 = 32.0;
const ROW_HEIGHT: f32 = 20.0;
const CELL_WIDTH: f32 = 8.0;
const MARKER_WIDTH: f32 = 16.0;
const GUTTER_HORIZONTAL_PADDING: f32 = 8.0;
const CONTENT_HORIZONTAL_PADDING: f32 = 8.0;
const TAB_WIDTH: usize = 4;

mod decorations;
mod document;
mod editing;
mod layout;
mod style;
mod syntax;
mod text_metrics;

/// One caret or selection endpoint in a projected visual row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodeEditorPosition {
    pub row_index: usize,
    pub byte_offset: usize,
}

/// Ordered selection endpoints projected into CodeEditor rows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodeEditorSelection {
    pub start: CodeEditorPosition,
    pub end: CodeEditorPosition,
}

/// Active platform preedit projected at the committed caret.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodeEditorComposition<'a> {
    pub text: &'a str,
    pub cursor: &'a zeta_ui::TextInputCompositionCursor,
}

/// Optional header treatment above a CodeEditor viewport.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CodeEditorHeader<'a> {
    #[default]
    Hidden,
    Label(&'a str),
}

impl CodeEditorHeader<'_> {
    const fn height(self) -> f32 {
        match self {
            Self::Hidden => 0.0,
            Self::Label(_) => HEADER_HEIGHT,
        }
    }
}

/// Retained scroll position for one CodeEditor document view.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CodeEditorViewport {
    first_visible_row: usize,
    horizontal_column: usize,
}

impl CodeEditorViewport {
    pub const fn new(first_visible_row: usize) -> Self {
        Self {
            first_visible_row,
            horizontal_column: 0,
        }
    }

    pub const fn with_horizontal_column(mut self, column: usize) -> Self {
        self.horizontal_column = column;
        self
    }

    pub const fn first_visible_row(self) -> usize {
        self.first_visible_row
    }

    pub const fn horizontal_column(self) -> usize {
        self.horizontal_column
    }

    pub fn scroll_rows(&mut self, delta: isize, row_count: usize, visible_row_capacity: usize) {
        let maximum = row_count.saturating_sub(visible_row_capacity);
        self.first_visible_row = if delta.is_negative() {
            self.first_visible_row.saturating_sub(delta.unsigned_abs())
        } else {
            self.first_visible_row
                .saturating_add(delta as usize)
                .min(maximum)
        };
    }

    pub fn set_horizontal_column(&mut self, column: usize) {
        self.horizontal_column = column;
    }

    pub fn clamp(&mut self, row_count: usize, visible_row_capacity: usize) {
        self.first_visible_row = self
            .first_visible_row
            .min(row_count.saturating_sub(visible_row_capacity));
    }
}

/// One byte-range highlight painted below code text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeEditorInlineHighlight {
    range: Range<usize>,
    color: Color,
}

impl CodeEditorInlineHighlight {
    pub const fn new(range: Range<usize>, color: Color) -> Self {
        Self { range, color }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CodeEditorMarker<'a> {
    text: &'a str,
    color: Color,
}

/// One visual row supplied to CodeEditor independently from document storage.
#[derive(Clone, Debug, PartialEq)]
pub struct CodeEditorRow<'a> {
    line_number: Option<usize>,
    text: Option<&'a str>,
    marker: Option<CodeEditorMarker<'a>>,
    background: Option<Color>,
    inline_highlights: Vec<CodeEditorInlineHighlight>,
    syntax_tokens: Vec<CodeEditorSyntaxToken>,
}

impl<'a> CodeEditorRow<'a> {
    pub const fn new(line_number: usize, text: &'a str) -> Self {
        Self {
            line_number: Some(line_number),
            text: Some(text),
            marker: None,
            background: None,
            inline_highlights: Vec::new(),
            syntax_tokens: Vec::new(),
        }
    }

    pub const fn placeholder() -> Self {
        Self {
            line_number: None,
            text: None,
            marker: None,
            background: None,
            inline_highlights: Vec::new(),
            syntax_tokens: Vec::new(),
        }
    }

    pub const fn with_marker(mut self, text: &'a str, color: Color) -> Self {
        self.marker = Some(CodeEditorMarker { text, color });
        self
    }

    pub const fn with_background(mut self, background: Color) -> Self {
        self.background = Some(background);
        self
    }

    pub fn with_inline_highlights(mut self, highlights: Vec<CodeEditorInlineHighlight>) -> Self {
        self.inline_highlights = highlights;
        self
    }

    pub fn with_syntax_tokens(mut self, tokens: Vec<CodeEditorSyntaxToken>) -> Self {
        self.syntax_tokens = tokens;
        self
    }
}

/// Lazy visual-row source consumed by CodeEditor.
///
/// Implementations retain authoritative document or projection data and return only the requested
/// visible row. A row source must keep row ordering and line-number identity stable for the
/// lifetime of one presentation frame. Diff projections may return placeholders for alignment;
/// ordinary documents should return real numbered rows.
pub trait CodeEditorRowSource {
    /// Returns the number of visual rows addressable in this frame.
    fn row_count(&self) -> usize;

    /// Returns the largest real line number used to size the gutter.
    fn largest_line_number(&self) -> usize;

    /// Projects one visual row, or `None` when the index is outside the source.
    fn row(&self, index: usize) -> Option<CodeEditorRow<'_>>;

    /// Returns the committed caret position for editable sources.
    fn caret(&self) -> Option<CodeEditorPosition> {
        None
    }

    /// Returns an ordered committed-text selection for editable sources.
    fn selection(&self) -> Option<CodeEditorSelection> {
        None
    }

    /// Returns active platform preedit without mutating committed text.
    fn composition(&self) -> Option<CodeEditorComposition<'_>> {
        None
    }
}

/// A mapped source location under a point in the visible CodeEditor body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CodeEditorLocation {
    pub row_index: usize,
    pub line_number: Option<usize>,
}

/// Native code viewport shared by ordinary and diff editor presentations.
///
/// This component owns row geometry, clipping, line-number paint, inline decorations, and
/// viewport projection. Its host owns editing commands, selection/caret state, file identity,
/// persistence, and input routing.
pub struct CodeEditor<'a> {
    bounds: Rect,
    rows: &'a dyn CodeEditorRowSource,
    viewport: CodeEditorViewport,
    header: CodeEditorHeader<'a>,
    style: CodeEditorStyle,
}

impl<'a> CodeEditor<'a> {
    pub fn new(
        bounds: Rect,
        rows: &'a dyn CodeEditorRowSource,
        viewport: CodeEditorViewport,
        header: CodeEditorHeader<'a>,
        style: CodeEditorStyle,
    ) -> Self {
        Self {
            bounds,
            rows,
            viewport,
            header,
            style,
        }
    }

    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn visible_row_capacity(&self) -> usize {
        let body_height = (self.bounds.size.height - self.header.height()).max(0.0);
        (body_height / ROW_HEIGHT).floor() as usize
    }

    pub fn content_height(&self) -> f32 {
        self.header.height() + self.rows.row_count() as f32 * ROW_HEIGHT
    }

    pub fn visible_row_range(&self) -> Range<usize> {
        let capacity = self.visible_row_capacity();
        let row_count = self.rows.row_count();
        let start = self
            .viewport
            .first_visible_row
            .min(row_count.saturating_sub(capacity));
        start..start.saturating_add(capacity).min(row_count)
    }

    pub fn location_at(&self, point: Point) -> Option<CodeEditorLocation> {
        let layout = self.layout();
        if !layout.body.contains(point) {
            return None;
        }
        let visible = self.visible_row_range();
        let row_index = visible.start
            + ((point.y - layout.body.origin.y) / ROW_HEIGHT)
                .floor()
                .max(0.0) as usize;
        let row = self.rows.row(row_index)?;
        Some(CodeEditorLocation {
            row_index,
            line_number: row.line_number,
        })
    }

    pub fn text_position_at(&self, point: Point) -> Option<CodeEditorPosition> {
        let layout = self.layout();
        let location = self.location_at(point)?;
        let row = self.rows.row(location.row_index)?;
        let text = row.text?;
        let visual_column = if point.x <= layout.content.origin.x + CONTENT_HORIZONTAL_PADDING {
            0
        } else {
            ((point.x - layout.content.origin.x - CONTENT_HORIZONTAL_PADDING) / CELL_WIDTH).floor()
                as usize
                + self.viewport.horizontal_column
        };
        Some(CodeEditorPosition {
            row_index: location.row_index,
            byte_offset: self::text_metrics::byte_offset_for_column(text, visual_column),
        })
    }

    fn layout(&self) -> CodeEditorLayout {
        build_layout(self.bounds, self.gutter_width(), self.header.height())
    }

    fn gutter_width(&self) -> f32 {
        let largest_line = self.rows.largest_line_number().max(1);
        let digits = largest_line.ilog10() as f32 + 1.0;
        MARKER_WIDTH + GUTTER_HORIZONTAL_PADDING * 2.0 + digits * CELL_WIDTH
    }

    fn paint_header(&self, scene: &mut UiScene, layout: CodeEditorLayout) {
        let CodeEditorHeader::Label(label) = self.header else {
            return;
        };
        scene.draw_rect(self.style.header_rect(layout.header));
        let label_bounds = Rect::from_xywh(
            layout.header.origin.x + CONTENT_HORIZONTAL_PADDING,
            layout.header.origin.y,
            (layout.header.size.width - CONTENT_HORIZONTAL_PADDING * 2.0).max(0.0),
            layout.header.size.height,
        );
        scene.draw_text(TextBlock::new(
            label,
            label_bounds.origin,
            label_bounds.size,
            self.style.header_text_style().clone(),
        ));
    }

    fn paint_row(
        &self,
        scene: &mut UiScene,
        layout: CodeEditorLayout,
        visible_row: usize,
        row_index: usize,
        row: CodeEditorRow<'_>,
    ) {
        let row_bounds = Rect::from_xywh(
            layout.body.origin.x,
            layout.body.origin.y + visible_row as f32 * ROW_HEIGHT,
            layout.body.size.width,
            ROW_HEIGHT,
        );
        scene.draw_rect(PaintRect::new(
            row_bounds,
            row.background.unwrap_or(self.style.surface()),
        ));
        let gutter_bounds = Rect::from_xywh(
            row_bounds.origin.x,
            row_bounds.origin.y,
            layout.gutter.size.width,
            ROW_HEIGHT,
        );
        scene.draw_rect(PaintRect::new(gutter_bounds, self.style.gutter()));

        let Some(text) = row.text else {
            return;
        };
        let line_number = row
            .line_number
            .expect("CodeEditor rows with text must have a line number");
        let number_width =
            (layout.gutter.size.width - MARKER_WIDTH - GUTTER_HORIZONTAL_PADDING * 2.0).max(0.0);
        let number = format!(
            "{:>width$}",
            line_number,
            width = (number_width / CELL_WIDTH).floor() as usize
        );
        let number_bounds = Rect::from_xywh(
            layout.gutter.origin.x + GUTTER_HORIZONTAL_PADDING,
            row_bounds.origin.y,
            number_width,
            ROW_HEIGHT,
        );
        scene.draw_text(TextBlock::new(
            number,
            number_bounds.origin,
            number_bounds.size,
            self.style.muted_text_style(),
        ));
        if let Some(marker) = row.marker {
            let marker_bounds = Rect::from_xywh(
                layout.gutter.right() - MARKER_WIDTH,
                row_bounds.origin.y,
                MARKER_WIDTH,
                ROW_HEIGHT,
            );
            scene.draw_text(TextBlock::new(
                marker.text,
                marker_bounds.origin,
                marker_bounds.size,
                self.style.text_with_color(marker.color),
            ));
        }

        let content_row_bounds = Rect::from_xywh(
            layout.content.origin.x,
            row_bounds.origin.y,
            layout.content.size.width,
            ROW_HEIGHT,
        );
        scene.with_clip(content_row_bounds, |scene| {
            self.paint_selection(scene, content_row_bounds, row_index, text);
            self.paint_inline_highlights(scene, content_row_bounds, text, &row.inline_highlights);
            let text_origin = Point::new(
                layout.content.origin.x + CONTENT_HORIZONTAL_PADDING
                    - self.viewport.horizontal_column as f32 * CELL_WIDTH,
                row_bounds.origin.y,
            );
            scene.draw_text(TextBlock::new(
                expand_tabs(text),
                text_origin,
                Size::new(display_columns(text) as f32 * CELL_WIDTH, ROW_HEIGHT),
                self.style.text_style().clone(),
            ));
            self.paint_syntax_tokens(scene, content_row_bounds, text, &row.syntax_tokens);
            self.paint_composition_and_caret(scene, content_row_bounds, row_index, text);
        });
    }
}

impl Component for CodeEditor<'_> {
    fn paint(&self, scene: &mut UiScene) {
        if self.bounds.is_empty() {
            return;
        }
        let layout = self.layout();
        scene.with_clip(self.bounds, |scene| {
            scene.draw_rect(PaintRect::new(self.bounds, self.style.surface()));
            self.paint_header(scene, layout);
            let visible = self.visible_row_range();
            for (visible_row, row_index) in visible.enumerate() {
                let Some(row) = self.rows.row(row_index) else {
                    continue;
                };
                self.paint_row(scene, layout, visible_row, row_index, row);
            }
        });
    }
}

#[cfg(test)]
#[path = "code_editor_tests.rs"]
mod tests;
