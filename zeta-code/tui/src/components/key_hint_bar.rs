use crate::render::RenderContext;
use crate::render::horizontal_margin;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, hints: &str, context: RenderContext<'_>) {
    let content = horizontal_margin(area, 2);
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hints,
            Style::default()
                .fg(context.muted())
                .add_modifier(Modifier::ITALIC),
        ))),
        content,
    );
}

#[cfg(test)]
#[path = "key_hint_bar_tests.rs"]
mod tests;
