mod highlight;
mod highlight_streaming;
mod layout;
mod text;
mod theme;

pub(crate) use highlight::SyntaxPalette;
pub(crate) use highlight::code_within_limits;
pub(crate) use highlight::highlight_code;
pub(crate) use highlight_streaming::StreamingCodeHighlighter;
pub(crate) use layout::Insets;
pub(crate) use layout::RectExt;
pub(crate) use layout::bottom_anchored_area;
pub(crate) use layout::horizontal_margin;
use ratatui::Frame;
use ratatui::layout::Rect;
pub(crate) use text::line_to_borrowed;
pub(crate) use text::prefix_lines;
pub(crate) use text::push_owned_lines;
pub(crate) use text::styled_text_lines;
pub(crate) use text::wrapped_height;
pub(crate) use theme::RenderContext;
pub(crate) use theme::RenderTheme;
#[cfg(test)]
pub(crate) use theme::test_context;

/// A terminal surface that can measure and draw itself from immutable presentation state.
pub(crate) trait Renderable {
    fn desired_height(&self, width: u16, context: RenderContext<'_>) -> u16;

    fn render(&self, frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>);
}
