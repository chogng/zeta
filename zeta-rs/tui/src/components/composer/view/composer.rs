use crate::ui::composer_chrome;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Block;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ComposerCursor {
    Hidden,
    Visible,
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    input: &str,
    cursor_width: usize,
    cursor: ComposerCursor,
) {
    let composer = Paragraph::new(Line::from(vec![
        Span::styled(
            "❯ ",
            Style::default()
                .fg(composer_chrome())
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(input),
    ]))
    .block(
        Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(Style::default().fg(composer_chrome())),
    );
    frame.render_widget(composer, area);

    if cursor == ComposerCursor::Visible {
        let input_width = cursor_width.min(area.width.saturating_sub(3) as usize) as u16;
        frame.set_cursor_position((area.x + 2 + input_width, area.y + 1));
    }
}
