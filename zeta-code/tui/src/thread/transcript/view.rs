use super::ChatHistoryRenderCache;
use super::ChatHistoryScroll;
use super::CommandStatus;
use super::ExecutionKind;
use super::Message;
use super::MessageRole;
use super::TranscriptScrollAnchor;
use super::TranscriptScrollDirection;
use super::TranscriptScrollTarget;
use crate::render::InteractionState;
use crate::render::InteractionTarget;
use crate::render::RenderContext;
use crate::render::Renderable;
use crate::render::action_style;
use crate::render::interaction_style;
use crate::render::prefix_lines;
use crate::render::push_owned_lines;
use crate::render::styled_text_lines;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use unicode_width::UnicodeWidthStr;
use zeta_ansi_escape::ansi_text;

const JUMP_TO_BOTTOM_LABEL: &str = "Jump to bottom (click) ↓";

pub(crate) struct ChatHistoryView<'a> {
    pub(crate) header: Option<&'a Buffer>,
    pub(crate) messages: &'a [Message],
    pub(crate) scroll: &'a ChatHistoryScroll,
    pub(crate) render_cache: &'a ChatHistoryRenderCache,
    pub(crate) pointer: ChatHistoryPointerState<'a>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct ChatHistoryPointerState<'a> {
    pub(crate) hovered_jump_to_bottom: bool,
    pub(crate) hovered_toggle: Option<&'a str>,
    pub(crate) hovered_details: Option<&'a str>,
    pub(crate) pressed_jump_to_bottom: bool,
    pub(crate) pressed_toggle: Option<&'a str>,
    pub(crate) pressed_details: Option<&'a str>,
}

