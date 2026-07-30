use crate::{CornerRadii, Edges, Point, Rect};

/// An sRGB color with straight alpha.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
    alpha: u8,
}

impl Color {
    pub const TRANSPARENT: Self = Self::rgba(0, 0, 0, 0);
    pub const WHITE: Self = Self::rgb(255, 255, 255);

    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self::rgba(red, green, blue, 255)
    }

    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub const fn components(self) -> [u8; 4] {
        [self.red, self.green, self.blue, self.alpha]
    }
}

/// A colored border with independently configurable edge widths.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Border {
    widths: Edges,
    color: Color,
}

impl Border {
    pub const fn new(widths: Edges, color: Color) -> Self {
        Self { widths, color }
    }

    pub const fn uniform(width: f32, color: Color) -> Self {
        Self::new(Edges::uniform(width), color)
    }

    pub const fn widths(self) -> Edges {
        self.widths
    }

    pub const fn color(self) -> Color {
        self.color
    }
}

impl Default for Border {
    fn default() -> Self {
        Self::uniform(0.0, Color::TRANSPARENT)
    }
}

/// Soft shadow cast by a rounded rectangular paint primitive.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoxShadow {
    color: Color,
    offset: Point,
    blur_radius: f32,
}

impl BoxShadow {
    pub const fn new(color: Color) -> Self {
        Self {
            color,
            offset: Point::new(0.0, 0.0),
            blur_radius: 0.0,
        }
    }

    pub const fn with_offset(mut self, offset: Point) -> Self {
        self.offset = offset;
        self
    }

    pub const fn with_blur_radius(mut self, blur_radius: f32) -> Self {
        self.blur_radius = blur_radius;
        self
    }

    pub const fn color(self) -> Color {
        self.color
    }

    pub const fn offset(self) -> Point {
        self.offset
    }

    pub const fn blur_radius(self) -> f32 {
        self.blur_radius
    }
}

/// A filled rectangular paint primitive with an optional shadow, border, and rounded corners.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintRect {
    bounds: Rect,
    fill: Color,
    shadow: Option<BoxShadow>,
    border: Border,
    corner_radii: CornerRadii,
    clip_bounds: Option<Rect>,
}

impl PaintRect {
    pub const fn new(bounds: Rect, fill: Color) -> Self {
        Self {
            bounds,
            fill,
            shadow: None,
            border: Border::uniform(0.0, Color::TRANSPARENT),
            corner_radii: CornerRadii::uniform(0.0),
            clip_bounds: None,
        }
    }

    pub const fn with_shadow(mut self, shadow: BoxShadow) -> Self {
        self.shadow = Some(shadow);
        self
    }

    pub const fn with_border(mut self, border: Border) -> Self {
        self.border = border;
        self
    }

    pub const fn with_corner_radii(mut self, corner_radii: CornerRadii) -> Self {
        self.corner_radii = corner_radii;
        self
    }

    pub const fn bounds(self) -> Rect {
        self.bounds
    }

    pub const fn fill(self) -> Color {
        self.fill
    }

    pub const fn shadow(self) -> Option<BoxShadow> {
        self.shadow
    }

    pub const fn border(self) -> Border {
        self.border
    }

    pub fn corner_radii(self) -> CornerRadii {
        self.corner_radii.clamped_for(self.bounds.size)
    }

    pub(crate) const fn requested_corner_radii(self) -> CornerRadii {
        self.corner_radii
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

#[cfg(test)]
#[path = "paint_tests.rs"]
mod tests;
