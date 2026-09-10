use crate::app::App;
use crate::keymap::bindings;
use crate::render::horizontal_margin;
use crate::status as status_line;
use crate::thread::composer as chat_input;
use crate::widgets::key_hint;
use crate::widgets::key_hint::KeyHints;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Paragraph;
use unicode_width::UnicodeWidthStr;

enum BottomContent<'a> {
    Keys(&'a KeyHints),
    Warning(String),
    Muted(&'a str),
    InputHints,
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
    match bottom_content(app) {
        BottomContent::Keys(hints) => {
            key_hint::draw(
                frame,
                bottom_row(area),
                hints,
                app.key_hint_style(),
                context,
            );
        }
        BottomContent::Warning(text) => frame.render_widget(
            Paragraph::new(text).style(Style::default().fg(context.warning())),
            chat_input::content_area(bottom_row(area)),
        ),
        BottomContent::Muted(text) => frame.render_widget(
            Paragraph::new(text).style(Style::default().fg(context.muted())),
            chat_input::content_area(bottom_row(area)),
        ),
        BottomContent::InputHints => {
            let content = horizontal_margin(bottom_row(area), 2);
            let hints = input_hints(app);
            let first_hint_width = hints.text().split('·').next().unwrap().trim().width() as u16;
            let policy = status_line::policy_line(
                app.status_line(),
                usize::from(content.width.saturating_sub((first_hint_width + 3).max(14))),
                app.approval_mode_status(),
                context,
            );
            let policy_width = policy.width() as u16;
            let hint_area = Rect {
                width: content
                    .width
                    .saturating_sub(policy_width + if policy_width > 0 { 3 } else { 0 }),
                ..content
            };
            key_hint::draw_content(frame, hint_area, &hints, app.key_hint_style(), context);
            frame.render_widget(
                Paragraph::new(policy),
                Rect::new(
                    content.right().saturating_sub(policy_width),
                    content.y,
                    policy_width,
                    content.height,
                ),
            );
        }
    }
}

fn input_hints(app: &App) -> KeyHints {
    if app.fullscreen_home_visible() {
        if !app.fullscreen.input_focused() {
            return KeyHints::new()
                .with_compact_action("Enter", "select")
                .with_compact_action("↑↓", "actions")
                .with_compact_action("Esc", "input");
        }
        return KeyHints::new()
            .with_compact_action("Enter", "send")
            .with_compact_action("Tab", "actions")
            .with_compact_action("/", "commands");
    }
    let mut hints = KeyHints::new().with_compact_action(
        "Enter",
        if app.active_turn().is_some() {
            "queue"
        } else {
            "send"
        },
    );
    if let Some(keys) = app.app_keymap.action_hint(
        crate::keymap::AppKeymapAction::CycleApprovalMode,
        app.app_keymap_context(true),
    ) {
        hints = hints.with_compact_action(keys, "permissions");
    }
    hints.with_note("/home")
}

fn bottom_content(app: &App) -> BottomContent<'_> {
    if let Some(manager) = app.issue_manager() {
        return BottomContent::Keys(manager.key_hints());
    }
    if app.session_preview().is_some() {
        return BottomContent::Keys(&bindings::CLOSE_HINTS);
    }
    if app.session_manager_view().is_some() {
        return BottomContent::Keys(app.session_manager_hint());
    }
    if app.approval_view().is_some() {
        return BottomContent::Keys(&bindings::APPROVAL_HINTS);
    }
    if let Some(query) = app.query_view() {
        return BottomContent::Keys(if query.custom_answer.is_some() {
            &bindings::CUSTOM_ANSWER_HINTS
        } else {
            &bindings::ANSWER_HINTS
        });
    }
    if app.queue_focused() {
        return BottomContent::Keys(&bindings::QUEUE_HINTS);
    }
    if app.transcript_selection_active() {
        return BottomContent::Keys(&bindings::TRANSCRIPT_HINTS);
    }
    if app.agent_thread_switcher_focused() {
        return BottomContent::Keys(&bindings::THREAD_HINTS);
    }
    if let Some(prefix) = app.pending_key_chord_label() {
        return BottomContent::Warning(format!(
            "{prefix} … waiting for next key · {}",
            bindings::CANCEL_HINTS.text()
        ));
    }
    if app.viewed_thread_completed() {
        return BottomContent::Muted("completed · choose Main or another Subagent");
    }
    BottomContent::InputHints
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