impl Renderable for ChatHistoryView<'_> {
    fn desired_height(&self, width: u16, context: RenderContext<'_>) -> u16 {
        let message_rows = measured_heights(self.messages, self.render_cache, width, context)
            .into_iter()
            .sum::<usize>();
        header_rows(self.header)
            .saturating_add(message_rows)
            .min(u16::MAX as usize) as u16
    }

    fn render(&self, frame: &mut Frame<'_>, area: Rect, context: RenderContext<'_>) {
        let heights = measured_heights(self.messages, self.render_cache, area.width, context);
        let header_rows = header_rows(self.header);
        let (content_area, jump_area) = scroll_areas(area, header_rows, &heights, self.scroll);
        let total_rows = header_rows.saturating_add(heights.iter().sum::<usize>());
        let bottom_offset = total_rows.saturating_sub(usize::from(content_area.height));
        let viewport_start = viewport_offset(
            self.messages,
            header_rows,
            &heights,
            self.scroll,
            bottom_offset,
        );
        render_header(
            frame.buffer_mut(),
            content_area,
            self.header,
            viewport_start,
        );
        render_cells(
            frame,
            content_area,
            header_rows,
            viewport_start,
            self.messages,
            &heights,
            self.render_cache,
            self.pointer,
            context,
        );
        render_jump_to_bottom(frame, jump_area, self.pointer, context);
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ChatHistoryPointerTarget {
    JumpToBottom,
    Toggle(String),
    Details(String),
}

pub(crate) fn pointer_target_at(
    area: Rect,
    header_rows: usize,
    messages: &[Message],
    scroll: &ChatHistoryScroll,
    render_cache: &ChatHistoryRenderCache,
    context: RenderContext<'_>,
    column: u16,
    row: u16,
) -> Option<ChatHistoryPointerTarget> {
    if column < area.x || column >= area.right() || row < area.y || row >= area.bottom() {
        return None;
    }
    let heights = measured_heights(messages, render_cache, area.width, context);
    let (content_area, jump_area) = scroll_areas(area, header_rows, &heights, scroll);
    if jump_area.is_some_and(|button| {
        column >= button.x && column < button.right() && row >= button.y && row < button.bottom()
    }) {
        return Some(ChatHistoryPointerTarget::JumpToBottom);
    }
    if row >= content_area.bottom() {
        return None;
    }
    let total_rows = header_rows.saturating_add(heights.iter().sum::<usize>());
    let bottom_offset = total_rows.saturating_sub(usize::from(content_area.height));
    let visible_offset = viewport_offset(messages, header_rows, &heights, scroll, bottom_offset);
    let target_row = visible_offset.saturating_add(usize::from(row - content_area.y));
    let mut start = header_rows;
    for (message, rows) in messages.iter().zip(heights) {
        let Some(cell_id) = message.cell_id.as_ref() else {
            start = start.saturating_add(rows);
            continue;
        };
        if message.can_expand && target_row == start && column < area.x.saturating_add(2) {
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

pub(crate) fn scroll_target(
    area: Rect,
    header_rows: usize,
    messages: &[Message],
    scroll: &ChatHistoryScroll,
    render_cache: &ChatHistoryRenderCache,
    context: RenderContext<'_>,
    direction: TranscriptScrollDirection,
    rows: usize,
) -> Option<TranscriptScrollTarget> {
    let heights = measured_heights(messages, render_cache, area.width, context);
    let (content_area, _) = scroll_areas(area, header_rows, &heights, scroll);
    let bottom_offset = header_rows
        .saturating_add(heights.iter().sum::<usize>())
        .saturating_sub(usize::from(content_area.height));
    let current = viewport_offset(messages, header_rows, &heights, scroll, bottom_offset);
    let target = match direction {
        TranscriptScrollDirection::Up => current.saturating_sub(rows),
        TranscriptScrollDirection::Down => current.saturating_add(rows),
    };
    if target >= bottom_offset {
        return scroll
            .anchor()
            .is_some()
            .then_some(TranscriptScrollTarget::FollowLatest);
    }
    if target == current {
        return None;
    }
    anchor_at(messages, header_rows, &heights, target).map(TranscriptScrollTarget::Anchor)
}

pub(crate) fn first_scroll_target(
    has_header: bool,
    messages: &[Message],
) -> Option<TranscriptScrollTarget> {
    if has_header {
        return Some(TranscriptScrollTarget::Anchor(
            TranscriptScrollAnchor::Header { line_offset: 0 },
        ));
    }
    messages.iter().find_map(|message| {
        message.cell_id.as_ref().map(|cell_id| {
            TranscriptScrollTarget::Anchor(TranscriptScrollAnchor::Cell {
                cell_id: cell_id.clone(),
                line_offset: 0,
            })
        })
    })
}

fn jump_to_bottom_area(
    area: Rect,
    header_rows: usize,
    heights: &[usize],
    scroll: &ChatHistoryScroll,
) -> Option<Rect> {
    if area.width == 0 || area.height == 0 {
        return None;
    }
    let total_rows = header_rows.saturating_add(heights.iter().sum::<usize>());
    let bottom_offset = total_rows.saturating_sub(usize::from(area.height));
    if bottom_offset == 0 || scroll.anchor().is_none() {
        return None;
    }
    let label_width = u16::try_from(JUMP_TO_BOTTOM_LABEL.width()).unwrap_or(u16::MAX);
    let width = label_width.min(area.width);
    let x = area.x.saturating_add((area.width - width) / 2);
    Some(Rect::new(x, area.bottom().saturating_sub(1), width, 1))
}

fn scroll_areas(
    area: Rect,
    header_rows: usize,
    heights: &[usize],
    scroll: &ChatHistoryScroll,
) -> (Rect, Option<Rect>) {
    let jump_area = jump_to_bottom_area(area, header_rows, heights, scroll);
    let content_area = if jump_area.is_some() {
        Rect::new(area.x, area.y, area.width, area.height.saturating_sub(1))
    } else {
        area
    };
    (content_area, jump_area)
}

fn viewport_offset(
    messages: &[Message],
    header_rows: usize,
    heights: &[usize],
    scroll: &ChatHistoryScroll,
    bottom_offset: usize,
) -> usize {
    let Some(anchor) = scroll.anchor() else {
        return bottom_offset;
    };
    if let TranscriptScrollAnchor::Header { line_offset } = anchor {
        return (*line_offset)
            .min(header_rows.saturating_sub(1))
            .min(bottom_offset);
    }
    let TranscriptScrollAnchor::Cell {
        cell_id,
        line_offset,
    } = anchor
    else {
        unreachable!();
    };
    let mut start = header_rows;
    for (message, height) in messages.iter().zip(heights) {
        if message.cell_id.as_deref() == Some(cell_id.as_str()) {
            return start
                .saturating_add((*line_offset).min(height.saturating_sub(1)))
                .min(bottom_offset);
        }
        start = start.saturating_add(*height);
    }
    bottom_offset
}

fn anchor_at(
    messages: &[Message],
    header_rows: usize,
    heights: &[usize],
    target: usize,
) -> Option<TranscriptScrollAnchor> {
    if target < header_rows {
        return Some(TranscriptScrollAnchor::Header {
            line_offset: target,
        });
    }
    let mut start = header_rows;
    for (index, (message, height)) in messages.iter().zip(heights).enumerate() {
        let end = start.saturating_add(*height);
        if target < end {
            let line_offset = target.saturating_sub(start);
            if line_offset == height.saturating_sub(1)
                && let Some(cell_id) = messages
                    .get(index.saturating_add(1))
                    .and_then(|message| message.cell_id.as_ref())
            {
                return Some(TranscriptScrollAnchor::Cell {
                    cell_id: cell_id.clone(),
                    line_offset: 0,
                });
            }
            return message
                .cell_id
                .as_ref()
                .map(|cell_id| TranscriptScrollAnchor::Cell {
                    cell_id: cell_id.clone(),
                    line_offset: line_offset.min(height.saturating_sub(2)),
                });
        }
        start = end;
    }
    None
}

fn render_jump_to_bottom(
    frame: &mut Frame<'_>,
    area: Option<Rect>,
    pointer: ChatHistoryPointerState<'_>,
    context: RenderContext<'_>,
) {
    let Some(area) = area else {
        return;
    };
    let style = if pointer.hovered_jump_to_bottom || pointer.pressed_jump_to_bottom {
        interaction_style(
            context,
            InteractionState {
                target: InteractionTarget::Rest,
                hovered: pointer.hovered_jump_to_bottom,
                pressed: pointer.pressed_jump_to_bottom,
                ..Default::default()
            },
        )
    } else {
        Style::default()
            .fg(context.foreground())
            .bg(context.transcript_jump_background())
    };
    frame.buffer_mut().set_stringn(
        area.x,
        area.y,
        JUMP_TO_BOTTOM_LABEL,
        usize::from(area.width),
        style,
    );
}

#[cfg(test)]
fn message_lines<'a>(messages: &'a [Message], context: RenderContext<'_>) -> Vec<Line<'a>> {
    message_lines_with_code(messages, context, None, true)
}

#[cfg(test)]
fn message_lines_with_code<'a>(
    messages: &'a [Message],
    context: RenderContext<'_>,
    cache: Option<&ChatHistoryRenderCache>,
    syntax_highlighting: bool,
) -> Vec<Line<'a>> {
    let mut lines = Vec::new();
    for message in messages {
        lines.extend(cell_lines_with_code(message, context, cache, syntax_highlighting).0);
    }
    lines
}

fn cell_lines_with_code<'a>(
    message: &'a Message,
    context: RenderContext<'_>,
    cache: Option<&ChatHistoryRenderCache>,
    syntax_highlighting: bool,
) -> (Vec<Line<'a>>, usize) {
    let mut lines = Vec::new();
    if message.role == MessageRole::Command {
        let (marker, color) = command_marker(message, context);
        let marker_style = Style::default().fg(color).add_modifier(Modifier::BOLD);
        let command_lines = styled_body_lines(message, context, cache, syntax_highlighting);
        let user_input_lines = if message.execution_kind == ExecutionKind::LocalCommand {
            command_lines.len()
        } else {
            0
        };
        lines.extend(prefix_lines(
            command_lines,
            Span::styled(format!("{marker} "), marker_style),
            Span::raw("  "),
        ));
        if let Some(detail) = &message.detail {
            push_detail_lines(&mut lines, message.role, detail, context);
        }
        push_details_affordance(&mut lines, message, context);
        lines.push(Line::default());
        return (lines, user_input_lines);
    }

    let (role_marker, color) = match message.role {
        MessageRole::User => (">", context.muted()),
        MessageRole::Agent | MessageRole::Reasoning | MessageRole::Plan => ("●", context.muted()),
        MessageRole::Notice => ("●", context.warning()),
        MessageRole::Error => ("●", context.danger()),
        MessageRole::Command => unreachable!("command messages render as a grouped surface"),
    };
    let marker_style = Style::default().fg(color).add_modifier(Modifier::BOLD);
    let body_lines = styled_body_lines(message, context, cache, syntax_highlighting);
    let user_input_lines = if message.role == MessageRole::User {
        body_lines.len()
    } else {
        0
    };
    lines.extend(prefix_lines(
        body_lines,
        Span::styled(format!("{role_marker} "), marker_style),
        Span::raw("  "),
    ));
    if let Some(detail) = &message.detail {
        push_detail_lines(&mut lines, message.role, detail, context);
    }
    push_details_affordance(&mut lines, message, context);
    lines.push(Line::default());
    (lines, user_input_lines)
}

fn styled_body_lines<'a>(
    message: &'a Message,
    context: RenderContext<'_>,
    cache: Option<&ChatHistoryRenderCache>,
    syntax_highlighting: bool,
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
                syntax_highlighting,
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
            syntax_highlighting,
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
    syntax_highlighting: bool,
) {
    if !syntax_highlighting {
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

fn command_marker(
    message: &Message,
    context: RenderContext<'_>,
) -> (&'static str, ratatui::style::Color) {
    if message.execution_kind == ExecutionKind::LocalCommand
        && message.command_status != Some(CommandStatus::Running)
    {
        return (">", context.muted());
    }
    ("●", execution_color(message, context))
}

fn execution_color(message: &Message, context: RenderContext<'_>) -> ratatui::style::Color {
    match message.command_status {
        Some(CommandStatus::Submitted | CommandStatus::Running) => context.warning(),
        Some(CommandStatus::Failed) => context.danger(),
        Some(CommandStatus::Succeeded) => match message.execution_kind {
            ExecutionKind::LocalCommand => context.muted(),
            ExecutionKind::Command => context.success(),
            ExecutionKind::Mutation => context.accent(),
            ExecutionKind::Neutral => context.muted(),
        },
        None => context.muted(),
    }
}

fn selected_style(message: &Message, context: RenderContext<'_>) -> Style {
    if message.selected {
        interaction_style(
            context,
            InteractionState {
                target: InteractionTarget::Rest,
                selected: true,
                hovered: false,
                pressed: false,
            },
        )
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
            action_style(context),
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
                cell_lines_with_code(message, context, None, false).0
            })
        })
        .collect()
}

