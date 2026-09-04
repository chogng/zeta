use crate::app::App;
use crate::app::command_panel::CommandPanelPointerTarget;
use crate::app::welcome;
use crate::host::process_resources::ProcessResourceDemand;
use crate::render::Renderable;
use crate::sessions;
use crate::status as status_line;
use crate::thread::composer as chat_composer;
use crate::thread::composer as chat_input;
use crate::thread::composer::ChatComposerPointerTarget;
use crate::thread::composer::ChatComposerSurface;
use crate::thread::goal;
use crate::thread::interaction::approval;
use crate::thread::interaction::query;
use crate::thread::plan;
use crate::thread::queue;
use crate::thread::queue::QueueId;
use crate::thread::transcript as chat_history;
use crate::thread::transcript::ChatHistoryPointerState;
use crate::thread::transcript::ChatHistoryView;
use crate::widgets::key_hint;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use std::borrow::Cow;

enum BottomContent<'a> {
    HitBar {
        text: Cow<'a, str>,
        style: HitBarStyle,
    },
    StatusLine,
}

#[derive(Clone, Copy)]
enum HitBarStyle {
    Keys,
    Warning,
    Muted,
}

const BOTTOM_ROWS: u16 = 2;

pub(crate) fn draw(frame: &mut Frame<'_>, app: &App) {
    let context = app.render_context();
    frame.render_widget(
        Block::default().style(
            Style::default()
                .fg(context.foreground())
                .bg(context.background()),
        ),
        frame.area(),
    );
    let areas = layout(app, frame.area());
    let hovered = app.hovered_pointer_target();
    let pressed = app.pressed_pointer_target();
    if let Some(manager) = app.session_manager_view() {
        let manager_areas = super::layout::manager_areas(
            areas.session.transcript,
            welcome::desired_height(areas.session.transcript.width),
        );
        welcome::draw(frame, manager_areas.welcome, app.welcome(), context);
        let hovered_manager = match hovered {
            Some(InputPointerTarget::SessionManager(target)) => Some(target),
            _ => None,
        };
        let pressed_manager = match pressed {
            Some(InputPointerTarget::SessionManager(target)) => Some(target),
            _ => None,
        };
        sessions::draw_manager(
            frame,
            manager_areas.sessions,
            manager,
            hovered_manager,
            pressed_manager,
            context,
        );
    } else {
        let messages = app.transcript_views();
        let header = welcome::history_buffer(
            areas.session.transcript.width,
            areas.session.transcript.height,
            app.welcome(),
            context,
        );
        ChatHistoryView {
            header: Some(&header),
            messages: &messages,
            scroll: app.transcript_scroll(),
            render_cache: app.transcript_render_cache(),
            pointer: transcript_pointer_state(hovered, pressed),
        }
        .render(frame, areas.session.transcript, context);
    }
    let cursor = if app.accepts_input() && app.chat_input_focused() {
        chat_input::ChatInputCursor::Visible
    } else {
        chat_input::ChatInputCursor::Hidden
    };
    let input_view = app.chat_composer_view();
    if let Some(approval) = app.approval_view() {
        let hovered_choice = match hovered {
            Some(InputPointerTarget::Approval(index)) => Some(*index),
            _ => None,
        };
        let pressed_choice = match pressed {
            Some(InputPointerTarget::Approval(index)) => Some(*index),
            _ => None,
        };
        approval::draw(
            frame,
            areas.session.composer,
            approval,
            hovered_choice,
            pressed_choice,
            context,
        );
    } else if let Some(panel) = app.command_panel() {
        let hovered_panel = match hovered {
            Some(InputPointerTarget::CommandPanel(target)) => Some(*target),
            _ => None,
        };
        let pressed_panel = match pressed {
            Some(InputPointerTarget::CommandPanel(target)) => Some(*target),
            _ => None,
        };
        panel.draw(
            frame,
            areas.session.composer,
            hovered_panel,
            pressed_panel,
            context,
        );
    } else {
        ChatComposerSurface {
            view: &input_view,
            cursor,
        }
        .render(frame, areas.input, context);
    }
    if let Some(query) = app.query_view() {
        let hovered_choice = match hovered {
            Some(InputPointerTarget::Query(index)) => Some(*index),
            _ => None,
        };
        let pressed_choice = match pressed {
            Some(InputPointerTarget::Query(index)) => Some(*index),
            _ => None,
        };
        query::draw(
            frame,
            areas.session.request,
            query,
            hovered_choice,
            pressed_choice,
            context,
        );
    }
    if app.session_manager_view().is_none() {
        goal::draw(frame, areas.session.goal, app.goal_view(), context);
        plan::draw(frame, areas.session.plan, app.plan_view(), context);
        let queue_view = app.queue_view();
        let hovered_queue = match hovered {
            Some(InputPointerTarget::Queue(queue_id)) => Some(*queue_id),
            _ => None,
        };
        let pressed_queue = match pressed {
            Some(InputPointerTarget::Queue(queue_id)) => Some(*queue_id),
            _ => None,
        };
        queue::draw(
            frame,
            areas.session.queue,
            &queue_view,
            queue::DEFAULT_MAX_VISIBLE_ITEMS,
            hovered_queue,
            pressed_queue,
            context,
        );
    }
    draw_bottom(frame, areas.session.bottom, app, context);
    if let Some(agent_thread_switcher) = app.agent_thread_switcher_view() {
        crate::thread::draw_agent_thread_switcher(
            frame,
            chat_input::content_area(areas.session.agent_thread_switcher),
            agent_thread_switcher,
            context,
        );
    }
    draw_top_tip(frame, areas.session.top_tip, app, context);
    if let Some(overlay) = app.overlay() {
        crate::widgets::overlay::draw(frame, transient_area(&areas), overlay, context);
    } else if completion_visible(app) {
        let hovered_composer = match hovered {
            Some(InputPointerTarget::Composer(target)) => Some(*target),
            _ => None,
        };
        let pressed_composer = match pressed {
            Some(InputPointerTarget::Composer(target)) => Some(*target),
            _ => None,
        };
        chat_composer::draw_completion_layer(
            frame,
            completion_area(&areas),
            &input_view,
            hovered_composer,
            pressed_composer,
            context,
        );
    }
    app.screen_selection().draw(frame.buffer_mut(), context);
}

