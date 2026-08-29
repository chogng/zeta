use ratatui::layout::Rect;

pub(crate) fn horizontal_margin(area: Rect, margin: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(margin),
        width: area.width.saturating_sub(margin.saturating_mul(2)),
        ..area
    }
}

pub(crate) fn bottom_anchored_area(area: Rect, desired_height: u16) -> Rect {
    let height = desired_height.min(area.height);
    Rect {
        y: area.y.saturating_add(area.height).saturating_sub(height),
        height,
        ..area
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
