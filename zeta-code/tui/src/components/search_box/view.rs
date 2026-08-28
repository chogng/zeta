use super::SearchBoxState;
use crate::ui::muted;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Padding;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

const SEARCH_BOX_LEFT_PADDING: u16 = 1;

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    search: &SearchBoxState,
    active_color: Color,
) {
    let rendered_query = search
        .masked()
        .then(|| "•".repeat(search.query().chars().count()));
    let text = if search.query().is_empty() {
        Span::styled(search.placeholder(), Style::default().fg(muted()))
    } else {
        Span::raw(rendered_query.as_deref().unwrap_or(search.query()))
    };
    let border_color = if search.input_active() {
        active_color
    } else {
        muted()
    };
    frame.render_widget(
        Paragraph::new(Line::from(text)).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(border_color))
                .padding(Padding::left(SEARCH_BOX_LEFT_PADDING)),
        ),
        area,
    );
    if search.input_active() && area.width > 2 && area.height > 2 {
        let cursor_width = rendered_query
            .as_deref()
            .unwrap_or(search.query())
            .width()
            .min(usize::from(
                area.width
                    .saturating_sub(2)
                    .saturating_sub(SEARCH_BOX_LEFT_PADDING)
                    .saturating_sub(1),
            )) as u16;
        frame.set_cursor_position((
            area.x + 1 + SEARCH_BOX_LEFT_PADDING + cursor_width,
            area.y + 1,
        ));
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
