use super::ChatHistoryScroll;
use super::CommandStatus;
use super::Message;
use super::MessageRole;
use super::row::estimated_wrapped_rows;
use crate::components::welcome;
use crate::components::welcome::WelcomeModel;
use crate::ui::accent;
use crate::ui::danger;
use crate::ui::horizontal_margin;
use crate::ui::muted;
use crate::ui::success;
use crate::ui::warning;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;
use zeta_ansi_escape::ansi_text;

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    messages: &[Message],
    scroll: &ChatHistoryScroll,
    welcome: &WelcomeModel,
    presentation_highlight: Color,
) {
    let content_area = horizontal_margin(area, 2);
    if messages.is_empty() {
        welcome::draw(frame, area, welcome, presentation_highlight);
        return;
    }

    let history_width = content_area.width as usize;
    let history_height = content_area.height as usize;
    let history_rows = messages
        .iter()
        .map(|message| message_rows(message, history_width))
        .sum::<usize>();
    let lines = message_lines(messages);
    let history = Paragraph::new(lines).wrap(Wrap { trim: false });
    let bottom_offset = history_rows.saturating_sub(history_height);
    frame.render_widget(
        history.scroll((scroll.paragraph_offset(bottom_offset), 0)),
        content_area,
    );
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ChatHistoryPointerTarget {
    Toggle(String),
    Details(String),
}

pub(crate) fn pointer_target_at(
    area: Rect,
    messages: &[Message],
    scroll: &ChatHistoryScroll,
    column: u16,
    row: u16,
) -> Option<ChatHistoryPointerTarget> {
    let content_area = horizontal_margin(area, 2);
    if column < content_area.x
        || column >= content_area.right()
        || row < content_area.y
        || row >= content_area.bottom()
    {
        return None;
    }
    let width = content_area.width as usize;
    let total_rows = messages
        .iter()
        .map(|message| message_rows(message, width))
        .sum::<usize>();
    let bottom_offset = total_rows.saturating_sub(content_area.height as usize);
    let visible_offset = usize::from(scroll.paragraph_offset(bottom_offset));
    let target_row = visible_offset.saturating_add(usize::from(row - content_area.y));
    let mut start = 0usize;
    for message in messages {
        let rows = message_rows(message, width);
        let Some(cell_id) = message.cell_id.as_ref() else {
            start = start.saturating_add(rows);
            continue;
        };
        if message.can_expand && target_row == start && column < content_area.x.saturating_add(2) {
            return Some(ChatHistoryPointerTarget::Toggle(cell_id.clone()));
        }
        if message.expanded
            && message.has_details
            && target_row == start.saturating_add(rows).saturating_sub(2)
        {
            return Some(ChatHistoryPointerTarget::Details(cell_id.clone()));
        }
        start = start.saturating_add(rows);
    }
    None
}

fn message_lines(messages: &[Message]) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    for message in messages {
        if message.role == MessageRole::Command {
            let (color, status_marker) = match message.command_status {
                Some(CommandStatus::Running) => (accent(), "◉"),
                Some(CommandStatus::Succeeded) => (success(), "●"),
                Some(CommandStatus::Failed) => (danger(), "×"),
                None => (muted(), "●"),
            };
            let marker = expansion_marker(message).unwrap_or(status_marker);
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{marker}  "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::styled(&message.text, selected_style(message)),
            ]));
            if let Some(detail) = &message.detail {
                push_detail_lines(&mut lines, message.role, detail);
            }
            push_details_affordance(&mut lines, message);
            lines.push(Line::default());
            continue;
        }

        let (role_marker, color) = match message.role {
            MessageRole::User => ("›", accent()),
            MessageRole::Agent => ("◆", success()),
            MessageRole::Reasoning => ("◇", muted()),
            MessageRole::Plan => ("≡", accent()),
            MessageRole::Notice => ("•", warning()),
            MessageRole::Error => ("×", danger()),
            MessageRole::Command => unreachable!("command messages render as a grouped surface"),
        };
        let marker = expansion_marker(message).unwrap_or(role_marker);
        lines.push(Line::from(vec![
            Span::styled(
                format!("{marker}  "),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::styled(&message.text, selected_style(message)),
        ]));
        if let Some(detail) = &message.detail {
            push_detail_lines(&mut lines, message.role, detail);
        }
        push_details_affordance(&mut lines, message);
        lines.push(Line::default());
    }
    lines
}

fn expansion_marker(message: &Message) -> Option<&'static str> {
    message
        .can_expand
        .then_some(if message.expanded { "▾" } else { "▸" })
}

fn selected_style(message: &Message) -> Style {
    if message.selected {
        Style::default().fg(accent()).add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

fn push_details_affordance<'a>(lines: &mut Vec<Line<'a>>, message: &Message) {
    if message.expanded && message.has_details {
        lines.push(Line::from(Span::styled(
            "   view full",
            Style::default().fg(accent()).add_modifier(Modifier::BOLD),
        )));
    }
}

fn push_detail_lines<'a>(lines: &mut Vec<Line<'a>>, role: MessageRole, detail: &'a str) {
    if role != MessageRole::Command {
        lines.push(Line::from(vec![
            Span::styled("└─ ", Style::default().fg(muted())),
            Span::styled(detail, Style::default().fg(muted())),
        ]));
        return;
    }

    let mut output = ansi_text(detail).lines;
    if output.is_empty() {
        output.push(Line::default());
    }
    for line in &mut output {
        for span in &mut line.spans {
            if span.style.fg.is_none() {
                span.style.fg = Some(muted());
            }
        }
    }
    output[0]
        .spans
        .insert(0, Span::styled("└─ ", Style::default().fg(muted())));
    lines.extend(output);
}

fn message_rows(message: &Message, available_width: usize) -> usize {
    let detail_rows = message
        .detail
        .as_deref()
        .map(|detail| estimated_wrapped_rows(3, detail, available_width))
        .unwrap_or_default();
    let details_rows = usize::from(message.expanded && message.has_details);
    estimated_wrapped_rows(3, &message.text, available_width)
        .saturating_add(detail_rows)
        .saturating_add(details_rows)
        .saturating_add(1)
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