fn draw_top_tip(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    context: crate::render::RenderContext<'_>,
) {
    if area.is_empty() {
        return;
    }
    app.top_tip()
        .draw(frame, area, app.screen_navigation_tip(), context);
}

#[cfg(test)]
pub(crate) fn input_overlay_index_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<usize> {
    match input_pointer_target_at(app, terminal_area, column, row) {
        Some(InputPointerTarget::Approval(index) | InputPointerTarget::Query(index)) => Some(index),
        Some(InputPointerTarget::Composer(ChatComposerPointerTarget::CompletionItem(index))) => {
            Some(index)
        }
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InputPointerTarget {
    Composer(ChatComposerPointerTarget),
    CommandPanel(CommandPanelPointerTarget),
    Approval(usize),
    Query(usize),
    SessionManager(crate::sessions::SessionManagerPointerTarget),
    Queue(QueueId),
    TranscriptJumpToBottom,
    TranscriptToggle(String),
    TranscriptDetails(String),
}

fn transcript_pointer_state<'a>(
    hovered: Option<&'a InputPointerTarget>,
    pressed: Option<&'a InputPointerTarget>,
) -> ChatHistoryPointerState<'a> {
    ChatHistoryPointerState {
        hovered_jump_to_bottom: matches!(hovered, Some(InputPointerTarget::TranscriptJumpToBottom)),
        hovered_toggle: match hovered {
            Some(InputPointerTarget::TranscriptToggle(cell_id)) => Some(cell_id.as_str()),
            _ => None,
        },
        hovered_details: match hovered {
            Some(InputPointerTarget::TranscriptDetails(cell_id)) => Some(cell_id.as_str()),
            _ => None,
        },
        pressed_jump_to_bottom: matches!(pressed, Some(InputPointerTarget::TranscriptJumpToBottom)),
        pressed_toggle: match pressed {
            Some(InputPointerTarget::TranscriptToggle(cell_id)) => Some(cell_id.as_str()),
            _ => None,
        },
        pressed_details: match pressed {
            Some(InputPointerTarget::TranscriptDetails(cell_id)) => Some(cell_id.as_str()),
            _ => None,
        },
    }
}

