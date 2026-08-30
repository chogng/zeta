use crate::app::App;
use crate::components::chat_composer;
use crate::components::chat_composer::ChatComposerAreas;
use crate::components::chat_composer::ChatComposerPointerTarget;
use crate::components::chat_composer::ChatComposerSurface;
use crate::components::chat_history;
use crate::components::chat_history::ChatHistoryView;
use crate::components::chat_input;
use crate::components::key_hint_bar;
use crate::components::welcome;
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
use std::borrow::Cow;
use unicode_width::UnicodeWidthStr;

const STATUS_HINT_GAP: u16 = 3;

enum StatusAreaView<'a> {
    Hint {
        text: Cow<'a, str>,
        style: StatusHintStyle,
    },
    StatusLine {
        supplemental_hint: Option<&'a str>,
    },
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
    let presentation_highlight = app
        .list_selection()
        .and_then(|view| view.presentation_highlight())
        .unwrap_or_else(|| context.highlight());

    if let Some(manager) = app.session_manager_view() {
        let manager_areas = super::screen_layout::manager_areas(
            areas.session.transcript,
            welcome::desired_height(areas.session.transcript.width),
        );
        welcome::draw(
            frame,
            manager_areas.welcome,
            app.welcome(),
            presentation_highlight,
            context,
        );
        sessions::draw_manager(frame, manager_areas.sessions, manager, context);
    } else {
        let messages = app.transcript_views();
        ChatHistoryView {
            messages: &messages,
            scroll: app.transcript_scroll(),
            render_cache: app.transcript_render_cache(),
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
    draw_status_notice(
        frame,
        areas.session.transcript,
        areas.session.composer,
        app.status_notice(),
        context,
    );
    if let Some(quick_view) = app.quick_view() {
        crate::components::quick_view::draw(frame, overlay_area(&areas), quick_view, context);
    }
    app.screen_selection().draw(frame.buffer_mut());
}

fn draw_status_notice(
    frame: &mut Frame<'_>,
    content: Rect,
    composer: Rect,
    notice: Option<&str>,
    context: crate::render::RenderContext<'_>,
) {
    let Some(notice) = notice else {
        return;
    };
    if content.is_empty() || composer.is_empty() || composer.y <= content.y {
        return;
    }
    key_hint_bar::draw_right(
        frame,
        Rect {
            x: content.x,
            y: composer.y.saturating_sub(1),
            width: content.width,
            height: 1,
        },
        notice,
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
    SessionManager(crate::features::sessions::SessionManagerPointerTarget),
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
    if let Some(manager) = app.session_manager_view() {
        let manager_areas = super::screen_layout::manager_areas(
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
    let status_area = status_area_view(app);
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
        status_area.desired_rows(app, terminal_area.width),
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
    match status_area_view(app) {
        StatusAreaView::Hint { text, style } => match style {
            StatusHintStyle::Keys => key_hint_bar::draw(frame, area, &text, context),
            StatusHintStyle::Warning => frame.render_widget(
                Paragraph::new(text.as_ref() as &str).style(Style::default().fg(context.warning())),
                chat_input::content_area(area),
            ),
            StatusHintStyle::Muted => frame.render_widget(
                Paragraph::new(text.as_ref() as &str).style(Style::default().fg(context.muted())),
                chat_input::content_area(area),
            ),
        },
        StatusAreaView::StatusLine { supplemental_hint } => {
            draw_status_line(frame, area, app, supplemental_hint, context)
        }
    }
}

fn status_area_view(app: &App) -> StatusAreaView<'_> {
    if app.session_manager_view().is_some() {
        let hint = if app.session_manager_focused() {
            app.session_manager_hint()
        } else {
            "↑ sessions · enter create"
        };
        return StatusAreaView::Hint {
            text: Cow::Borrowed(hint),
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
    if app.subagent_pane_focused() {
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
    StatusAreaView::StatusLine {
        supplemental_hint: app.root_navigation_hint(),
    }
}

fn draw_status_line(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    supplemental_hint: Option<&str>,
    context: crate::render::RenderContext<'_>,
) {
    let status_rows = status_line::desired_rows(app.status_line_runtime(), 2);
    let Some(hint) = supplemental_hint else {
        status_line::draw(
            frame,
            chat_input::content_area(area),
            app.status_line(),
            app.approval_mode_status(),
            app.status_line_runtime(),
            context,
        );
        return;
    };
    let separate_hint_row =
        !status_and_hint_fit(app, area.width, hint) && area.height > status_rows;
    let status_area = chat_input::content_area(if separate_hint_row {
        Rect {
            height: area.height.saturating_sub(1),
            ..area
        }
    } else {
        let hint_width = hint.width().min(usize::from(area.width)) as u16;
        Rect {
            width: area
                .width
                .saturating_sub(2)
                .saturating_sub(hint_width)
                .saturating_sub(STATUS_HINT_GAP),
            ..area
        }
    });
    status_line::draw(
        frame,
        status_area,
        app.status_line(),
        app.approval_mode_status(),
        app.status_line_runtime(),
        context,
    );
    let hint_area = if separate_hint_row {
        Rect {
            y: area.bottom().saturating_sub(1),
            height: 1,
            ..area
        }
    } else {
        Rect {
            y: area
                .y
                .saturating_add(status_rows.min(area.height).saturating_sub(1)),
            height: 1,
            ..area
        }
    };
    key_hint_bar::draw_right(frame, hint_area, hint, context);
}

fn status_and_hint_fit(app: &App, width: u16, hint: &str) -> bool {
    let status_width = usize::from(width.saturating_sub(2));
    let configured = app
        .status_line()
        .text_for_width(status_width, app.approval_mode_status());
    let runtime = app.status_line_runtime().text();
    let longest_status = configured.width().max(runtime.width());
    longest_status
        .saturating_add(usize::from(STATUS_HINT_GAP))
        .saturating_add(hint.width())
        <= usize::from(width.saturating_sub(4))
}

impl StatusAreaView<'_> {
    fn desired_rows(&self, app: &App, width: u16) -> u16 {
        match self {
            Self::Hint { .. } => 1,
            Self::StatusLine { supplemental_hint } => {
                let status_rows = status_line::desired_rows(app.status_line_runtime(), 2);
                status_rows
                    + u16::from(
                        supplemental_hint
                            .is_some_and(|hint| !status_and_hint_fit(app, width, hint)),
                    )
            }
        }
    }
}

#[cfg(test)]
#[path = "frame_tests.rs"]
mod tests;
