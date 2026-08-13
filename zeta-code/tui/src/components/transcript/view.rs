use super::CommandStatus;
use super::Message;
use super::MessageRole;
use super::TranscriptScroll;
use super::row::estimated_wrapped_rows;
use crate::components::welcome;
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
    scroll: &TranscriptScroll,
    presentation_highlight: Color,
) {
    let content_area = horizontal_margin(area, 2);
    if messages.is_empty() {
        welcome::draw(frame, area, presentation_highlight);
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

fn message_lines(messages: &[Message]) -> Vec<Line<'_>> {
    let mut lines = Vec::new();
    for message in messages {
        if message.role == MessageRole::Command {
            let (color, marker) = match message.command_status {
                Some(CommandStatus::Running) => (accent(), "◉"),
                Some(CommandStatus::Succeeded) => (success(), "●"),
                None => (muted(), "●"),
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{marker}  "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(&message.text),
            ]));
            if let Some(detail) = &message.detail {
                push_detail_lines(&mut lines, message.role, detail);
            }
            lines.push(Line::default());
            continue;
        }

        let (marker, color) = match message.role {
            MessageRole::User => ("›", accent()),
            MessageRole::Agent => ("◆", success()),
            MessageRole::Reasoning => ("◇", muted()),
            MessageRole::Plan => ("≡", accent()),
            MessageRole::Tool => ("⚙", warning()),
            MessageRole::ToolError => ("×", danger()),
            MessageRole::Notice => ("•", warning()),
            MessageRole::Error => ("×", danger()),
            MessageRole::Command => unreachable!("command messages render as a grouped surface"),
        };
        lines.push(Line::from(vec![
            Span::styled(
                format!("{marker}  "),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::raw(&message.text),
        ]));
        if let Some(detail) = &message.detail {
            push_detail_lines(&mut lines, message.role, detail);
        }
        lines.push(Line::default());
    }
    lines
}

fn push_detail_lines<'a>(lines: &mut Vec<Line<'a>>, role: MessageRole, detail: &'a str) {
    if !matches!(role, MessageRole::Tool | MessageRole::ToolError) {
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
    estimated_wrapped_rows(3, &message.text, available_width)
        .saturating_add(detail_rows)
        .saturating_add(1)
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
