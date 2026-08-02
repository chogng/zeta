use ratatui::layout::Rect;

const BODY_KEY_HINT_GAP: u16 = 2;
const KEY_HINT_BAR_HEIGHT: u16 = 1;

pub(crate) struct PaneAreas {
    pub(crate) body: Rect,
    pub(crate) key_hint_bar: Rect,
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
