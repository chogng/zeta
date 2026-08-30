mod layout;
mod theme;

pub(crate) use layout::bottom_anchored_area;
pub(crate) use layout::horizontal_margin;
use ratatui::Frame;
use ratatui::layout::Rect;
pub(crate) use theme::RenderContext;
pub(crate) use theme::RenderTheme;
#[cfg(test)]
pub(crate) use theme::test_context;

/// A terminal surface that can measure and draw itself from immutable presentation state.
pub(crate) trait Renderable {
    fn desired_height(&self, width: u16) -> u16;

    fn render(&self, frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>);
}
