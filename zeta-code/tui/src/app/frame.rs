mod footer;

use crate::app::App;
use crate::components::approval;
use crate::components::chat_history;
use crate::components::chat_input;
use crate::components::chat_input_area;
use crate::components::chat_input_area::ChatInputAreaAreas;
use crate::components::chat_input_area::ChatInputAreaHeightEntryKind;
use crate::components::chat_input_area::ChatInputAreaHeightEntryView;
use crate::components::chat_input_area::ChatInputAreaOverlayView;
use crate::components::chat_input_area::PaneEntryView;
use crate::components::chat_widget;
use crate::components::chat_widget::ChatWidgetAreas;
use crate::components::detail_list;
use crate::components::key_capture;
use crate::components::key_hint_bar;
use crate::components::list_selection;
use crate::components::pane;
use crate::components::plan_progress;
use crate::components::query;
use crate::components::queue;
use crate::components::text_prompt;
use crate::ui::background;
use crate::ui::bottom_anchored_area;
use crate::ui::foreground;
use crate::ui::highlight;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::widgets::Block;

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
    for (entry, allocation) in app
        .input_height_entries()
        .into_iter()
        .zip(&areas.input.height_entries)
    {
        debug_assert_eq!(entry.kind(), allocation.kind);
        match entry {
            ChatInputAreaHeightEntryView::Pane(view) => {
                let pane_areas = pane::areas(allocation.area);
                let key_hints = match view {
                    PaneEntryView::DetailList(view) => {
                        detail_list::draw(frame, pane_areas.body, view.body());
                        view.key_hints()
                    }
                    PaneEntryView::KeyCapture(view) => {
                        key_capture::draw(frame, pane_areas.body, view.body());
                        view.key_hints()
                    }
                    PaneEntryView::ListSelection(view) => {
                        list_selection::draw(frame, pane_areas.body, view.body());
                        view.key_hints()
                    }
                    PaneEntryView::TextPrompt(view) => {
                        text_prompt::draw(frame, pane_areas.body, view.body());
                        view.key_hints()
                    }
                };
                key_hint_bar::draw(frame, pane_areas.key_hint_bar, key_hints);
            }
            ChatInputAreaHeightEntryView::PlanProgress(view) => {
                plan_progress::draw(frame, allocation.area, view);
            }
            ChatInputAreaHeightEntryView::Queue(view) => {
                queue::draw(frame, allocation.area, view);
            }
        }
    }

    let cursor = if app.accepts_input() && app.chat_input_focused() {
        chat_input::ChatInputCursor::Visible
    } else {
        chat_input::ChatInputCursor::Hidden
    };
    chat_input::draw_chat_input(
        frame,
        areas.input.input,
        app.input(),
        app.input_cursor_width(),
        app.input_cursor_line(),
        cursor,
    );
    footer::draw(frame, areas.widget.footer, app);

    let overlay_area = overlay_area(&areas);
    match app.input_overlay() {
        Some(ChatInputAreaOverlayView::Suggest(view)) => {
            chat_input::draw_suggest(frame, overlay_area, Some(view));
        }
        Some(ChatInputAreaOverlayView::Approval(view)) => {
            let area = bottom_anchored_area(overlay_area, approval::desired_height(view));
            approval::draw(frame, area, view);
        }
        Some(ChatInputAreaOverlayView::Query(view)) => {
            let area = bottom_anchored_area(overlay_area, query::desired_height(view));
            query::draw(frame, area, view);
        }
        None => {}
    }
}

pub(crate) fn input_overlay_index_at(
    app: &App,
    terminal_area: Rect,
    column: u16,
    row: u16,
) -> Option<usize> {
    let areas = layout(app, terminal_area);
    let available = overlay_area(&areas);
    match app.input_overlay()? {
        ChatInputAreaOverlayView::Suggest(view) => {
            chat_input::suggest_index_at(available, Some(view), column, row)
        }
        ChatInputAreaOverlayView::Approval(view) => {
            let area = bottom_anchored_area(available, approval::desired_height(view));
            approval::choice_index_at(area, view, column, row)
        }
        ChatInputAreaOverlayView::Query(view) => {
            let area = bottom_anchored_area(available, query::desired_height(view));
            query::choice_index_at(area, view, column, row)
        }
    }
}

pub(crate) struct FrameLayout {
    pub(crate) widget: ChatWidgetAreas,
    pub(crate) input: ChatInputAreaAreas,
}

pub(crate) fn layout(app: &App, terminal_area: Rect) -> FrameLayout {
    let input_height = app.chat_input_desired_height(terminal_area.width);
    let height_entries = app
        .input_height_entries()
        .into_iter()
        .map(|entry| {
            let kind = entry.kind();
            let height = match entry {
                ChatInputAreaHeightEntryView::Pane(view) => {
                    let body_height = match view {
                        PaneEntryView::DetailList(view) => view.body().desired_height(),
                        PaneEntryView::KeyCapture(view) => view.body().desired_height(),
                        PaneEntryView::ListSelection(view) => {
                            view.body().desired_height(terminal_area.width)
                        }
                        PaneEntryView::TextPrompt(view) => view.body().desired_height(),
                    };
                    pane::desired_height(body_height)
                }
                ChatInputAreaHeightEntryView::PlanProgress(view) => {
                    plan_progress::desired_height(view)
                }
                ChatInputAreaHeightEntryView::Queue(view) => queue::desired_height(view),
            };
            (kind, height)
        })
        .collect::<Vec<_>>();
    let desired_height = chat_input_area::desired_height(input_height, &height_entries);
    let widget = chat_widget::areas(terminal_area, desired_height);
    let input = chat_input_area::areas(widget.chat_input_area, input_height, &height_entries);
    FrameLayout { widget, input }
}

pub(crate) fn height_entry_area(
    app: &App,
    terminal_area: Rect,
    kind: ChatInputAreaHeightEntryKind,
) -> Option<Rect> {
    layout(app, terminal_area)
        .input
        .height_entries
        .into_iter()
        .find(|entry| entry.kind == kind)
        .map(|entry| entry.area)
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

#[cfg(test)]
#[path = "frame/frame_tests.rs"]
mod tests;
