use crate::{Color, Rect};

/// Immutable SVG bytes that identify one reusable symbolic icon.
///
/// Renderers interpret the SVG as an alpha mask and apply the paint color at draw time.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SvgIcon {
    name: &'static str,
    data: &'static [u8],
}

impl SvgIcon {
    pub const fn new(name: &'static str, data: &'static [u8]) -> Self {
        Self { name, data }
    }

    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn data(self) -> &'static [u8] {
        self.data
    }
}

/// A theme-colored SVG icon placed in logical UI coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintIcon {
    icon: SvgIcon,
    bounds: Rect,
    color: Color,
    clip_bounds: Option<Rect>,
}

impl PaintIcon {
    pub const fn new(icon: SvgIcon, bounds: Rect, color: Color) -> Self {
        Self {
            icon,
            bounds,
            color,
            clip_bounds: None,
        }
    }

    pub const fn icon(self) -> SvgIcon {
        self.icon
    }

    pub const fn bounds(self) -> Rect {
        self.bounds
    }

    pub const fn color(self) -> Color {
        self.color
    }

    pub(crate) const fn clip_bounds(self) -> Option<Rect> {
        self.clip_bounds
    }

    pub(crate) fn apply_clip(&mut self, clip_bounds: Rect) {
        self.clip_bounds = Some(match self.clip_bounds {
            Some(current) => current.intersection(clip_bounds),
            None => clip_bounds,
        });
    }
}
