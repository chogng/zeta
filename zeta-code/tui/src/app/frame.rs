use crate::app::App;
use crate::app::welcome;
use zeta_memory_diagnostics::ProcessResourceDemand;
use crate::keymap::bindings;
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
    if let Some(preview) = app.session_preview() {
        let messages = preview.messages();
        let header = welcome::history_buffer(
            areas.session.transcript.width,
            areas.session.transcript.height,
            app.welcome(),
            context,
        );
        ChatHistoryView {
            header: Some(&header),
            messages: &messages,
            scroll: &preview.scroll,
            render_cache: &preview.cache,
            pointer: Default::default(),
        }
        .render(frame, areas.session.transcript, context);
        let title = format!("  Preview · {} · read only", preview.title);
        frame.render_widget(
            Paragraph::new(title).style(Style::default().fg(context.muted())),
            areas.session.composer,
        );
        if let Some(notice) = preview.notice() {
            frame.render_widget(
                Paragraph::new(notice).style(Style::default().fg(context.muted())),
                areas.session.top_tip,
            );
        }
        draw_bottom(frame, areas.session.bottom, app, context);
        if let Some(overlay) = app.overlay() {
            crate::widgets::overlay::draw(
                frame,
                transient_area_from_layout(&areas),
                overlay,
                context,
            );
        }
        app.screen_selection().draw(frame.buffer_mut(), context);
        return;
    }
    let hovered = app.hovered_pointer_target();
    let pressed = app.pressed_pointer_target();
    if let Some(manager) = app.session_manager_view() {
        let manager_areas = super::layout::manager_areas(
            areas.session.transcript,
            welcome::desired_height(areas.session.transcript.width),
        );
        welcome::draw(frame, manager_areas.welcome, app.welcome(), context);
        sessions::draw_manager(frame, manager_areas.sessions, manager, None, None, context);
    } else {
        let messages = app.visible_transcript_views();
        let header = app.transcript_header_visible().then(|| {
            welcome::history_buffer(
                areas.session.transcript.width,
                areas.session.transcript.height,
                app.welcome(),
                context,
            )
        });
        ChatHistoryView {
            header: header.as_ref(),
            messages: &messages,
            scroll: app.transcript_scroll(),
            render_cache: app.transcript_render_cache(),
            pointer: Default::default(),
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
        approval::draw(frame, areas.session.composer, approval, None, None, context);
    } else if let Some(panel) = app.command_panel() {
        panel.draw(frame, areas.session.composer, None, None, context);
    } else {
        ChatComposerSurface {
            view: &input_view,
            cursor,
        }
        .render(frame, areas.input, context);
    }
    if let Some(query) = app.query_view() {
        query::draw(frame, areas.session.request, query, None, None, context);
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
            None,
            None,
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
        crate::widgets::overlay::draw(frame, transient_area_from_layout(&areas), overlay, context);
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
        Some(InputPointerTarget::Composer(ChatComposerPointerTarget::CompletionItem(index))) => {
            Some(index)
        }
        _ => None,
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InputPointerTarget {
    Composer(ChatComposerPointerTarget),
}

pub(crate) fn input_pointer_target_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<InputPointerTarget> {
    if app.mouse_mode() != crate::terminal::mouse::MouseMode::TuiCapture || !completion_visible(app)
    {
        return None;
    }
    let areas = layout(app, terminal_area);
    chat_composer::pointer_target_at(
        completion_area(&areas),
        &app.chat_composer_view(),
        true,
        column,
        row,
    )
    .map(InputPointerTarget::Composer)
}

pub(crate) fn overlay_mouse_contains(
    app: &App,
    terminal_area: Rect,
    position: ratatui::layout::Position,
) -> bool {
    if app.mouse_mode() != crate::terminal::mouse::MouseMode::TuiCapture {
        return false;
    }
    let areas = layout(app, terminal_area);
    if let Some(overlay) = app.overlay() {
        return overlay
            .surface(transient_area_from_layout(&areas))
            .contains(position);
    }
    matches!(
        input_pointer_target_at(app, terminal_area, position.x, position.y),
        Some(InputPointerTarget::Composer(_))
    )
}

pub(crate) struct FrameLayout {
    pub(crate) session: super::layout::SessionAreas,
    pub(crate) input: Rect,
}

pub(crate) fn layout(app: &App, terminal_area: Rect) -> FrameLayout {
    if app.session_preview().is_some() {
        let session = super::layout::session_areas(terminal_area, 0, 0, 0, 0, 1, 1, 0);
        return FrameLayout {
            input: Rect::default(),
            session,
        };
    }
    if let Some(panel) = app.command_panel() {
        return FrameLayout {
            session: super::layout::command_panel_areas(
                terminal_area,
                panel.desired_height(terminal_area.width),
                BOTTOM_ROWS,
            ),
            input: Rect::default(),
        };
    }
    let input_view = app.chat_composer_view();
    let input_rows = ChatComposerSurface {
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
    let input = if approval_rows > 0 {
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
        return ProcessResourceDemand::Detailed;
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
            ProcessResourceDemand::Summary,
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

fn transient_area_from_layout(areas: &FrameLayout) -> Rect {
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

pub(crate) fn transient_area(app: &App, terminal_area: Rect) -> Rect {
    transient_area_from_layout(&layout(app, terminal_area))
}

pub(super) fn completion_visible(app: &App) -> bool {
    app.session_preview().is_none()
        && app.overlay().is_none()
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
    if app.overlay().is_some() || app.session_preview().is_some() {
        return BottomContent::HitBar {
            text: Cow::Borrowed(bindings::CLOSE_HINTS.as_str()),
            style: HitBarStyle::Keys,
        };
    }
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
            text: Cow::Borrowed(bindings::APPROVAL_HINTS.as_str()),
            style: HitBarStyle::Keys,
        };
    }
    if let Some(query) = app.query_view() {
        return BottomContent::HitBar {
            text: Cow::Borrowed(if query.custom_answer.is_some() {
                bindings::CUSTOM_ANSWER_HINTS.as_str()
            } else {
                bindings::ANSWER_HINTS.as_str()
            }),
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
            text: Cow::Borrowed(bindings::TRANSCRIPT_HINTS.as_str()),
            style: HitBarStyle::Keys,
        };
    }
    if app.agent_thread_switcher_focused() {
        return BottomContent::HitBar {
            text: Cow::Borrowed(bindings::THREAD_HINTS.as_str()),
            style: HitBarStyle::Keys,
        };
    }
    if let Some(prefix) = app.pending_key_chord_label() {
        return BottomContent::HitBar {
            text: Cow::Owned(format!(
                "{prefix} … waiting for next key · {}",
                bindings::CANCEL_HINTS.as_str()
            )),
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
