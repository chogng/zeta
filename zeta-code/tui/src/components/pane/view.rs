use super::PaneBodyView;
use super::PaneView;
use crate::components::key_capture;
use crate::components::key_hint_bar;
use crate::components::list_selection;
use crate::components::text_prompt;
use crate::render::RenderContext;
use ratatui::Frame;
use ratatui::layout::Rect;

const BODY_KEY_HINT_GAP: u16 = 2;
const KEY_HINT_BAR_HEIGHT: u16 = 1;

pub(crate) struct PaneAreas {
    pub(crate) body: Rect,
    pub(crate) key_hint_bar: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PanePointerTarget {
    Tab(usize),
    Item(usize),
}

pub(crate) fn view_desired_height(view: PaneView<'_>, available_width: u16) -> u16 {
    let body_height = match view.body() {
        PaneBodyView::KeyCapture(body) => body.desired_height(),
        PaneBodyView::ListSelection(body) => body.desired_height(available_width),
        PaneBodyView::TextPrompt(body) => body.desired_height(),
    };
    desired_height(body_height)
}

pub(crate) fn draw(
    frame: &mut Frame<'_>,
    area: Rect,
    view: PaneView<'_>,
    hovered: Option<PanePointerTarget>,
    context: RenderContext<'_>,
) {
    let pane_areas = areas(area);
    match view.body() {
        PaneBodyView::KeyCapture(body) => key_capture::draw(frame, pane_areas.body, body, context),
        PaneBodyView::ListSelection(body) => {
            let hovered_item = match hovered {
                Some(PanePointerTarget::Item(index)) => Some(index),
                Some(PanePointerTarget::Tab(_)) | None => None,
            };
            list_selection::draw_with_hover(frame, pane_areas.body, body, hovered_item, context)
        }
        PaneBodyView::TextPrompt(body) => text_prompt::draw(frame, pane_areas.body, body, context),
    }
    key_hint_bar::draw(frame, pane_areas.key_hint_bar, view.key_hints(), context);
}

pub(crate) fn pointer_target_at(
    area: Rect,
    view: PaneView<'_>,
    column: u16,
    row: u16,
) -> Option<PanePointerTarget> {
    let PaneBodyView::ListSelection(body) = view.body() else {
        return None;
    };
    let body_area = areas(area).body;
    body.tab_index_at(body_area, column, row)
        .map(PanePointerTarget::Tab)
        .or_else(|| {
            body.item_index_at(body_area, column, row)
                .map(PanePointerTarget::Item)
        })
}

pub(crate) fn desired_height(body_height: u16) -> u16 {
    body_height
        .saturating_add(BODY_KEY_HINT_GAP)
        .saturating_add(KEY_HINT_BAR_HEIGHT)
}

pub(crate) fn areas(area: Rect) -> PaneAreas {
    let key_hint_bar_height = KEY_HINT_BAR_HEIGHT.min(area.height);
    let remaining = area.height.saturating_sub(key_hint_bar_height);
    let gap_height = BODY_KEY_HINT_GAP.min(remaining);
    let body_height = remaining.saturating_sub(gap_height);
    PaneAreas {
        body: Rect {
            height: body_height,
            ..area
        },
        key_hint_bar: Rect {
            y: area
                .y
                .saturating_add(area.height)
                .saturating_sub(key_hint_bar_height),
            height: key_hint_bar_height,
            ..area
        },
    }
}

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;