fn header_rows(header: Option<&Buffer>) -> usize {
    header.map_or(0, |buffer| usize::from(buffer.area.height))
}

fn render_header(target: &mut Buffer, area: Rect, header: Option<&Buffer>, viewport_start: usize) {
    let Some(header) = header else {
        return;
    };
    let header_rows = usize::from(header.area.height);
    let viewport_end = viewport_start.saturating_add(usize::from(area.height));
    let visible_start = viewport_start.min(header_rows);
    let visible_end = viewport_end.min(header_rows);
    if visible_start >= visible_end {
        return;
    }
    let target_y = area.y;
    let target_height = u16::try_from(visible_end - visible_start).unwrap_or(area.height);
    let width = area.width.min(header.area.width);
    for row in 0..target_height {
        let source_y = visible_start.saturating_add(usize::from(row)) as u16;
        for column in 0..width {
            let Some(source) = header.cell((column, source_y)) else {
                continue;
            };
            if let Some(destination) = target.cell_mut((area.x + column, target_y + row)) {
                *destination = source.clone();
            }
        }
    }
}

fn render_cells(
    frame: &mut Frame<'_>,
    area: Rect,
    header_rows: usize,
    viewport_start: usize,
    messages: &[Message],
    heights: &[usize],
    cache: &ChatHistoryRenderCache,
    pointer: ChatHistoryPointerState<'_>,
    context: RenderContext<'_>,
) {
    let viewport_end = viewport_start.saturating_add(usize::from(area.height));
    let mut cell_start = header_rows;
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
                cell_lines_with_code(message, context, Some(cache), true)
            });
            cell.render(
                frame.buffer_mut(),
                Rect::new(area.x, target_y, area.width, target_height),
                source_row,
            );
            render_pointer_feedback(
                frame,
                area,
                message,
                cell_start,
                cell_end,
                viewport_start,
                pointer,
                context,
            );
        }
        cell_start = cell_end;
        if cell_start >= viewport_end {
            break;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn render_pointer_feedback(
    frame: &mut Frame<'_>,
    area: Rect,
    message: &Message,
    cell_start: usize,
    cell_end: usize,
    viewport_start: usize,
    pointer: ChatHistoryPointerState<'_>,
    context: RenderContext<'_>,
) {
    let Some(cell_id) = message.cell_id.as_deref() else {
        return;
    };
    let toggle_hovered = pointer.hovered_toggle == Some(cell_id);
    let toggle_pressed = pointer.pressed_toggle == Some(cell_id);
    if message.can_expand && (toggle_hovered || toggle_pressed) {
        render_action_feedback(
            frame,
            area,
            cell_start,
            viewport_start,
            1,
            toggle_hovered,
            toggle_pressed,
            context,
        );
    }
    let details_hovered = pointer.hovered_details == Some(cell_id);
    let details_pressed = pointer.pressed_details == Some(cell_id);
    if message.expanded && message.has_details && (details_hovered || details_pressed) {
        render_action_feedback(
            frame,
            area,
            cell_end.saturating_sub(2),
            viewport_start,
            "   view full".len() as u16,
            details_hovered,
            details_pressed,
            context,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn render_action_feedback(
    frame: &mut Frame<'_>,
    area: Rect,
    logical_row: usize,
    viewport_start: usize,
    width: u16,
    hovered: bool,
    pressed: bool,
    context: RenderContext<'_>,
) {
    let Some(row) = logical_row.checked_sub(viewport_start) else {
        return;
    };
    if row >= usize::from(area.height) {
        return;
    }
    let style = interaction_style(
        context,
        InteractionState {
            target: InteractionTarget::Rest,
            selected: false,
            hovered,
            pressed,
        },
    )
    .add_modifier(Modifier::BOLD);
    frame.buffer_mut().set_style(
        Rect::new(
            area.x,
            area.y.saturating_add(row as u16),
            width.min(area.width),
            1,
        ),
        style,
    );
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
