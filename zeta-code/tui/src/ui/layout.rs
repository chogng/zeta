use ratatui::layout::Rect;

const MIN_HISTORY_HEIGHT: u16 = 4;
const STATUS_LINE_HEIGHT: u16 = 1;
const FOOTER_HEIGHT: u16 = 1;

pub(crate) struct FrameAreas {
    pub(crate) history: Rect,
    pub(crate) status_line: Rect,
    pub(crate) interaction: Rect,
    pub(crate) footer: Rect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InteractionLayout {
    Composer { desired_height: u16 },
    Expanded { desired_height: u16 },
}

pub(crate) fn frame_areas(area: Rect, interaction_layout: InteractionLayout) -> FrameAreas {
    let available_height = area.height;
    let (requested_status_line_height, requested_interaction_height, requested_footer_height) =
        match interaction_layout {
            InteractionLayout::Composer { desired_height } => {
                (STATUS_LINE_HEIGHT, desired_height.max(3), FOOTER_HEIGHT)
            }
            InteractionLayout::Expanded { desired_height } => (0, desired_height, 0),
        };
    let footer_height = requested_footer_height.min(available_height);
    let available_above_footer = available_height.saturating_sub(footer_height);
    let history_height = MIN_HISTORY_HEIGHT.min(available_above_footer);
    let interaction_height =
        requested_interaction_height.min(available_above_footer.saturating_sub(history_height));
    let status_line_height = requested_status_line_height.min(
        available_above_footer
            .saturating_sub(history_height)
            .saturating_sub(interaction_height),
    );

    let bottom = area.y.saturating_add(area.height);
    let footer_y = bottom.saturating_sub(footer_height);
    let interaction_y = footer_y.saturating_sub(interaction_height);
    let status_line_y = interaction_y.saturating_sub(status_line_height);
    let history_y = area.y;

    FrameAreas {
        history: Rect {
            y: history_y,
            height: status_line_y.saturating_sub(history_y),
            ..area
        },
        status_line: Rect {
            y: status_line_y,
            height: status_line_height,
            ..area
        },
        interaction: Rect {
            y: interaction_y,
            height: interaction_height,
            ..area
        },
        footer: Rect {
            y: footer_y,
            height: footer_height,
            ..area
        },
    }
}

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
