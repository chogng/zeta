use super::layout::horizontal_margin;
use super::theme::ACCENT;
use super::theme::MUTED;
use crate::app::App;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

pub(super) fn draw(frame: &mut Frame<'_>, area: Rect, app: &App) {
    let area = horizontal_margin(area, 2);
    let border_color = if app.accepts_input() { ACCENT } else { MUTED };
    let composer = Paragraph::new(Line::from(vec![
        Span::styled(
            "› ",
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(app.input()),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color)),
    );
    frame.render_widget(composer, area);

    if app.accepts_input() {
        let input_width = app
            .input_cursor_width()
            .min(area.width.saturating_sub(5) as usize) as u16;
        frame.set_cursor_position((area.x + 3 + input_width, area.y + 1));
    }
}