pub(crate) fn input_pointer_target_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<InputPointerTarget> {
    let areas = layout(app, terminal_area);
    if app.overlay().is_some() {
        return None;
    }
    let input_view = app.chat_composer_view();
    if completion_visible(app)
        && let Some(target) = chat_composer::pointer_target_at(
            completion_area(&areas),
            &input_view,
            true,
            column,
            row,
        )
    {
        return Some(InputPointerTarget::Composer(target));
    }
    if let Some(view) = app.approval_view() {
        return approval::choice_index_at(areas.session.composer, view, column, row)
            .map(InputPointerTarget::Approval);
    }
    if let Some(view) = app.query_view()
        && let Some(index) = query::choice_index_at(areas.session.request, view, column, row)
    {
        return Some(InputPointerTarget::Query(index));
    }
    if let Some(panel) = app.command_panel()
        && let Some(target) = panel.pointer_target_at(areas.session.composer, column, row)
    {
        return Some(InputPointerTarget::CommandPanel(target));
    }
    if let Some(manager) = app.session_manager_view() {
        let manager_areas = super::layout::manager_areas(
            areas.session.transcript,
            welcome::desired_height(areas.session.transcript.width),
        );
        if let Some(target) =
            sessions::pointer_target_at(manager_areas.sessions, manager, column, row)
        {
            return Some(InputPointerTarget::SessionManager(target));
        }
    } else {
        let queue_view = app.queue_view();
        if let Some(queue_id) = queue::pointer_target_at(
            areas.session.queue,
            &queue_view,
            queue::DEFAULT_MAX_VISIBLE_ITEMS,
            column,
            row,
        ) {
            return Some(InputPointerTarget::Queue(queue_id));
        }
        let messages = app.transcript_views();
        if let Some(target) = chat_history::pointer_target_at(
            areas.session.transcript,
            usize::from(welcome::history_height(areas.session.transcript.height)),
            &messages,
            app.transcript_scroll(),
            app.transcript_render_cache(),
            app.render_context(),
            column,
            row,
        ) {
            return Some(match target {
                chat_history::ChatHistoryPointerTarget::JumpToBottom => {
                    InputPointerTarget::TranscriptJumpToBottom
                }
                chat_history::ChatHistoryPointerTarget::Toggle(entry_id) => {
                    InputPointerTarget::TranscriptToggle(entry_id)
                }
                chat_history::ChatHistoryPointerTarget::Details(entry_id) => {
                    InputPointerTarget::TranscriptDetails(entry_id)
                }
            });
        }
    }
    None
}

pub(crate) fn transcript_contains(app: &App, terminal_area: Rect, column: u16, row: u16) -> bool {
    if app.overlay().is_some() || app.session_manager_view().is_some() {
        return false;
    }
    let area = layout(app, terminal_area).session.transcript;
    column >= area.x && column < area.right() && row >= area.y && row < area.bottom()
}

pub(crate) struct FrameLayout {
    pub(crate) session: super::layout::SessionAreas,
    pub(crate) input: Rect,
}

pub(crate) fn layout(app: &App, terminal_area: Rect) -> FrameLayout {
    let input_view = app.chat_composer_view();
    let input_rows = ChatComposerSurface {
        view: &input_view,
        cursor: chat_input::ChatInputCursor::Hidden,
    }
    .desired_height(terminal_area.width, app.render_context());
    let command_panel_rows = app
        .command_panel()
        .map(|panel| panel.desired_height(terminal_area.width))
        .unwrap_or_default();
    let approval_rows = app
        .approval_view()
        .map(approval::desired_height)
        .unwrap_or_default();
    let query_rows = app
        .query_view()
        .map(query::desired_height)
        .unwrap_or_default();
    let composer_rows = if approval_rows > 0 {
        approval_rows
    } else if command_panel_rows > 0 {
        command_panel_rows
    } else {
        input_rows
    };
    let queue_rows = if app.session_manager_view().is_some() {
        0
    } else {
        let queue_view = app.queue_view();
        queue::desired_height(&queue_view, queue::DEFAULT_MAX_VISIBLE_ITEMS)
    };
    let session = super::layout::session_areas(
        terminal_area,
        if app.session_manager_view().is_some() {
            0
        } else {
            goal::desired_height(app.goal_view())
        },
        if app.session_manager_view().is_some() {
            0
        } else {
            plan::desired_height(app.plan_view())
        },
        queue_rows,
        query_rows,
        composer_rows,
        BOTTOM_ROWS,
        app.agent_thread_switcher_rows(),
    );
    let input = if approval_rows > 0 || command_panel_rows > 0 {
        Rect {
            y: session.composer.bottom(),
            height: 0,
            ..session.composer
        }
    } else {
        let height = input_rows.min(session.composer.height);
        Rect {
            y: session.composer.bottom().saturating_sub(height),
            height,
            ..session.composer
        }
    };
    FrameLayout { session, input }
}

pub(crate) fn process_resource_demand(app: &App, terminal_area: Rect) -> ProcessResourceDemand {
    let areas = layout(app, terminal_area);
    if app
        .command_panel()
        .is_some_and(|panel| panel.process_resources_visible(areas.session.composer))
    {
        return ProcessResourceDemand::Processes;
    }
    if !matches!(bottom_content(app), BottomContent::StatusLine) {
        return ProcessResourceDemand::Disabled;
    }
    let area = chat_input::content_area(areas.session.bottom);
    if area.is_empty() {
        return ProcessResourceDemand::Disabled;
    }
    let policy = app
        .status_line()
        .policy_text_for_width(usize::from(area.width), app.approval_mode_status());
    if area.height == 1 && !policy.is_empty() {
        return ProcessResourceDemand::Disabled;
    }
    app.status_line()
        .visible_process_resources(usize::from(area.width), app.status_line_runtime())
        .map_or(
            ProcessResourceDemand::Disabled,
            ProcessResourceDemand::StatusLine,
        )
}

