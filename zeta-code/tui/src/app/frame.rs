use crate::app::App;
use crate::app::input_surface::InputSurfacePointerTarget;
use crate::app::welcome;
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

enum StatusAreaView<'a> {
    Hidden,
    Hint {
        text: Cow<'a, str>,
        style: StatusHintStyle,
    },
    StatusLine,
}

#[derive(Clone, Copy)]
enum StatusHintStyle {
    Keys,
    Warning,
    Muted,
}

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
        if messages.is_empty() {
            welcome::draw(frame, areas.session.transcript, app.welcome(), context);
        } else {
            ChatHistoryView {
                messages: &messages,
                scroll: app.transcript_scroll(),
                render_cache: app.transcript_render_cache(),
                pointer: transcript_pointer_state(hovered, pressed),
            }
            .render(frame, areas.session.transcript, context);
        }
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
    } else if let Some(mode) = app.input_surface() {
        let hovered_mode = match hovered {
            Some(InputPointerTarget::InputSurface(target)) => Some(*target),
            _ => None,
        };
        let pressed_mode = match pressed {
            Some(InputPointerTarget::InputSurface(target)) => Some(*target),
            _ => None,
        };
        mode.draw(
            frame,
            areas.session.composer,
            hovered_mode,
            pressed_mode,
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
        queue::draw(
            frame,
            areas.session.queue,
            &queue_view,
            queue::DEFAULT_MAX_VISIBLE_ITEMS,
            context,
        );
    }
    draw_status_area(frame, areas.session.status, app, context);
    if let Some(agent_thread_switcher) = app.agent_thread_switcher_view() {
        crate::thread::draw_agent_thread_switcher(
            frame,
            chat_input::content_area(areas.session.agent_thread_switcher),
            agent_thread_switcher,
            context,
        );
    }
    draw_top_tip(
        frame,
        areas.session.transcript,
        areas.session.composer,
        app,
        context,
    );
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
    content: Rect,
    composer: Rect,
    app: &App,
    context: crate::render::RenderContext<'_>,
) {
    if content.is_empty() || composer.is_empty() || composer.y <= content.y {
        return;
    }
    app.top_tip().draw(
        frame,
        Rect {
            x: content.x,
            y: composer.y.saturating_sub(1),
            width: content.width,
            height: 1,
        },
        app.screen_navigation_tip(),
        context,
    );
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
    InputSurface(InputSurfacePointerTarget),
    Approval(usize),
    Query(usize),
    SessionManager(crate::sessions::SessionManagerPointerTarget),
    TranscriptToggle(String),
    TranscriptDetails(String),
}

