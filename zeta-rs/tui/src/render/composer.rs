use super::theme::COMPOSER_CHROME;
use crate::app::App;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

pub(super) fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let composer = Paragraph::new(Line::from(vec![
        Span::styled(
            "❯ ",
            Style::default()
                .fg(COMPOSER_CHROME)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(app.input()),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(Style::default().fg(COMPOSER_CHROME)),
    );
    frame.render_widget(composer, area);

    if app.accepts_input() {
        let input_width = app
            .input_cursor_width()
            .min(area.width.saturating_sub(3) as usize) as u16;
        frame.set_cursor_position((area.x + 2 + input_width, area.y + 1));
    }
}
