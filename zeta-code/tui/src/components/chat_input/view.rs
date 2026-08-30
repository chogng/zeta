use super::wrap::PROMPT_WIDTH;
use super::wrap::wrap_input;
use crate::render::RenderContext;
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

pub(crate) fn content_area(area: Rect) -> Rect {
    let offset = (PROMPT_WIDTH as u16).min(area.width);
    Rect {
        x: area.x.saturating_add(offset),
        width: area.width.saturating_sub(offset),
        ..area
    }
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    input: &str,
    cursor_width: usize,
    cursor_line: usize,
    prompt: &str,
    cursor: ChatInputCursor,
    context: RenderContext<'_>,
) {
    let wrapped = wrap_input(input, cursor_line, cursor_width, area.width);
    let lines = wrapped
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let prompt = if index == 0 { prompt } else { "  " };
            Line::from(vec![
                Span::styled(
                    prompt,
                    Style::default()
                        .fg(context.chat_input_chrome())
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
                .border_style(Style::default().fg(context.chat_input_chrome())),
        );
    frame.render_widget(chat_input, area);

    if cursor == ChatInputCursor::Visible {
        let content = content_area(area);
        let input_width = wrapped
            .cursor_column
            .min(content.width.saturating_sub(1) as usize) as u16;
        let visible_cursor_line = wrapped.cursor_row.saturating_sub(scroll_row);
        let cursor_y = area
            .y
            .saturating_add(1)
            .saturating_add(visible_cursor_line.min(u16::MAX as usize) as u16)
            .min(area.y.saturating_add(area.height.saturating_sub(2)));
        frame.set_cursor_position((content.x.saturating_add(input_width), cursor_y));
    }
}
