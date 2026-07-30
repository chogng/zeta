use crate::{Point, Rect};

use super::{ScrollAxis, ScrollCommand};

/// Resolved track and thumb geometry for one scrollbar axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarLayout {
    pub(super) axis: ScrollAxis,
    pub(super) track_bounds: Rect,
    pub(super) thumb_bounds: Rect,
}

impl ScrollbarLayout {
    pub const fn axis(self) -> ScrollAxis {
        self.axis
    }

    pub const fn track_bounds(self) -> Rect {
        self.track_bounds
    }

    pub const fn thumb_bounds(self) -> Rect {
        self.thumb_bounds
    }
}

/// Semantic region under a pointer within one resolved scrollbar.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ScrollbarPart {
    Track,
    Thumb,
}

/// Axis and semantic part returned by [`super::ScrollView::hit_test_scrollbar`].
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ScrollbarHit {
    pub(super) axis: ScrollAxis,
    pub(super) part: ScrollbarPart,
}

impl ScrollbarHit {
    pub const fn axis(self) -> ScrollAxis {
        self.axis
    }

    pub const fn part(self) -> ScrollbarPart {
        self.part
    }
}

/// Stable mapping captured when a scrollbar thumb drag begins.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarDrag {
    pub(super) axis: ScrollAxis,
    pub(super) track_origin: f32,
    pub(super) available_travel: f32,
    pub(super) grab_offset: f32,
    pub(super) maximum_offset: f32,
    pub(super) starting_offset: Point,
}

impl ScrollbarDrag {
    pub const fn axis(self) -> ScrollAxis {
        self.axis
    }

    /// Maps a pointer position back into an absolute logical content offset.
    pub fn command_at(self, point: Point) -> ScrollCommand {
        assert!(
            point.x.is_finite() && point.y.is_finite(),
            "Scrollbar pointer position must be finite"
        );
        let pointer = axis_coordinate(self.axis, point);
        let thumb_offset =
            (pointer - self.track_origin - self.grab_offset).clamp(0.0, self.available_travel);
        let content_offset = if self.available_travel > 0.0 {
            thumb_offset / self.available_travel * self.maximum_offset
        } else {
            0.0
        };
        let offset = match self.axis {
            ScrollAxis::Horizontal => Point::new(content_offset, self.starting_offset.y),
            ScrollAxis::Vertical => Point::new(self.starting_offset.x, content_offset),
            ScrollAxis::Both => self.starting_offset,
        };
        ScrollCommand::ToOffset(offset)
    }
}

pub(super) fn axis_coordinate(axis: ScrollAxis, point: Point) -> f32 {
    match axis {
        ScrollAxis::Horizontal => point.x,
        ScrollAxis::Vertical => point.y,
        ScrollAxis::Both => 0.0,
    }
}

pub(super) fn axis_origin(axis: ScrollAxis, bounds: Rect) -> f32 {
    axis_coordinate(axis, bounds.origin)
}

pub(super) fn axis_extent(axis: ScrollAxis, bounds: Rect) -> f32 {
    match axis {
        ScrollAxis::Horizontal => bounds.size.width,
        ScrollAxis::Vertical => bounds.size.height,
        ScrollAxis::Both => 0.0,
    }
}
