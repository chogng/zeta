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
    if super::modal::is_open(app) {
        return;
    }
    let status_area = Rect {
        height: area.height.saturating_sub(1),
        ..area
    };
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
        BottomContent::StatusLine => {
            draw_status_line(frame, status_area, app, context);
            key_hint::draw(frame, bottom_row(area), &input_hints(app), context);
        }
    }
}

fn input_hints(app: &App) -> String {
    if app.fullscreen_home_visible() {
        return "Enter send  ·  Tab actions  ·  / commands".into();
    }
    let mut hints = if app.active_turn().is_some() {
        "Enter queue".to_owned()
    } else {
        "Enter send".to_owned()
    };
    if let Some(keys) = app.app_keymap.action_hint(
        crate::keymap::AppKeymapAction::CycleApprovalMode,
        app.app_keymap_context(true),
    ) {
        hints.push_str(&format!("  ·  {keys} permissions"));
    }
    hints.push_str("  ·  /home");
    hints
}

fn bottom_content(app: &App) -> BottomContent<'_> {
    if let Some(manager) = app.issue_manager() {
        return BottomContent::HitBar {
            text: Cow::Borrowed(manager.key_hints()),
            style: HitBarStyle::Keys,
        };
    }
    if app.session_preview().is_some() {
        return BottomContent::HitBar {
            text: Cow::Borrowed(bindings::CLOSE_HINTS.as_str()),
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
    if app.fullscreen.home_visible() {
        let text = if app.sessions.pending_submission.is_some() {
            "Starting session…"
        } else if let Some(error) = &app.sessions.creation_error {
            error
        } else {
            if app.sessions.active_session_id().is_some() {
                "Type a new task · Tab actions · Esc return"
            } else if area.width < 54 {
                "Type a task · Tab actions"
            } else {
                "Type a task to begin, or use Tab to choose an action."
            }
        };
        frame.render_widget(
            Paragraph::new(text).style(Style::default().fg(
                if app.sessions.creation_error.is_some() {
                    context.danger()
                } else {
                    context.muted()
                },
            )),
            chat_input::content_area(area),
        );
        return;
    }
    if !area.is_empty()
        && let Some(text) = app.top_tip().text(app.screen_navigation_tip())
    {
        key_hint::draw_right(frame, area, text, context);
    }
}
