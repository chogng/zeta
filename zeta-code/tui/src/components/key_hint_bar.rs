use crate::render::RenderContext;
use crate::render::horizontal_margin;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

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

pub(crate) fn draw_right(
    frame: &mut Frame<'_>,
    area: Rect,
    hints: &str,
    context: RenderContext<'_>,
) {
    let content = horizontal_margin(area, 2);
    let width = hints.width().min(usize::from(content.width)) as u16;
    let hint_area = Rect {
        x: content.right().saturating_sub(width),
        width,
        ..content
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            hints,
            Style::default()
                .fg(context.muted())
                .add_modifier(Modifier::ITALIC),
        ))),
        hint_area,
    );
}

#[cfg(test)]
#[path = "key_hint_bar_tests.rs"]
mod tests;
