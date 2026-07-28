use ratatui::layout::Rect;
use ratatui::layout::{Constraint, Direction, Layout};

pub(super) struct FrameAreas {
    pub(super) header: Rect,
    pub(super) history: Rect,
    pub(super) composer: Rect,
    pub(super) footer: Rect,
}

pub(super) fn frame_areas(area: Rect) -> FrameAreas {
    let areas = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(4),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(area);
    FrameAreas {
        header: areas[0],
        history: areas[1],
        composer: areas[2],
        footer: areas[3],
    }
}

pub(super) fn horizontal_margin(area: Rect, margin: u16) -> Rect {
    Rect {
        x: area.x.saturating_add(margin),
        width: area.width.saturating_sub(margin.saturating_mul(2)),
        ..area
    }
}