fn transcript_pointer_state<'a>(
    hovered: Option<&'a InputPointerTarget>,
    pressed: Option<&'a InputPointerTarget>,
) -> ChatHistoryPointerState<'a> {
    ChatHistoryPointerState {
        hovered_toggle: match hovered {
            Some(InputPointerTarget::TranscriptToggle(cell_id)) => Some(cell_id.as_str()),
            _ => None,
        },
        hovered_details: match hovered {
            Some(InputPointerTarget::TranscriptDetails(cell_id)) => Some(cell_id.as_str()),
            _ => None,
        },
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
    if let Some(mode) = app.input_surface()
        && let Some(target) = mode.pointer_target_at(areas.session.composer, column, row)
    {
        return Some(InputPointerTarget::InputSurface(target));
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
        let messages = app.transcript_views();
        if let Some(target) = chat_history::pointer_target_at(
            areas.session.transcript,
            &messages,
            app.transcript_scroll(),
            app.transcript_render_cache(),
            app.render_context(),
            column,
            row,
        ) {
            return Some(match target {
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
    let mode_rows = app
        .input_surface()
        .map(|mode| mode.desired_height(terminal_area.width))
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
    } else if mode_rows > 0 {
        mode_rows
    } else {
        input_rows
    };
    let queue_rows = if app.session_manager_view().is_some() {
        0
    } else {
        let queue_view = app.queue_view();
        queue::desired_height(&queue_view, queue::DEFAULT_MAX_VISIBLE_ITEMS)
    };
    let status_area = status_area_view(app);
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
        status_area.desired_rows(app),
        app.agent_thread_switcher_rows(),
    );
    let input = if approval_rows > 0 || mode_rows > 0 {
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
            .status
            .y
            .saturating_sub(areas.session.transcript.y),
    }
}

fn completion_visible(app: &App) -> bool {
    app.overlay().is_none()
        && app.input_surface().is_none()
        && app.approval_view().is_none()
        && app.query_view().is_none()
        && app.completion().is_some()
}

fn draw_status_area(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    context: crate::render::RenderContext<'_>,
) {
    match status_area_view(app) {
        StatusAreaView::Hidden => {}
        StatusAreaView::Hint { text, style } => match style {
            StatusHintStyle::Keys => key_hint::draw(frame, area, &text, context),
            StatusHintStyle::Warning => frame.render_widget(
                Paragraph::new(text.as_ref() as &str).style(Style::default().fg(context.warning())),
                chat_input::content_area(area),
            ),
            StatusHintStyle::Muted => frame.render_widget(
                Paragraph::new(text.as_ref() as &str).style(Style::default().fg(context.muted())),
                chat_input::content_area(area),
            ),
        },
        StatusAreaView::StatusLine => draw_status_line(frame, area, app, context),
    }
}

fn status_area_view(app: &App) -> StatusAreaView<'_> {
    if let Some(hints) = app.composer_key_hints() {
        if hints.is_empty() {
            return StatusAreaView::Hidden;
        }
        return StatusAreaView::Hint {
            text: Cow::Borrowed(hints),
            style: StatusHintStyle::Keys,
        };
    }
    if app.session_manager_view().is_some() {
        return StatusAreaView::Hint {
            text: Cow::Borrowed(app.session_manager_hint()),
            style: StatusHintStyle::Keys,
        };
    }
    if app.approval_view().is_some() {
        return StatusAreaView::Hint {
            text: Cow::Borrowed("↑↓ choose · enter confirm"),
            style: StatusHintStyle::Keys,
        };
    }
    if app.query_view().is_some() {
        return StatusAreaView::Hint {
            text: Cow::Borrowed("↑↓ choose · enter answer · esc cancel custom input"),
            style: StatusHintStyle::Keys,
        };
    }
    if app.transcript_selection_active() {
        return StatusAreaView::Hint {
            text: Cow::Borrowed("↑↓ select · space expand · enter details · esc input"),
            style: StatusHintStyle::Keys,
        };
    }
    if app.agent_thread_switcher_focused() {
        return StatusAreaView::Hint {
            text: Cow::Borrowed("↑↓ select · enter switch · esc input"),
            style: StatusHintStyle::Keys,
        };
    }
    if let Some(prefix) = app.pending_key_chord_label() {
        return StatusAreaView::Hint {
            text: Cow::Owned(format!("{prefix} … waiting for next key · esc cancel")),
            style: StatusHintStyle::Warning,
        };
    }
    if app.viewed_thread_completed() {
        return StatusAreaView::Hint {
            text: Cow::Borrowed("completed · choose Main or another Subagent"),
            style: StatusHintStyle::Muted,
        };
    }
    StatusAreaView::StatusLine
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

impl StatusAreaView<'_> {
    fn desired_rows(&self, app: &App) -> u16 {
        match self {
            Self::Hidden => 0,
            Self::Hint { .. } => 1,
            Self::StatusLine => status_line::desired_rows(
                app.status_line(),
                app.approval_mode_status(),
                app.status_line_runtime(),
                2,
            ),
        }
    }
}

#[cfg(test)]
#[path = "frame_tests.rs"]
mod tests;
