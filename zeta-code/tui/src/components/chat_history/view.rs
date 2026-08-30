use super::ChatHistoryScroll;
use super::CommandStatus;
use super::Message;
use super::MessageRole;
use crate::components::welcome;
use crate::components::welcome::WelcomeModel;
use crate::render::RenderContext;
use crate::render::Renderable;
use crate::render::horizontal_margin;
use crate::render::prefix_lines;
use crate::render::styled_text_lines;
use crate::render::wrapped_height;
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

pub(crate) struct ChatHistoryView<'a> {
    pub(crate) messages: &'a [Message],
    pub(crate) scroll: &'a ChatHistoryScroll,
    pub(crate) welcome: &'a WelcomeModel,
    pub(crate) presentation_highlight: Color,
}

impl Renderable for ChatHistoryView<'_> {
    fn desired_height(&self, width: u16, context: RenderContext<'_>) -> u16 {
        if self.messages.is_empty() {
            return welcome::desired_height(width);
        }
        let content_width = horizontal_margin(Rect::new(0, 0, width, u16::MAX), 2).width;
        self.messages
            .iter()
            .map(|message| message_height(message, content_width, context))
            .sum::<usize>()
            .min(u16::MAX as usize) as u16
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>) {
        let content_area = horizontal_margin(area, 2);
        if self.messages.is_empty() {
            welcome::draw(
                frame,
                area,
                self.welcome,
                self.presentation_highlight,
                context,
            );
            return;
        }

        let history_height = content_area.height as usize;
        let lines = message_lines(self.messages, context);
        let history_rows = wrapped_height(&lines, content_area.width);
        let history = Paragraph::new(lines).wrap(Wrap { trim: false });
        let bottom_offset = history_rows.saturating_sub(history_height);
        frame.render_widget(
            history.scroll((self.scroll.paragraph_offset(bottom_offset), 0)),
            content_area,
        );
    }
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
    context: RenderContext<'_>,
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
    let total_rows = messages
        .iter()
        .map(|message| message_height(message, content_area.width, context))
        .sum::<usize>();
    let bottom_offset = total_rows.saturating_sub(content_area.height as usize);
    let visible_offset = usize::from(scroll.paragraph_offset(bottom_offset));
    let target_row = visible_offset.saturating_add(usize::from(row - content_area.y));
    let mut start = 0usize;
    for message in messages {
        let rows = message_height(message, content_area.width, context);
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

fn message_lines<'a>(messages: &'a [Message], context: RenderContext<'_>) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    for message in messages {
        if message.role == MessageRole::Command {
            let (color, status_marker) = match message.command_status {
                Some(CommandStatus::Running) => (context.accent(), "◉"),
                Some(CommandStatus::Succeeded) => (context.success(), "●"),
                Some(CommandStatus::Failed) => (context.danger(), "×"),
                None => (context.muted(), "●"),
            };
            let marker = expansion_marker(message).unwrap_or(status_marker);
            let command_lines = styled_text_lines(&message.text, selected_style(message, context));
            lines.extend(prefix_lines(
                command_lines,
                Span::styled(
                    format!("{marker}  "),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw("   "),
            ));
            if let Some(detail) = &message.detail {
                push_detail_lines(&mut lines, message.role, detail, context);
            }
            push_details_affordance(&mut lines, message, context);
            lines.push(Line::default());
            continue;
        }

        let (role_marker, color) = match message.role {
            MessageRole::User => ("›", context.accent()),
            MessageRole::Agent => ("◆", context.success()),
            MessageRole::Reasoning => ("◇", context.muted()),
            MessageRole::Plan => ("≡", context.accent()),
            MessageRole::Notice => ("•", context.warning()),
            MessageRole::Error => ("×", context.danger()),
            MessageRole::Command => unreachable!("command messages render as a grouped surface"),
        };
        let marker = expansion_marker(message).unwrap_or(role_marker);
        lines.extend(prefix_lines(
            styled_text_lines(&message.text, selected_style(message, context)),
            Span::styled(
                format!("{marker}  "),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
        ));
        if let Some(detail) = &message.detail {
            push_detail_lines(&mut lines, message.role, detail, context);
        }
        push_details_affordance(&mut lines, message, context);
        lines.push(Line::default());
    }
    lines
}

fn expansion_marker(message: &Message) -> Option<&'static str> {
    message
        .can_expand
        .then_some(if message.expanded { "▾" } else { "▸" })
}

fn selected_style(message: &Message, context: RenderContext<'_>) -> Style {
    if message.selected {
        Style::default()
            .fg(context.accent())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    }
}

fn push_details_affordance<'a>(
    lines: &mut Vec<Line<'a>>,
    message: &Message,
    context: RenderContext<'_>,
) {
    if message.expanded && message.has_details {
        lines.push(Line::from(Span::styled(
            "   view full",
            Style::default()
                .fg(context.accent())
                .add_modifier(Modifier::BOLD),
        )));
    }
}

fn push_detail_lines<'a>(
    lines: &mut Vec<Line<'a>>,
    role: MessageRole,
    detail: &'a str,
    context: RenderContext<'_>,
) {
    if role != MessageRole::Command {
        lines.extend(prefix_lines(
            styled_text_lines(detail, Style::default().fg(context.muted())),
            Span::styled("└─ ", Style::default().fg(context.muted())),
            Span::raw("   "),
        ));
        return;
    }

    let mut output = ansi_text(detail).lines;
    if output.is_empty() {
        output.push(Line::default());
    }
    for line in &mut output {
        for span in &mut line.spans {
            if span.style.fg.is_none() {
                span.style.fg = Some(context.muted());
            }
        }
    }
    lines.extend(prefix_lines(
        output,
        Span::styled("└─ ", Style::default().fg(context.muted())),
        Span::raw("   "),
    ));
}

fn message_height(message: &Message, width: u16, context: RenderContext<'_>) -> usize {
    wrapped_height(
        &message_lines(std::slice::from_ref(message), context),
        width,
    )
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
