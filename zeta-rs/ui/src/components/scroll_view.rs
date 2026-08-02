use crate::{
    Color, Component, ComponentElement, CornerRadii, Element, PaintRect, Point, Rect, Size, UiScene,
};

mod geometry;
mod interaction;
mod state;

pub use geometry::{ScrollbarDrag, ScrollbarHit, ScrollbarLayout, ScrollbarPart};
use geometry::{axis_coordinate, axis_extent, axis_origin};
pub use interaction::{
    ScrollbarController, ScrollbarPointerPresence, ScrollbarPresentation, ScrollbarState,
};
pub use state::{ScrollAxis, ScrollCommand, ScrollDelta, ScrollMetrics, ScrollState};
use state::{assert_rect, assert_size};

/// Policy for painting one axis scrollbar.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ScrollbarVisibility {
    Hidden,
    Always,
    #[default]
    Automatic,
}

/// Component-owned visual and geometry tokens for scrollbars.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarStyle {
    track: Color,
    hovered_track: Color,
    active_track: Color,
    thumb: Color,
    hovered_thumb: Color,
    active_thumb: Color,
    thickness: f32,
    inset: f32,
    minimum_thumb_extent: f32,
    corner_radii: CornerRadii,
}

impl ScrollbarStyle {
    pub const fn new(track: Color, thumb: Color) -> Self {
        Self {
            track,
            hovered_track: track,
            active_track: track,
            thumb,
            hovered_thumb: thumb,
            active_thumb: thumb,
            thickness: 8.0,
            inset: 2.0,
            minimum_thumb_extent: 24.0,
            corner_radii: CornerRadii::uniform(4.0),
        }
    }

    pub const fn with_hovered_colors(mut self, track: Color, thumb: Color) -> Self {
        self.hovered_track = track;
        self.hovered_thumb = thumb;
        self
    }

    pub const fn with_active_colors(mut self, track: Color, thumb: Color) -> Self {
        self.active_track = track;
        self.active_thumb = thumb;
        self
    }

    pub const fn with_thickness(mut self, thickness: f32) -> Self {
        self.thickness = thickness;
        self
    }

    pub const fn with_inset(mut self, inset: f32) -> Self {
        self.inset = inset;
        self
    }

    pub const fn with_minimum_thumb_extent(mut self, extent: f32) -> Self {
        self.minimum_thumb_extent = extent;
        self
    }

    pub const fn with_corner_radii(mut self, corner_radii: CornerRadii) -> Self {
        self.corner_radii = corner_radii;
        self
    }
}

/// Scrollbar visibility and presentation for a [`ScrollView`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollViewStyle {
    scrollbar: ScrollbarStyle,
    visibility: ScrollbarVisibility,
}

impl ScrollViewStyle {
    pub const fn new(scrollbar: ScrollbarStyle) -> Self {
        Self {
            scrollbar,
            visibility: ScrollbarVisibility::Automatic,
        }
    }

    pub const fn with_visibility(mut self, visibility: ScrollbarVisibility) -> Self {
        self.visibility = visibility;
        self
    }
}

/// Geometry passed to arbitrary content hosted by [`ScrollView::draw`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollViewport {
    bounds: Rect,
    content_origin: Point,
    visible_content_bounds: Rect,
}

impl ScrollViewport {
    pub const fn bounds(self) -> Rect {
        self.bounds
    }

    pub const fn content_origin(self) -> Point {
        self.content_origin
    }

    pub const fn visible_content_bounds(self) -> Rect {
        self.visible_content_bounds
    }
}

/// Clipping viewport with shared scrollbar geometry, presentation, and pointer mapping.
///
/// The host retains [`ScrollState`], normalizes platform input into [`ScrollCommand`], computes
/// content size, and owns scrollbar interaction routing and pointer capture. ScrollView owns
/// effective offset clamping, content translation, clipping, scrollbar geometry and paint, hit
/// testing, track paging commands, and thumb-drag coordinate mapping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollView {
    bounds: Rect,
    content_size: Size,
    state: ScrollState,
    axis: ScrollAxis,
    style: ScrollViewStyle,
    scrollbar_presentation: ScrollbarPresentation,
}

impl ScrollView {
    pub fn new(
        bounds: Rect,
        content_size: Size,
        state: ScrollState,
        axis: ScrollAxis,
        style: ScrollViewStyle,
    ) -> Self {
        assert_rect(bounds);
        assert_size(content_size, "Scroll content");
        Self {
            bounds,
            content_size,
            state,
            axis,
            style,
            scrollbar_presentation: ScrollbarPresentation::default(),
        }
    }

    pub const fn with_scrollbar_presentation(
        mut self,
        presentation: ScrollbarPresentation,
    ) -> Self {
        self.scrollbar_presentation = presentation;
        self
    }

    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    pub fn metrics(&self) -> ScrollMetrics {
        ScrollMetrics::new(self.bounds.size, self.content_size)
    }

