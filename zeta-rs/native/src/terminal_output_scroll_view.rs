use std::ops::Range;

use zeta_ui::{
    Point, Rect, ScrollAxis, ScrollCommand, ScrollState, ScrollView, ScrollViewport,
    ScrollbarPresentation, Size, UiScene,
};

use crate::shell_style::ShellPalette;
use crate::terminal_projection::block_view_range;

/// Product adapter from bottom-relative terminal history to a top-relative ScrollView.
#[derive(Clone, Copy)]
pub(crate) struct TerminalOutputScrollView {
    bounds: Rect,
    line_count: usize,
    line_capacity: usize,
    line_height: f32,
    scroll_offset: usize,
    scrollbar_presentation: ScrollbarPresentation,
    palette: ShellPalette,
}

impl TerminalOutputScrollView {
    pub(crate) fn new(
        bounds: Rect,
        line_count: usize,
        line_height: f32,
        scroll_offset: usize,
        scrollbar_presentation: ScrollbarPresentation,
        palette: ShellPalette,
    ) -> Self {
        assert!(
            line_height.is_finite() && line_height > 0.0,
            "Terminal output line height must be positive and finite"
        );
        let line_capacity = ((bounds.size.height / line_height).floor() as usize).max(1);
        Self {
            bounds,
            line_count,
            line_capacity,
            line_height,
            scroll_offset: scroll_offset.min(line_count.saturating_sub(line_capacity)),
            scrollbar_presentation,
            palette,
        }
    }

    pub(crate) fn visible_line_range(self) -> Range<usize> {
        block_view_range(self.line_count, self.line_capacity, self.scroll_offset)
    }

    pub(crate) fn scroll_view(self) -> ScrollView {
        let mut state = ScrollState::default();
        let first_visible_line = self.visible_line_range().start;
        state.apply(
            ScrollCommand::ToOffset(Point::new(
                0.0,
                first_visible_line as f32 * self.line_height,
            )),
            self.metrics(),
            ScrollAxis::Vertical,
        );
        ScrollView::new(
            self.bounds,
            self.metrics().content(),
            state,
            ScrollAxis::Vertical,
            self.palette.terminal_scroll_view_style(),
        )
        .with_scrollbar_presentation(self.scrollbar_presentation)
    }

    pub(crate) fn draw<R>(
        self,
        scene: &mut UiScene,
        draw_content: impl FnOnce(&mut UiScene, ScrollViewport, Range<usize>) -> R,
    ) -> R {
        self.scroll_view().draw(scene, |scene, viewport| {
            draw_content(scene, viewport, self.visible_line_range())
        })
    }

    fn metrics(self) -> zeta_ui::ScrollMetrics {
        let rendered_height = self.line_count as f32 * self.line_height;
        let content_height = if self.line_count > self.line_capacity {
            let unused_viewport_height =
                self.bounds.size.height - self.line_capacity as f32 * self.line_height;
            rendered_height + unused_viewport_height
        } else {
            rendered_height.min(self.bounds.size.height)
        };
        zeta_ui::ScrollMetrics::new(
            self.bounds.size,
            Size::new(self.bounds.size.width, content_height),
        )
    }
}

#[cfg(test)]
#[path = "terminal_output_scroll_view_tests.rs"]
mod tests;