fn completion_area(areas: &FrameLayout) -> Rect {
    Rect {
        x: areas.session.transcript.x,
        y: areas.session.transcript.y,
        width: areas.session.transcript.width,
        height: areas.input.y.saturating_sub(areas.session.transcript.y),
    }
}

fn transient_area(areas: &FrameLayout) -> Rect {
    Rect {
        x: areas.session.transcript.x,
        y: areas.session.transcript.y,
        width: areas.session.transcript.width,
        height: areas
            .session
            .bottom
            .y
            .saturating_sub(areas.session.transcript.y),
    }
}

fn completion_visible(app: &App) -> bool {
    app.overlay().is_none()
        && app.command_panel().is_none()
        && app.approval_view().is_none()
        && app.query_view().is_none()
        && app.completion().is_some()
}

fn draw_bottom(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    context: crate::render::RenderContext<'_>,
) {
    match bottom_content(app) {
        BottomContent::HitBar { text, style } => match style {
            HitBarStyle::Keys => key_hint::draw(frame, bottom_row(area), &text, context),
            HitBarStyle::Warning => frame.render_widget(
                Paragraph::new(text.as_ref() as &str).style(Style::default().fg(context.warning())),
                chat_input::content_area(bottom_row(area)),
            ),
            HitBarStyle::Muted => frame.render_widget(
                Paragraph::new(text.as_ref() as &str).style(Style::default().fg(context.muted())),
                chat_input::content_area(bottom_row(area)),
            ),
        },
        BottomContent::StatusLine => draw_status_line(frame, area, app, context),
    }
}

fn bottom_content(app: &App) -> BottomContent<'_> {
    if let Some(hints) = app.command_panel_key_hints() {
        return BottomContent::HitBar {
            text: Cow::Borrowed(hints),
            style: HitBarStyle::Keys,
        };
    }
    if app.session_manager_view().is_some() {
        return BottomContent::HitBar {
            text: Cow::Borrowed(app.session_manager_hint()),
            style: HitBarStyle::Keys,
        };
    }
    if app.approval_view().is_some() {
        return BottomContent::HitBar {
            text: Cow::Borrowed("↑↓ to choose · Enter to confirm"),
            style: HitBarStyle::Keys,
        };
    }
    if app.query_view().is_some() {
        return BottomContent::HitBar {
            text: Cow::Borrowed("↑↓ to choose · Enter to answer · Esc to cancel custom input"),
            style: HitBarStyle::Keys,
        };
    }
    if app.queue_focused() {
        return BottomContent::HitBar {
            text: Cow::Borrowed(app.queue_key_hints()),
            style: HitBarStyle::Keys,
        };
    }
    if app.transcript_selection_active() {
        return BottomContent::HitBar {
            text: Cow::Borrowed(
                "↑↓ to select · Space to expand · Enter to view details · Esc to return to input",
            ),
            style: HitBarStyle::Keys,
        };
    }
    if app.agent_thread_switcher_focused() {
        return BottomContent::HitBar {
            text: Cow::Borrowed("↑↓ to select · Enter to switch · Esc to return to input"),
            style: HitBarStyle::Keys,
        };
    }
    if let Some(prefix) = app.pending_key_chord_label() {
        return BottomContent::HitBar {
            text: Cow::Owned(format!("{prefix} … waiting for next key · Esc to cancel")),
            style: HitBarStyle::Warning,
        };
    }
    if app.viewed_thread_completed() {
        return BottomContent::HitBar {
            text: Cow::Borrowed("completed · choose Main or another Subagent"),
            style: HitBarStyle::Muted,
        };
    }
    BottomContent::StatusLine
}

fn draw_status_line(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    context: crate::render::RenderContext<'_>,
) {
    status_line::draw(
        frame,
        chat_input::content_area(area),
        app.status_line(),
        app.approval_mode_status(),
        app.status_line_runtime(),
        context,
    );
}

fn bottom_row(area: Rect) -> Rect {
    Rect {
        y: area.bottom().saturating_sub(1),
        height: area.height.min(1),
        ..area
    }
}

#[cfg(test)]
#[path = "frame_tests.rs"]
mod tests;
