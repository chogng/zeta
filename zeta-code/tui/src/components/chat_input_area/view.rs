use super::ChatInputAreaHeightEntryKind;
use super::ChatInputAreaHeightEntryView;
use super::ChatInputAreaOverlayView;
use super::ChatInputAreaView;
use super::PaneEntryView;
use crate::components::approval;
use crate::components::chat_input;
use crate::components::chat_input::ChatInputCursor;
use crate::components::detail_list;
use crate::components::key_capture;
use crate::components::key_hint_bar;
use crate::components::list_selection;
use crate::components::pane;
use crate::components::plan_progress;
use crate::components::query;
use crate::components::queue;
use crate::components::steer;
use crate::components::text_prompt;
use crate::ui::bottom_anchored_area;
use ratatui::Frame;
use ratatui::layout::Rect;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChatInputAreaAreas {
    pub(crate) height_entries: Vec<ChatInputAreaHeightEntryArea>,
    pub(crate) input: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ChatInputAreaHeightEntryArea {
    pub(crate) kind: ChatInputAreaHeightEntryKind,
    pub(crate) area: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChatInputAreaPointerTarget {
    PlanProgress,
    PaneTab(usize),
    PaneItem(usize),
    OverlayItem(usize),
}

pub(crate) fn view_desired_height(view: &ChatInputAreaView<'_>, available_width: u16) -> u16 {
    let input_height = view.input_desired_height(available_width);
    desired_height(input_height, &height_entries(view, available_width))
}

pub(crate) fn view_areas(area: Rect, view: &ChatInputAreaView<'_>) -> ChatInputAreaAreas {
    let input_height = view.input_desired_height(area.width);
    areas(area, input_height, &height_entries(view, area.width))
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    areas: &ChatInputAreaAreas,
    overlay_area: Rect,
    view: &ChatInputAreaView<'_>,
    cursor: ChatInputCursor,
) {
    for (entry, allocation) in view.height_entries().into_iter().zip(&areas.height_entries) {
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
            ChatInputAreaHeightEntryView::Steer(view) => {
                steer::draw(frame, allocation.area, &view);
            }
        }
    }

    chat_input::draw_chat_input(
        frame,
        areas.input,
        view.input(),
        view.input_cursor_width(),
        view.input_cursor_line(),
        cursor,
    );

    match view.overlay() {
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

pub(crate) fn pointer_target_at(
    areas: &ChatInputAreaAreas,
    overlay_area: Rect,
    view: &ChatInputAreaView<'_>,
    column: u16,
    row: u16,
) -> Option<ChatInputAreaPointerTarget> {
    for (entry, allocation) in view.height_entries().into_iter().zip(&areas.height_entries) {
        match entry {
            ChatInputAreaHeightEntryView::Pane(PaneEntryView::ListSelection(view)) => {
                let body = pane::areas(allocation.area).body;
                if let Some(index) = view.body().tab_index_at(body, column, row) {
                    return Some(ChatInputAreaPointerTarget::PaneTab(index));
                }
                if let Some(index) = view.body().item_index_at(body, column, row) {
                    return Some(ChatInputAreaPointerTarget::PaneItem(index));
                }
            }
            ChatInputAreaHeightEntryView::PlanProgress(_) => {
                if allocation.area.contains((column, row).into()) {
                    return Some(ChatInputAreaPointerTarget::PlanProgress);
                }
            }
            ChatInputAreaHeightEntryView::Pane(_)
            | ChatInputAreaHeightEntryView::Queue(_)
            | ChatInputAreaHeightEntryView::Steer(_) => {}
        }
    }

    let index = match view.overlay()? {
        ChatInputAreaOverlayView::Suggest(view) => {
            chat_input::suggest_index_at(overlay_area, Some(view), column, row)
        }
        ChatInputAreaOverlayView::Approval(view) => {
            let area = bottom_anchored_area(overlay_area, approval::desired_height(view));
            approval::choice_index_at(area, view, column, row)
        }
        ChatInputAreaOverlayView::Query(view) => {
            let area = bottom_anchored_area(overlay_area, query::desired_height(view));
            query::choice_index_at(area, view, column, row)
        }
    }?;
    Some(ChatInputAreaPointerTarget::OverlayItem(index))
}

fn height_entries(
    view: &ChatInputAreaView<'_>,
    available_width: u16,
) -> Vec<(ChatInputAreaHeightEntryKind, u16)> {
    view.height_entries()
        .into_iter()
        .map(|entry| {
            let kind = entry.kind();
            let height = match entry {
                ChatInputAreaHeightEntryView::Pane(view) => {
                    let body_height = match view {
                        PaneEntryView::DetailList(view) => view.body().desired_height(),
                        PaneEntryView::KeyCapture(view) => view.body().desired_height(),
                        PaneEntryView::ListSelection(view) => {
                            view.body().desired_height(available_width)
                        }
                        PaneEntryView::TextPrompt(view) => view.body().desired_height(),
                    };
                    pane::desired_height(body_height)
                }
                ChatInputAreaHeightEntryView::PlanProgress(view) => {
                    plan_progress::desired_height(view)
                }
                ChatInputAreaHeightEntryView::Queue(view) => queue::desired_height(view),
                ChatInputAreaHeightEntryView::Steer(view) => steer::desired_height(&view),
            };
            (kind, height)
        })
        .collect()
}

pub(crate) fn desired_height(
    input_height: u16,
    height_entries: &[(ChatInputAreaHeightEntryKind, u16)],
) -> u16 {
    height_entries
        .iter()
        .fold(input_height, |height, (_, entry_height)| {
            height.saturating_add(*entry_height)
        })
}

pub(crate) fn areas(
    area: Rect,
    input_desired_height: u16,
    height_entries: &[(ChatInputAreaHeightEntryKind, u16)],
) -> ChatInputAreaAreas {
    let input_height = input_desired_height.min(area.height);
    let input_y = area
        .y
        .saturating_add(area.height)
        .saturating_sub(input_height);
    let input = Rect {
        y: input_y,
        height: input_height,
        ..area
    };
    let mut next_y = input_y;
    let mut remaining_height = area.height.saturating_sub(input_height);
    let mut allocated = Vec::with_capacity(height_entries.len());
    for (kind, desired_height) in height_entries {
        let height = (*desired_height).min(remaining_height);
        next_y = next_y.saturating_sub(height);
        remaining_height = remaining_height.saturating_sub(height);
        allocated.push(ChatInputAreaHeightEntryArea {
            kind: *kind,
            area: Rect {
                y: next_y,
                height,
                ..area
            },
        });
    }

    ChatInputAreaAreas {
        height_entries: allocated,
        input,
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
