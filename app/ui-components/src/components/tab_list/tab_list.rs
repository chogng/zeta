use crate::{
    Border, Color, Component, ComponentElement, ComputedElement, CornerRadii, Element,
    ElementLength, PaintRect, Rect, Size, UiScene,
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
/// A `Tab` deliberately carries no product content, identity, or inspection node. Hosts that own
/// tab semantics compose one identified component per item inside the bounds returned by
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
        self.element_tree()
            .compute()
            .child(index)
            .map(ComputedElement::bounds)
    }

    /// Paints the list's state-dependent tab surfaces without registering item components.
    ///
    /// Product hosts use this under their identified list root, then compose one semantic
    /// component for each visible tab.
    pub fn paint_surfaces(&self, scene: &mut UiScene) {
        self.paint_layout(scene, &self.element_tree().compute());
    }

    fn element_tree(&self) -> ComponentElement {
        let children = self.tabs.iter().map(|_| {
            Element::row("Tab")
                .width(ElementLength::px(self.style.tab_size.width))
                .height(ElementLength::px(self.style.tab_size.height))
        });
        match self.orientation {
            TabListOrientation::Horizontal => Element::row("TabList"),
            TabListOrientation::Vertical => Element::column("TabList"),
        }
        .gap(self.style.gap)
        .children(children)
        .in_bounds(self.bounds)
    }

    fn paint_layout(&self, scene: &mut UiScene, layout: &ComputedElement) {
        scene.with_clip(self.bounds, |scene| {
            for (index, tab) in self.tabs.iter().copied().enumerate() {
                let Some(bounds) = layout.child(index).map(ComputedElement::bounds) else {
                    continue;
                };
                tab.paint(bounds, self.style.tab_style, scene);
            }
        });
    }
}

impl Component for TabList {
    fn element(&self) -> ComponentElement {
        self.element_tree()
    }

    fn paint_element(&self, scene: &mut UiScene, element: &ComputedElement) {
        self.paint_layout(scene, element);
    }

    fn paint(&self, scene: &mut UiScene) {
        self.paint_surfaces(scene);
    }
}

#[cfg(test)]
#[path = "tab_list_tests.rs"]
mod tests;
