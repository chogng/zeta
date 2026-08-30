use crate::app::App;
use crate::components::chat_composer;
use crate::components::chat_composer::ChatComposerAreas;
use crate::components::chat_composer::ChatComposerPointerTarget;
use crate::components::chat_composer::ChatComposerSurface;
use crate::components::chat_history;
use crate::components::chat_history::ChatHistoryView;
use crate::components::chat_input;
use crate::components::key_hint_bar;
use crate::features::approval;
use crate::features::query;
use crate::features::queue;
use crate::features::sessions;
use crate::features::status_line;
use crate::features::thread::goal;
use crate::features::thread::plan;
use crate::render::Renderable;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;

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
    let presentation_highlight = app
        .list_selection()
        .and_then(|view| view.presentation_highlight())
        .unwrap_or_else(|| context.highlight());

    if let Some(manager) = app.session_manager_view() {
        sessions::draw_manager(frame, areas.session.transcript, manager, context);
    } else {
        let messages = app.transcript_views();
        ChatHistoryView {
            messages: &messages,
            scroll: app.transcript_scroll(),
            welcome: app.welcome(),
            presentation_highlight,
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
        approval::draw(frame, areas.session.composer, approval, context);
    } else {
        ChatComposerSurface {
            overlay_area: overlay_area(&areas),
            view: &input_view,
            cursor,
        }
        .render(frame, areas.session.composer, context);
    }
    if let Some(query) = app.query_view() {
        query::draw(frame, areas.session.request, query, context);
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
    if let Some(subagent_pane) = app.subagent_pane_view() {
        crate::features::thread::draw_subagent_pane(
            frame,
            chat_input::content_area(areas.session.subagent_pane),
            subagent_pane,
            context,
        );
    }
    if let Some(quick_view) = app.quick_view() {
        crate::components::quick_view::draw(frame, overlay_area(&areas), quick_view, context);
    }
    app.screen_selection().draw(frame.buffer_mut());
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
        Some(InputPointerTarget::Composer(ChatComposerPointerTarget::SuggestItem(index))) => {
            Some(index)
        }
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InputPointerTarget {
    Composer(ChatComposerPointerTarget),
    Approval(usize),
    Query(usize),
    TranscriptToggle(String),
    TranscriptDetails(String),
}

pub(crate) fn input_pointer_target_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<InputPointerTarget> {
    let areas = layout(app, terminal_area);
    if app.session_manager_view().is_none() {
        let messages = app.transcript_views();
        if let Some(target) = chat_history::pointer_target_at(
            areas.session.transcript,
            &messages,
            app.transcript_scroll(),
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
    if let Some(view) = app.approval_view() {
        return approval::choice_index_at(areas.session.composer, view, column, row)
            .map(InputPointerTarget::Approval);
    }
    if let Some(view) = app.query_view()
        && let Some(index) = query::choice_index_at(areas.session.request, view, column, row)
    {
        return Some(InputPointerTarget::Query(index));
    }
    let input_view = app.chat_composer_view();
    chat_composer::pointer_target_at(&areas.input, overlay_area(&areas), &input_view, column, row)
        .map(InputPointerTarget::Composer)
}

pub(crate) struct FrameLayout {
    pub(crate) session: super::screen_layout::SessionAreas,
    pub(crate) input: ChatComposerAreas,
}

pub(crate) fn layout(app: &App, terminal_area: Rect) -> FrameLayout {
    let input_view = app.chat_composer_view();
    let input_rows = ChatComposerSurface {
        overlay_area: Rect::default(),
        view: &input_view,
        cursor: chat_input::ChatInputCursor::Hidden,
    }
    .desired_height(terminal_area.width, app.render_context());
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
    } else {
        input_rows
    };
    let queue_rows = if app.session_manager_view().is_some() {
        0
    } else {
        let queue_view = app.queue_view();
        queue::desired_height(&queue_view, queue::DEFAULT_MAX_VISIBLE_ITEMS)
    };
    let session = super::screen_layout::session_areas(
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
        if app.thread_request_active()
            || app.transcript_selection_active()
            || app.subagent_pane_focused()
            || app.pending_key_chord_label().is_some()
            || app.viewed_thread_completed()
        {
            1
        } else {
            status_line::desired_rows(app.status_line_runtime(), 2)
        },
        app.subagent_pane_rows(),
    );
    let input = chat_composer::view_areas(session.composer, &input_view);
    FrameLayout { session, input }
}

fn overlay_area(areas: &FrameLayout) -> Rect {
    Rect {
        x: areas.session.transcript.x,
        y: areas.session.transcript.y,
        width: areas.session.transcript.width,
        height: areas
            .input
            .input
            .y
            .saturating_sub(areas.session.transcript.y),
    }
}

fn draw_status_area(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    context: crate::render::RenderContext<'_>,
) {
    if app.approval_view().is_some() {
        key_hint_bar::draw(frame, area, "↑↓ choose · enter confirm", context);
        return;
    }
    if app.query_view().is_some() {
        key_hint_bar::draw(
            frame,
            area,
            "↑↓ choose · enter answer · esc cancel custom input",
            context,
        );
        return;
    }
    if app.transcript_selection_active() {
        key_hint_bar::draw(
            frame,
            area,
            "↑↓ select · space expand · enter details · esc input",
            context,
        );
        return;
    }
    if app.subagent_pane_focused() {
        key_hint_bar::draw(frame, area, "↑↓ select · enter switch · esc input", context);
        return;
    }
    if let Some(prefix) = app.pending_key_chord_label() {
        frame.render_widget(
            Paragraph::new(format!("{prefix} … waiting for next key · esc cancel"))
                .style(Style::default().fg(context.warning())),
            chat_input::content_area(area),
        );
        return;
    }
    if app.viewed_thread_completed() {
        frame.render_widget(
            Paragraph::new("completed · choose Main or another Subagent")
                .style(Style::default().fg(context.muted())),
            chat_input::content_area(area),
        );
        return;
    }
    status_line::draw(
        frame,
        chat_input::content_area(area),
        app.status_line(),
        app.approval_mode_status(),
        app.status_line_runtime(),
        context,
    );
}

#[cfg(test)]
#[path = "frame_tests.rs"]
mod tests;
