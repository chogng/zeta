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
use ratatui::widgets::BorderType;
use ratatui::widgets::Borders;
use ratatui::widgets::Paragraph;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChatInputCursor {
    Hidden,
    Visible,
}

#[derive(Clone, Copy)]
pub(crate) enum ChatInputChrome {
    Rules,
    Box,
}

impl ChatInputChrome {
    pub(crate) fn inset(self) -> u16 {
        match self {
            Self::Rules => 0,
            Self::Box => 2,
        }
    }
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
    chrome: ChatInputChrome,
    context: RenderContext<'_>,
) {
    let wrapped = wrap_input(
        input,
        cursor_line,
        cursor_width,
        area.width.saturating_sub(chrome.inset()),
    );
    let lines = wrapped
        .lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let prompt = match chrome {
                ChatInputChrome::Rules if index == 0 => prompt,
                ChatInputChrome::Rules => "  ",
                ChatInputChrome::Box => "",
            };
            Line::from(vec![
                Span::styled(
                    prompt,
                    Style::default()
                        .fg(context.foreground())
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
                .borders(match chrome {
                    ChatInputChrome::Rules => Borders::TOP | Borders::BOTTOM,
                    ChatInputChrome::Box => Borders::ALL,
                })
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(context.chat_input_chrome())),
        );
    let border_area = match chrome {
        ChatInputChrome::Rules => area,
        ChatInputChrome::Box => content_area(area),
    };
    frame.render_widget(chat_input, border_area);
    if matches!(chrome, ChatInputChrome::Box) && scroll_row == 0 && visible_rows > 0 {
        frame.render_widget(
            Paragraph::new(prompt).style(
                Style::default()
                    .fg(context.foreground())
                    .add_modifier(Modifier::BOLD),
            ),
            Rect::new(area.x, area.y.saturating_add(1), 2.min(area.width), 1),
        );
    }

    if cursor == ChatInputCursor::Visible {
        let inset = chrome.inset() / 2;
        let content = content_area(Rect::new(
            area.x + inset.min(area.width),
            area.y,
            area.width.saturating_sub(chrome.inset()),
            area.height,
        ));
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
