use crate::{
    Border, Color, Component, ComponentInspection, CornerRadii, PaintRect, Rect, Size, UiScene,
};

/// Axis along which a [`TabList`] arranges its tabs.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TabListOrientation {
    Horizontal,
    #[default]
    Vertical,
}

/// Visual interaction state projected onto one [`Tab`] by its host.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TabState {
    #[default]
    Resting,
    Hovered,
    Focused,
    Pressed,
}

/// Selection presentation projected onto one [`Tab`] by its host.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum TabSelection {
    #[default]
    Unselected,
    Selected,
}

/// State-dependent background colors for a tab surface.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TabBackgrounds {
    resting: Color,
    hovered: Color,
    focused: Color,
    pressed: Color,
}

impl TabBackgrounds {
    pub const fn new(resting: Color) -> Self {
        Self {
            resting,
            hovered: resting,
            focused: resting,
            pressed: resting,
        }
    }

    pub const fn with_hovered(mut self, hovered: Color) -> Self {
        self.hovered = hovered;
        self
    }

    pub const fn with_focused(mut self, focused: Color) -> Self {
        self.focused = focused;
        self
    }

    pub const fn with_pressed(mut self, pressed: Color) -> Self {
        self.pressed = pressed;
        self
    }

    const fn for_state(self, state: TabState) -> Color {
        match state {
            TabState::Resting => self.resting,
            TabState::Hovered => self.hovered,
            TabState::Focused => self.focused,
            TabState::Pressed => self.pressed,
        }
    }
}

/// Shared surface style for the tabs arranged by a [`TabList`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabStyle {
    backgrounds: TabBackgrounds,
    selected_backgrounds: TabBackgrounds,
    border: Border,
    corner_radii: CornerRadii,
}

impl TabStyle {
    pub const fn new(backgrounds: TabBackgrounds) -> Self {
        Self {
            backgrounds,
            selected_backgrounds: backgrounds,
            border: Border::uniform(0.0, Color::TRANSPARENT),
            corner_radii: CornerRadii::uniform(0.0),
        }
    }

    pub const fn with_selected_backgrounds(mut self, selected_backgrounds: TabBackgrounds) -> Self {
        self.selected_backgrounds = selected_backgrounds;
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

    const fn backgrounds_for(self, selection: TabSelection) -> TabBackgrounds {
        match selection {
            TabSelection::Unselected => self.backgrounds,
            TabSelection::Selected => self.selected_backgrounds,
        }
    }
}

/// Presentation state for one selectable surface inside a [`TabList`].
///
/// A `Tab` deliberately carries no product content or identity. Composed controls paint their
/// labels, icons, status indicators, and close actions inside the bounds returned by
/// [`TabList::tab_bounds`].
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct Tab {
    state: TabState,
    selection: TabSelection,
}

impl Tab {
    pub const fn new(state: TabState) -> Self {
        Self {
            state,
            selection: TabSelection::Unselected,
        }
    }

    pub const fn with_selection(mut self, selection: TabSelection) -> Self {
        self.selection = selection;
        self
    }

    fn paint(self, bounds: Rect, style: TabStyle, scene: &mut UiScene) {
        scene.draw_rect(
            PaintRect::new(
                bounds,
                style.backgrounds_for(self.selection).for_state(self.state),
            )
            .with_border(style.border)
            .with_corner_radii(style.corner_radii),
        );
    }
}

/// Geometry and shared surface style for a [`TabList`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TabListStyle {
    tab_style: TabStyle,
    tab_size: Size,
    gap: f32,
}

impl TabListStyle {
    pub const fn new(tab_style: TabStyle, tab_size: Size) -> Self {
        Self {
            tab_style,
            tab_size,
            gap: 0.0,
        }
    }

    pub const fn with_gap(mut self, gap: f32) -> Self {
        self.gap = gap;
        self
    }
}

/// Presentation-only tab collection with component-owned arrangement and surface paint.
///
/// The host retains tab identity, activation, focus, accessibility, active panel ownership, and
/// product content. It must use [`TabList::tab_bounds`] for both interaction registration and
/// composed tab content so paint and hit geometry remain aligned.
#[derive(Clone, Debug, PartialEq)]
pub struct TabList {
    bounds: Rect,
    orientation: TabListOrientation,
    tabs: Vec<Tab>,
    style: TabListStyle,
}

impl TabList {
    pub fn new(
        bounds: Rect,
        orientation: TabListOrientation,
        tabs: Vec<Tab>,
        style: TabListStyle,
    ) -> Self {
        Self {
            bounds,
            orientation,
            tabs,
            style,
        }
    }

    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Returns the clipped visual bounds for a tab, or `None` when the index is absent.
    pub fn tab_bounds(&self, index: usize) -> Option<Rect> {
        self.tabs.get(index)?;
        let gap = self.style.gap.max(0.0);
        let extent = match self.orientation {
            TabListOrientation::Horizontal => self.style.tab_size.width.max(0.0),
            TabListOrientation::Vertical => self.style.tab_size.height.max(0.0),
        };
        let offset = index as f32 * (extent + gap);
        Some(match self.orientation {
            TabListOrientation::Horizontal => Rect::from_xywh(
                self.bounds.origin.x + offset,
                self.bounds.origin.y,
                extent.min((self.bounds.size.width - offset).max(0.0)),
                self.style
                    .tab_size
                    .height
                    .max(0.0)
                    .min(self.bounds.size.height.max(0.0)),
            ),
            TabListOrientation::Vertical => Rect::from_xywh(
                self.bounds.origin.x,
                self.bounds.origin.y + offset,
                self.style
                    .tab_size
                    .width
                    .max(0.0)
                    .min(self.bounds.size.width.max(0.0)),
                extent.min((self.bounds.size.height - offset).max(0.0)),
            ),
        })
    }
}

impl Component for TabList {
    fn inspection(&self) -> ComponentInspection {
        ComponentInspection::new("TabList", self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.with_clip(self.bounds, |scene| {
            for (index, tab) in self.tabs.iter().copied().enumerate() {
                let Some(bounds) = self.tab_bounds(index) else {
                    continue;
                };
                scene.draw_component(&TabSurface {
                    tab,
                    bounds,
                    style: self.style.tab_style,
                });
            }
        });
    }
}

struct TabSurface {
    tab: Tab,
    bounds: Rect,
    style: TabStyle,
}

impl Component for TabSurface {
    fn inspection(&self) -> ComponentInspection {
        ComponentInspection::new("Tab", self.bounds).with_corner_radii(self.style.corner_radii)
    }

    fn paint(&self, scene: &mut UiScene) {
        self.tab.paint(self.bounds, self.style, scene);
    }
}

#[cfg(test)]
#[path = "tab_list_tests.rs"]
mod tests;
