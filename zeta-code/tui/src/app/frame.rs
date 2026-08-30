use crate::app::App;
use crate::app::Status;
use crate::components::chat_history;
use crate::components::chat_input;
use crate::components::chat_input_area;
use crate::components::chat_input_area::ChatInputAreaAreas;
use crate::components::chat_input_area::ChatInputAreaPointerTarget;
use crate::components::chat_widget;
use crate::components::chat_widget::ChatWidgetAreas;
use crate::features::config::FollowUpMode;
use crate::features::status_line;
use crate::ui::background;
use crate::ui::foreground;
use crate::ui::highlight;
use crate::ui::muted;
use crate::ui::warning;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;

pub(crate) fn draw(frame: &mut Frame<'_>, app: &App) {
    frame.render_widget(
        Block::default().style(Style::default().fg(foreground()).bg(background())),
        frame.area(),
    );
    let areas = layout(app, frame.area());
    let presentation_highlight = app
        .list_selection()
        .and_then(|view| view.presentation_highlight())
        .unwrap_or_else(highlight);

    chat_history::draw(
        frame,
        areas.widget.chat_history,
        app.messages(),
        app.transcript_scroll(),
        app.welcome(),
        presentation_highlight,
    );
    let cursor = if app.accepts_input() && app.chat_input_focused() {
        chat_input::ChatInputCursor::Visible
    } else {
        chat_input::ChatInputCursor::Hidden
    };
    let input_view = app.chat_input_area_view();
    chat_input_area::draw(
        frame,
        &areas.input,
        overlay_area(&areas),
        &input_view,
        cursor,
    );
    draw_footer(frame, areas.widget.footer, app);
}

#[cfg(test)]
pub(crate) fn input_overlay_index_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<usize> {
    match input_pointer_target_at(app, terminal_area, column, row) {
        Some(ChatInputAreaPointerTarget::OverlayItem(index)) => Some(index),
        _ => None,
    }
}

pub(crate) fn input_pointer_target_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<ChatInputAreaPointerTarget> {
    let areas = layout(app, terminal_area);
    let input_view = app.chat_input_area_view();
    chat_input_area::pointer_target_at(&areas.input, overlay_area(&areas), &input_view, column, row)
}

pub(crate) struct FrameLayout {
    pub(crate) widget: ChatWidgetAreas,
    pub(crate) input: ChatInputAreaAreas,
}

pub(crate) fn layout(app: &App, terminal_area: Rect) -> FrameLayout {
    let input_view = app.chat_input_area_view();
    let desired_height = chat_input_area::view_desired_height(&input_view, terminal_area.width);
    let widget = chat_widget::areas(terminal_area, desired_height);
    let input = chat_input_area::view_areas(widget.chat_input_area, &input_view);
    FrameLayout { widget, input }
}

fn overlay_area(areas: &FrameLayout) -> Rect {
    Rect {
        x: areas.widget.chat_history.x,
        y: areas.widget.chat_history.y,
        width: areas.widget.chat_history.width,
        height: areas
            .input
            .input
            .y
            .saturating_sub(areas.widget.chat_history.y),
    }
}

fn draw_footer(frame: &mut Frame<'_>, area: Rect, app: &App) {
    if let Some(prefix) = app.pending_key_chord_label() {
        frame.render_widget(
            Paragraph::new(format!("{prefix} … waiting for next key · esc cancel"))
                .style(Style::default().fg(warning())),
            area,
        );
        return;
    }
    if matches!(app.status(), Status::Working) && !app.input().trim().is_empty() {
        let hint = if app.steers_active_turn() && app.follow_up_mode() == FollowUpMode::Steer {
            "enter steer"
        } else {
            "enter queue"
        };
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(muted())),
            area,
        );
        return;
    }
    if app.has_editable_queue() && app.input().trim().is_empty() {
        let hint = if app.steers_active_turn() && app.follow_up_mode() == FollowUpMode::Steer {
            "enter send queued now · ↑ edit"
        } else {
            "↑ edit queued message"
        };
        frame.render_widget(
            Paragraph::new(hint).style(Style::default().fg(muted())),
            area,
        );
        return;
    }
    status_line::draw(frame, area, app.status_line(), app.approval_mode_status());
}

#[cfg(test)]
#[path = "frame_tests.rs"]
mod tests;
