use super::wrap::PROMPT_WIDTH;
use super::wrap::wrap_input;
use crate::ui::chat_input_chrome;
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
pub(crate) enum ChatInputCursor {
    Hidden,
    Visible,
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    input: &str,
    cursor_width: usize,
    cursor_line: usize,
    cursor: ChatInputCursor,
) {
    let wrapped = wrap_input(input, cursor_line, cursor_width, area.width);
    let lines = wrapped
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let prompt = if index == 0 { "❯ " } else { "  " };
            Line::from(vec![
                Span::styled(
                    prompt,
                    Style::default()
                        .fg(chat_input_chrome())
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(line),
            ])
        })
        .collect::<Vec<_>>();
    let visible_rows = area.height.saturating_sub(2) as usize;
    let scroll_row = wrapped
        .cursor_row
        .saturating_sub(visible_rows.saturating_sub(1));
    let chat_input = Paragraph::new(lines)
        .scroll((scroll_row.min(u16::MAX as usize) as u16, 0))
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_style(Style::default().fg(chat_input_chrome())),
        );
    frame.render_widget(chat_input, area);

    if cursor == ChatInputCursor::Visible {
        let input_width = wrapped
            .cursor_column
            .min(area.width.saturating_sub(PROMPT_WIDTH as u16 + 1) as usize)
            as u16;
        let visible_cursor_line = wrapped.cursor_row.saturating_sub(scroll_row);
        let cursor_y = area
            .y
            .saturating_add(1)
            .saturating_add(visible_cursor_line.min(u16::MAX as usize) as u16)
            .min(area.y.saturating_add(area.height.saturating_sub(2)));
        frame.set_cursor_position((area.x + PROMPT_WIDTH as u16 + input_width, cursor_y));
    }
}
