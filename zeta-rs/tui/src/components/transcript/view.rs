use super::Message;
use super::MessageRole;
use super::row::estimated_wrapped_rows;
use crate::components::welcome;
use crate::ui::ACCENT;
use crate::ui::DANGER;
use crate::ui::SUCCESS;
use crate::ui::WARNING;
use crate::ui::horizontal_margin;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

pub(crate) fn draw(frame: &mut Frame<'_>, area: Rect, messages: &[Message]) {
    let content_area = horizontal_margin(area, 2);
    if messages.is_empty() {
        welcome::draw(frame, area);
        return;
    }

    let history_width = content_area.width as usize;
    let history_height = content_area.height as usize;
    let history_rows = messages
        .iter()
        .map(|message| estimated_wrapped_rows(3, &message.text, history_width).saturating_add(1))
        .sum::<usize>();
    let lines = message_lines(messages);
    let history = Paragraph::new(lines).wrap(Wrap { trim: false });
    let scroll = history_rows
        .saturating_sub(history_height)
        .min(u16::MAX as usize) as u16;
    frame.render_widget(history.scroll((scroll, 0)), content_area);
}

fn message_lines(messages: &[Message]) -> Vec<Line<'_>> {
    messages
        .iter()
        .flat_map(|message| {
            let (marker, color) = match message.role {
                MessageRole::User => ("›", ACCENT),
                MessageRole::Agent => ("◆", SUCCESS),
                MessageRole::Notice => ("•", WARNING),
                MessageRole::Error => ("×", DANGER),
            };
            [
                Line::from(vec![
                    Span::styled(
                        format!("{marker}  "),
                        Style::default().fg(color).add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(&message.text),
                ]),
                Line::default(),
            ]
        })
        .collect()
}