    pub fn viewport(&self) -> ScrollViewport {
        let offset = self.effective_offset();
        ScrollViewport {
            bounds: self.bounds,
            content_origin: Point::new(
                self.bounds.origin.x - offset.x,
                self.bounds.origin.y - offset.y,
            ),
            visible_content_bounds: Rect::from_xywh(
                offset.x,
                offset.y,
                self.bounds.size.width,
                self.bounds.size.height,
            ),
        }
    }

    pub fn vertical_scrollbar(&self) -> Option<ScrollbarLayout> {
        self.scrollbar(ScrollAxis::Vertical)
    }

    pub fn horizontal_scrollbar(&self) -> Option<ScrollbarLayout> {
        self.scrollbar(ScrollAxis::Horizontal)
    }

    /// Resolves whether a point lies over a thumb or the remainder of its track.
    pub fn hit_test_scrollbar(&self, point: Point) -> Option<ScrollbarHit> {
        assert!(
            point.x.is_finite() && point.y.is_finite(),
            "Scrollbar pointer position must be finite"
        );
        for layout in [self.vertical_scrollbar(), self.horizontal_scrollbar()]
            .into_iter()
            .flatten()
        {
            if layout.thumb_bounds.contains(point) {
                return Some(ScrollbarHit {
                    axis: layout.axis,
                    part: ScrollbarPart::Thumb,
                });
            }
            if layout.track_bounds.contains(point) {
                return Some(ScrollbarHit {
                    axis: layout.axis,
                    part: ScrollbarPart::Track,
                });
            }
        }
        None
    }

    /// Captures the pointer-to-thumb relationship used throughout a drag.
    pub fn begin_scrollbar_drag(&self, hit: ScrollbarHit, point: Point) -> Option<ScrollbarDrag> {
        if hit.part != ScrollbarPart::Thumb {
            return None;
        }
        let layout = self.scrollbar(hit.axis)?;
        let track_origin = axis_origin(hit.axis, layout.track_bounds);
        let thumb_origin = axis_origin(hit.axis, layout.thumb_bounds);
        let track_extent = axis_extent(hit.axis, layout.track_bounds);
        let thumb_extent = axis_extent(hit.axis, layout.thumb_bounds);
        let maximum = self.metrics().maximum_offset();
        Some(ScrollbarDrag {
            axis: hit.axis,
            track_origin,
            available_travel: (track_extent - thumb_extent).max(0.0),
            grab_offset: axis_coordinate(hit.axis, point) - thumb_origin,
            maximum_offset: match hit.axis {
                ScrollAxis::Horizontal => maximum.x,
                ScrollAxis::Vertical => maximum.y,
                ScrollAxis::Both => 0.0,
            },
            starting_offset: self.effective_offset(),
        })
    }

    /// Returns a one-viewport page command for a click before or after the thumb.
    pub fn track_click_command(&self, hit: ScrollbarHit, point: Point) -> Option<ScrollCommand> {
        if hit.part != ScrollbarPart::Track {
            return None;
        }
        let layout = self.scrollbar(hit.axis)?;
        let pointer = axis_coordinate(hit.axis, point);
        let thumb_start = axis_origin(hit.axis, layout.thumb_bounds);
        let thumb_end = thumb_start + axis_extent(hit.axis, layout.thumb_bounds);
        let direction = if pointer < thumb_start {
            -1.0
        } else if pointer >= thumb_end {
            1.0
        } else {
            return None;
        };
        let viewport = self.metrics().viewport();
        Some(ScrollCommand::ByPixels(match hit.axis {
            ScrollAxis::Horizontal => ScrollDelta::horizontal(direction * viewport.width),
            ScrollAxis::Vertical => ScrollDelta::vertical(direction * viewport.height),
            ScrollAxis::Both => return None,
        }))
    }

    /// Clips arbitrary content to the viewport, then paints scrollbars above it.
    pub fn draw<R>(
        &self,
        scene: &mut UiScene,
        draw_content: impl FnOnce(&mut UiScene, ScrollViewport) -> R,
    ) -> R {
        scene.with_element(self.element_tree(), |scene, _element| {
            let result = scene.with_clip(self.bounds, |scene| draw_content(scene, self.viewport()));
            self.paint(scene);
            result
        })
    }

    fn element_tree(&self) -> ComponentElement {
        Element::leaf("ScrollView").in_bounds(self.bounds)
    }

    fn effective_offset(&self) -> Point {
        let maximum = self.metrics().maximum_offset();
        Point::new(
            if self.axis.permits_horizontal() {
                self.state.horizontal_offset().clamp(0.0, maximum.x)
            } else {
                0.0
            },
            if self.axis.permits_vertical() {
                self.state.vertical_offset().clamp(0.0, maximum.y)
            } else {
                0.0
            },
        )
    }

