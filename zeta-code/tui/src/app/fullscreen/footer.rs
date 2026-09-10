use super::layout::Layout;
use crate::app::App;
use crate::keymap::bindings;
use crate::status as status_line;
use crate::thread::composer as chat_input;
use crate::widgets::key_hint;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use std::borrow::Cow;
use zeta_memory_diagnostics::ProcessResourceDemand;

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

pub(super) fn process_resource_demand(app: &App, areas: &Layout) -> ProcessResourceDemand {
    if app
        .command_panel()
        .is_some_and(|panel| super::panel::process_resources_visible(panel, areas.session.composer))
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

pub(super) fn draw(
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
    if let Some(manager) = app.issue_manager() {
        return BottomContent::HitBar {
            text: Cow::Borrowed(manager.key_hints()),
            style: HitBarStyle::Keys,
        };
    }
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

pub(super) fn draw_tip(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &App,
    context: crate::render::RenderContext<'_>,
) {
    if !area.is_empty()
        && let Some(text) = app.top_tip().text(app.screen_navigation_tip())
    {
        key_hint::draw_right(frame, area, text, context);
    }
}
