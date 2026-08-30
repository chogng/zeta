use super::ChatHistoryRenderCache;
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
use crate::render::push_owned_lines;
use crate::render::styled_text_lines;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use zeta_ansi_escape::ansi_text;

pub(crate) struct ChatHistoryView<'a> {
    pub(crate) messages: &'a [Message],
    pub(crate) scroll: &'a ChatHistoryScroll,
    pub(crate) render_cache: &'a ChatHistoryRenderCache,
    pub(crate) welcome: &'a WelcomeModel,
    pub(crate) presentation_highlight: Color,
}

impl Renderable for ChatHistoryView<'_> {
    fn desired_height(&self, width: u16, context: RenderContext<'_>) -> u16 {
        if self.messages.is_empty() {
            return welcome::desired_height(width);
        }
        let content_width = horizontal_margin(Rect::new(0, 0, width, u16::MAX), 2).width;
        measured_heights(self.messages, self.render_cache, content_width, context)
            .into_iter()
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

        let heights = measured_heights(
            self.messages,
            self.render_cache,
            content_area.width,
            context,
        );
        render_cells(
            frame,
            content_area,
            self.scroll,
            self.messages,
            &heights,
            self.render_cache,
            context,
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
    render_cache: &ChatHistoryRenderCache,
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
    let heights = measured_heights(messages, render_cache, content_area.width, context);
    let total_rows = heights.iter().sum::<usize>();
    let bottom_offset = total_rows.saturating_sub(content_area.height as usize);
    let visible_offset = scroll.paragraph_offset(bottom_offset);
    let target_row = visible_offset.saturating_add(usize::from(row - content_area.y));
    let mut start = 0usize;
    for (message, rows) in messages.iter().zip(heights) {
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

#[cfg(test)]
fn message_lines<'a>(messages: &'a [Message], context: RenderContext<'_>) -> Vec<Line<'a>> {
    message_lines_with_code(messages, context, None, true)
}

fn message_lines_with_code<'a>(
    messages: &'a [Message],
    context: RenderContext<'_>,
    cache: Option<&ChatHistoryRenderCache>,
    highlight: bool,
) -> Vec<Line<'a>> {
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
            let command_lines = styled_body_lines(message, context, cache, highlight);
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
            styled_body_lines(message, context, cache, highlight),
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

fn styled_body_lines<'a>(
    message: &'a Message,
    context: RenderContext<'_>,
    cache: Option<&ChatHistoryRenderCache>,
    highlight: bool,
) -> Vec<Line<'a>> {
    if !message
        .text
        .lines()
        .any(|line| line.trim_start().starts_with("```"))
    {
        return styled_text_lines(&message.text, selected_style(message, context));
    }

    let mut output = Vec::new();
    let mut plain = String::new();
    let mut code = String::new();
    let mut language = String::new();
    let mut in_code = false;
    let mut block_index = 0;
    for source_line in message.text.split_inclusive('\n') {
        let visible = source_line.strip_suffix('\n').unwrap_or(source_line);
        let visible = visible.strip_suffix('\r').unwrap_or(visible);
        if !in_code {
            if let Some(opening) = visible.trim_start().strip_prefix("```") {
                push_plain_block(&mut output, &mut plain, selected_style(message, context));
                language = opening.trim().to_owned();
                in_code = true;
            } else {
                plain.push_str(source_line);
            }
            continue;
        }

        if visible.trim() == "```" {
            push_code_block(
                &mut output,
                message,
                block_index,
                &language,
                &code,
                context,
                cache,
                highlight,
            );
            code.clear();
            block_index += 1;
            in_code = false;
        } else {
            code.push_str(source_line);
        }
    }
    if in_code {
        push_code_block(
            &mut output,
            message,
            block_index,
            &language,
            &code,
            context,
            cache,
            highlight,
        );
    } else {
        push_plain_block(&mut output, &mut plain, selected_style(message, context));
    }
    if output.is_empty() {
        output.push(Line::default());
    }
    output
}

fn push_plain_block(output: &mut Vec<Line<'static>>, text: &mut String, style: Style) {
    if text.is_empty() {
        return;
    }
    let lines = styled_text_lines(text.trim_end_matches('\n'), style);
    push_owned_lines(&lines, output);
    text.clear();
}

#[allow(clippy::too_many_arguments)]
fn push_code_block(
    output: &mut Vec<Line<'static>>,
    message: &Message,
    block_index: usize,
    language: &str,
    code: &str,
    context: RenderContext<'_>,
    cache: Option<&ChatHistoryRenderCache>,
    highlight: bool,
) {
    if !highlight {
        let lines = styled_text_lines(
            code.strip_suffix('\n').unwrap_or(code),
            Style::default().fg(context.foreground()),
        );
        push_owned_lines(&lines, output);
        return;
    }
    let lines = cache.map_or_else(
        || crate::render::highlight_code(code, language, context.into()),
        |cache| cache.highlight_code_block(message, block_index, language, code, context),
    );
    output.extend(lines);
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

fn measured_heights(
    messages: &[Message],
    cache: &ChatHistoryRenderCache,
    width: u16,
    context: RenderContext<'_>,
) -> Vec<usize> {
    cache.retain_messages(messages);
    messages
        .iter()
        .map(|message| {
            cache.measure(message, width, context, || {
                message_lines_with_code(std::slice::from_ref(message), context, None, false)
            })
        })
        .collect()
}

fn render_cells(
    frame: &mut Frame<'_>,
    area: Rect,
    scroll: &ChatHistoryScroll,
    messages: &[Message],
    heights: &[usize],
    cache: &ChatHistoryRenderCache,
    context: RenderContext<'_>,
) {
    let total_rows = heights.iter().sum::<usize>();
    let bottom_offset = total_rows.saturating_sub(usize::from(area.height));
    let viewport_start = scroll.paragraph_offset(bottom_offset);
    let viewport_end = viewport_start.saturating_add(usize::from(area.height));
    let mut cell_start = 0usize;
    for (message, height) in messages.iter().zip(heights) {
        let cell_end = cell_start.saturating_add(*height);
        let visible_start = cell_start.max(viewport_start);
        let visible_end = cell_end.min(viewport_end);
        if visible_start < visible_end {
            let target_y = area
                .y
                .saturating_add((visible_start - viewport_start) as u16);
            let target_height = (visible_end - visible_start) as u16;
            let source_row = visible_start - cell_start;
            let cell = cache.prepare(message, area.width, context, || {
                message_lines_with_code(std::slice::from_ref(message), context, Some(cache), true)
            });
            cell.render(
                frame.buffer_mut(),
                Rect::new(area.x, target_y, area.width, target_height),
                source_row,
            );
        }
        cell_start = cell_end;
        if cell_start >= viewport_end {
            break;
        }
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
