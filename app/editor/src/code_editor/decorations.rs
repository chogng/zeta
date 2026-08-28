use zui::ui::{PaintRect, Point, Rect, Size, UiScene};

use super::text_metrics::{display_columns, display_columns_until, expand_tabs};
use super::{
    CELL_WIDTH, CONTENT_HORIZONTAL_PADDING, CodeEditor, CodeEditorCaretStyle,
    CodeEditorInlineHighlight, CodeEditorSyntaxToken, ROW_HEIGHT, paint_text_block,
};

const BAR_CARET_WIDTH: f32 = 1.5;

impl CodeEditor<'_> {
    pub fn caret_bounds(&self) -> Option<Rect> {
        let caret = self.rows.caret()?;
        let visual_row = self.caret_visual_row()?;
        let visible = self.visible_row_range();
        if !visible.contains(&visual_row) {
            return None;
        }
        let line = self.visual_projection.line(visual_row)?;
        let row = self.rows.row(line.row_index)?;
        let text = row.text?;
        let layout = self.layout();
        let column = display_columns_until(text, caret.byte_offset.min(text.len()));
        let mut x = layout.content.origin.x
            + CONTENT_HORIZONTAL_PADDING
            + (column as isize - self.horizontal_origin_column(line) as isize) as f32 * CELL_WIDTH;
        if let Some(composition) = self.rows.composition()
            && let zui::ui::TextInputCompositionCursor::Visible(cursor) = composition.cursor
        {
            x += display_columns_until(composition.text, cursor.end) as f32 * CELL_WIDTH;
        }
        Some(Rect::from_xywh(
            x,
            layout.body.origin.y + (visual_row - visible.start) as f32 * ROW_HEIGHT,
            match self.caret_style {
                CodeEditorCaretStyle::Bar => BAR_CARET_WIDTH,
                CodeEditorCaretStyle::Block => CELL_WIDTH,
            },
            ROW_HEIGHT,
        ))
    }

    pub(super) fn paint_selection(
        &self,
        scene: &mut UiScene,
        bounds: Rect,
        row_index: usize,
        text: &str,
        origin_column: usize,
    ) {
        let Some(selection) = self.rows.selection() else {
            return;
        };
        if row_index < selection.start.row_index || row_index > selection.end.row_index {
            return;
        }
        let start = if row_index == selection.start.row_index {
            selection.start.byte_offset.min(text.len())
        } else {
            0
        };
        let end = if row_index == selection.end.row_index {
            selection.end.byte_offset.min(text.len())
        } else {
            text.len()
        };
        let start_column = display_columns_until(text, start);
        let end_column = display_columns_until(text, end);
        let width = (end_column - start_column) as f32 * CELL_WIDTH
            + if row_index < selection.end.row_index {
                CELL_WIDTH
            } else {
                0.0
            };
        if width <= 0.0 {
            return;
        }
        scene.draw_rect(PaintRect::new(
            Rect::from_xywh(
                bounds.origin.x
                    + CONTENT_HORIZONTAL_PADDING
                    + (start_column as isize - origin_column as isize) as f32 * CELL_WIDTH,
                bounds.origin.y,
                width,
                ROW_HEIGHT,
            ),
            self.style.selection(),
        ));
    }

    pub(super) fn paint_inline_highlights(
        &self,
        scene: &mut UiScene,
        bounds: Rect,
        text: &str,
        highlights: &[CodeEditorInlineHighlight],
        origin_column: usize,
    ) {
        for highlight in highlights {
            if highlight.range.is_empty()
                || highlight.range.end > text.len()
                || !text.is_char_boundary(highlight.range.start)
                || !text.is_char_boundary(highlight.range.end)
            {
                continue;
            }
            let start = display_columns_until(text, highlight.range.start);
            let end = display_columns_until(text, highlight.range.end);
            if end <= start {
                continue;
            }
            scene.draw_rect(PaintRect::new(
                Rect::from_xywh(
                    bounds.origin.x
                        + CONTENT_HORIZONTAL_PADDING
                        + (start as isize - origin_column as isize) as f32 * CELL_WIDTH,
                    bounds.origin.y,
                    (end - start) as f32 * CELL_WIDTH,
                    ROW_HEIGHT,
                ),
                highlight.color,
            ));
        }
    }

    pub(super) fn paint_syntax_tokens(
        &self,
        scene: &mut UiScene,
        bounds: Rect,
        text: &str,
        tokens: &[CodeEditorSyntaxToken],
        origin_column: usize,
    ) {
        for token in tokens {
            if token.range.is_empty()
                || token.range.end > text.len()
                || !text.is_char_boundary(token.range.start)
                || !text.is_char_boundary(token.range.end)
            {
                continue;
            }
            let start = display_columns_until(text, token.range.start);
            let token_text = &text[token.range.clone()];
            paint_text_block(
                scene,
                expand_tabs(token_text),
                Point::new(
                    bounds.origin.x
                        + CONTENT_HORIZONTAL_PADDING
                        + (start as isize - origin_column as isize) as f32 * CELL_WIDTH,
                    bounds.origin.y,
                ),
                Size::new(display_columns(token_text) as f32 * CELL_WIDTH, ROW_HEIGHT),
                self.style
                    .text_with_color(self.style.syntax_color(token.role)),
            );
        }
    }

    pub(super) fn paint_composition_and_caret(
        &self,
        scene: &mut UiScene,
        bounds: Rect,
        row_index: usize,
        visual_row: usize,
        text: &str,
        origin_column: usize,
    ) {
        let Some(caret) = self.rows.caret().filter(|caret| {
            caret.row_index == row_index && self.caret_visual_row() == Some(visual_row)
        }) else {
            return;
        };
        let column = display_columns_until(text, caret.byte_offset.min(text.len()));
        let base_x = bounds.origin.x
            + CONTENT_HORIZONTAL_PADDING
            + (column as isize - origin_column as isize) as f32 * CELL_WIDTH;
        let mut caret_x = base_x;
        if let Some(composition) = self.rows.composition() {
            let width = display_columns(composition.text) as f32 * CELL_WIDTH;
            paint_text_block(
                scene,
                expand_tabs(composition.text),
                Point::new(base_x, bounds.origin.y),
                Size::new(width, ROW_HEIGHT),
                self.style.text_style().clone(),
            );
            scene.draw_rect(PaintRect::new(
                Rect::from_xywh(base_x, bounds.bottom() - 1.0, width.max(CELL_WIDTH), 1.0),
                self.style.composition_underline(),
            ));
            if let zui::ui::TextInputCompositionCursor::Visible(cursor) = composition.cursor {
                caret_x += display_columns_until(composition.text, cursor.end) as f32 * CELL_WIDTH;
            }
        }
        if self.caret_visibility == zui::ui::CaretVisibility::Visible {
            scene.draw_rect(PaintRect::new(
                Rect::from_xywh(
                    caret_x,
                    bounds.origin.y,
                    match self.caret_style {
                        CodeEditorCaretStyle::Bar => BAR_CARET_WIDTH,
                        CodeEditorCaretStyle::Block => CELL_WIDTH,
                    },
                    ROW_HEIGHT,
                ),
                self.style.caret(),
            ));
        }
    }

    pub(super) fn paint_ghost_text(
        &self,
        scene: &mut UiScene,
        bounds: Rect,
        row_index: usize,
        visual_row: usize,
        text: &str,
        origin_column: usize,
    ) {
        let Some(ghost_text) = self
            .ghost_text
            .and_then(|text| text.split(['\r', '\n']).next())
            .filter(|text| !text.is_empty())
        else {
            return;
        };
        if self.rows.composition().is_some()
            || self
                .rows
                .selection()
                .is_some_and(|selection| selection.start != selection.end)
        {
            return;
        }
        let Some(caret) = self.rows.caret().filter(|caret| {
            caret.row_index == row_index && self.caret_visual_row() == Some(visual_row)
        }) else {
            return;
        };
        let column = display_columns_until(text, caret.byte_offset.min(text.len()));
        let x = bounds.origin.x
            + CONTENT_HORIZONTAL_PADDING
            + (column as isize - origin_column as isize) as f32 * CELL_WIDTH;
        paint_text_block(
            scene,
            expand_tabs(ghost_text),
            Point::new(x, bounds.origin.y),
            Size::new(display_columns(ghost_text) as f32 * CELL_WIDTH, ROW_HEIGHT),
            self.style.muted_text_style(),
        );
    }
}