    fn scrollbar(&self, axis: ScrollAxis) -> Option<ScrollbarLayout> {
        let maximum = self.metrics().maximum_offset();
        let permitted = match axis {
            ScrollAxis::Horizontal => self.axis.permits_horizontal(),
            ScrollAxis::Vertical => self.axis.permits_vertical(),
            ScrollAxis::Both => false,
        };
        let maximum_offset = match axis {
            ScrollAxis::Horizontal => maximum.x,
            ScrollAxis::Vertical => maximum.y,
            ScrollAxis::Both => 0.0,
        };
        let visible = permitted
            && match self.style.visibility {
                ScrollbarVisibility::Hidden => false,
                ScrollbarVisibility::Always => true,
                ScrollbarVisibility::Automatic => maximum_offset > 0.0,
            };
        if !visible {
            return None;
        }
        let style = self.style.scrollbar;
        let inset = style.inset.max(0.0);
        let thickness = style.thickness.max(0.0);
        let track_bounds = match axis {
            ScrollAxis::Vertical => Rect::from_xywh(
                self.bounds.right() - inset - thickness,
                self.bounds.origin.y + inset,
                thickness,
                (self.bounds.size.height - inset * 2.0).max(0.0),
            ),
            ScrollAxis::Horizontal => Rect::from_xywh(
                self.bounds.origin.x + inset,
                self.bounds.bottom() - inset - thickness,
                (self.bounds.size.width - inset * 2.0).max(0.0),
                thickness,
            ),
            ScrollAxis::Both => return None,
        };
        let (track_extent, viewport_extent, content_extent, offset) = match axis {
            ScrollAxis::Horizontal => (
                track_bounds.size.width,
                self.bounds.size.width,
                self.content_size.width,
                self.effective_offset().x,
            ),
            ScrollAxis::Vertical => (
                track_bounds.size.height,
                self.bounds.size.height,
                self.content_size.height,
                self.effective_offset().y,
            ),
            ScrollAxis::Both => return None,
        };
        let thumb_extent = if content_extent <= 0.0 {
            track_extent
        } else {
            (track_extent * viewport_extent / content_extent)
                .max(style.minimum_thumb_extent.max(0.0))
                .min(track_extent)
        };
        let available_travel = (track_extent - thumb_extent).max(0.0);
        let thumb_offset = if maximum_offset > 0.0 {
            offset / maximum_offset * available_travel
        } else {
            0.0
        };
        let thumb_bounds = match axis {
            ScrollAxis::Vertical => Rect::from_xywh(
                track_bounds.origin.x,
                track_bounds.origin.y + thumb_offset,
                track_bounds.size.width,
                thumb_extent,
            ),
            ScrollAxis::Horizontal => Rect::from_xywh(
                track_bounds.origin.x + thumb_offset,
                track_bounds.origin.y,
                thumb_extent,
                track_bounds.size.height,
            ),
            ScrollAxis::Both => return None,
        };
        Some(ScrollbarLayout {
            axis,
            track_bounds,
            thumb_bounds,
        })
    }
}

impl Component for ScrollView {
    fn element(&self) -> ComponentElement {
        self.element_tree()
    }

    fn paint(&self, scene: &mut UiScene) {
        let opacity = self.scrollbar_presentation.opacity();
        if opacity <= 0.0 {
            return;
        }
        let (track, thumb) = match self.scrollbar_presentation.state() {
            ScrollbarState::Resting => (self.style.scrollbar.track, self.style.scrollbar.thumb),
            ScrollbarState::Hovered => (
                self.style.scrollbar.hovered_track,
                self.style.scrollbar.hovered_thumb,
            ),
            ScrollbarState::Active => (
                self.style.scrollbar.active_track,
                self.style.scrollbar.active_thumb,
            ),
        };
        for scrollbar in [self.horizontal_scrollbar(), self.vertical_scrollbar()]
            .into_iter()
            .flatten()
        {
            scene.draw_rect(
                PaintRect::new(scrollbar.track_bounds, color_with_opacity(track, opacity))
                    .with_corner_radii(self.style.scrollbar.corner_radii),
            );
            scene.draw_rect(
                PaintRect::new(scrollbar.thumb_bounds, color_with_opacity(thumb, opacity))
                    .with_corner_radii(self.style.scrollbar.corner_radii),
            );
        }
    }
}

fn color_with_opacity(color: Color, opacity: f32) -> Color {
    let [red, green, blue, alpha] = color.components();
    Color::rgba(
        red,
        green,
        blue,
        (f32::from(alpha) * opacity.clamp(0.0, 1.0)).round() as u8,
    )
}

#[cfg(test)]
#[path = "scroll_view_tests.rs"]
mod tests;
