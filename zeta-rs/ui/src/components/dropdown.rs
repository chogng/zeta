use crate::{
    Color, Component, ContextView, ContextViewPlacement, ContextViewStyle, CornerRadii, Point,
    Rect, Size, UiScene,
};

use super::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle,
    ButtonSelection, ButtonState, ButtonStyle,
};

/// One label item projected into a [`Dropdown`].
#[derive(Clone, Debug, PartialEq)]
pub struct DropdownItem {
    label: String,
    state: ButtonState,
}

impl DropdownItem {
    pub fn new(label: impl Into<String>, state: ButtonState) -> Self {
        Self {
            label: label.into(),
            state,
        }
    }

    const fn is_enabled(&self) -> bool {
        !matches!(self.state, ButtonState::Disabled)
    }
}

/// Selection policy used when a dropdown is presented.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum DropdownSelection {
    /// Selects the first enabled item. This is the default open-state behavior.
    #[default]
    FirstEnabled,
    /// Selects one item by its presentation index.
    Item(usize),
    /// Presents the dropdown without a selected item.
    None,
}

/// Shared surface, item, and anchor presentation for a [`Dropdown`].
#[derive(Clone, Debug, PartialEq)]
pub struct DropdownStyle {
    background: Color,
    corner_radii: CornerRadii,
    button_style: ButtonStyle,
    item_size: Size,
    header_height: f32,
    placement: ContextViewPlacement,
}

impl DropdownStyle {
    pub fn new(background: Color, button_style: ButtonStyle, item_size: Size) -> Self {
        Self {
            background,
            corner_radii: CornerRadii::uniform(0.0),
            button_style,
            item_size,
            header_height: 0.0,
            placement: ContextViewPlacement::new(),
        }
    }

    pub const fn with_corner_radii(mut self, corner_radii: CornerRadii) -> Self {
        self.corner_radii = corner_radii;
        self
    }

    /// Reserves a leading row that the product host can paint with [`Dropdown::paint_with_header`].
    pub const fn with_header_height(mut self, header_height: f32) -> Self {
        self.header_height = header_height;
        self
    }

    pub const fn with_placement(mut self, placement: ContextViewPlacement) -> Self {
        self.placement = placement;
        self
    }
}

/// Presentation-only anchored dropdown with a selected item and shared item geometry.
///
/// Dropdown composes [`ContextView`] placement with a vertical [`ActionBar`]. Its surface is
/// intentionally borderless and has no outer padding, so item bounds fill the floating surface.
/// The product host owns retained open state, selected identity, input routing, accessibility,
/// dismissal, and command execution.
#[derive(Clone, Debug, PartialEq)]
pub struct Dropdown {
    context_view: ContextView,
    item_bounds: Rect,
    header_bounds: Option<Rect>,
    items: Vec<DropdownItem>,
    style: DropdownStyle,
    selection: DropdownSelection,
}

impl Dropdown {
    pub fn new(
        viewport: Rect,
        anchor: Rect,
        items: Vec<DropdownItem>,
        style: DropdownStyle,
    ) -> Self {
        let desired_content_size = Size::new(
            style.item_size.width.max(0.0),
            style.header_height.max(0.0) + style.item_size.height.max(0.0) * items.len() as f32,
        );
        let context_view = ContextView::new(
            viewport,
            anchor,
            desired_content_size,
            style.placement,
            ContextViewStyle::new(style.background).with_corner_radii(style.corner_radii),
        );
        let content_bounds = context_view.content_bounds();
        let header_height = style.header_height.max(0.0).min(content_bounds.size.height);
        let header_bounds = (header_height > 0.0).then(|| {
            Rect::from_xywh(
                content_bounds.origin.x,
                content_bounds.origin.y,
                content_bounds.size.width,
                header_height,
            )
        });
        let item_bounds = Rect::from_xywh(
            content_bounds.origin.x,
            content_bounds.origin.y + header_height,
            content_bounds.size.width,
            (content_bounds.size.height - header_height).max(0.0),
        );
        Self {
            context_view,
            item_bounds,
            header_bounds,
            items,
            style,
            selection: DropdownSelection::default(),
        }
    }

    pub const fn with_selection(mut self, selection: DropdownSelection) -> Self {
        self.selection = selection;
        self
    }

    pub const fn bounds(&self) -> Rect {
        self.context_view.bounds()
    }

    pub const fn content_bounds(&self) -> Rect {
        self.context_view.content_bounds()
    }

    /// Returns the host-owned leading row, when one was reserved by the style.
    pub const fn header_bounds(&self) -> Option<Rect> {
        self.header_bounds
    }

    pub fn selected_index(&self) -> Option<usize> {
        match self.selection {
            DropdownSelection::FirstEnabled => self.items.iter().position(DropdownItem::is_enabled),
            DropdownSelection::Item(index) => self
                .items
                .get(index)
                .filter(|item| item.is_enabled())
                .map(|_| index),
            DropdownSelection::None => None,
        }
    }

    pub fn item_bounds(&self, index: usize) -> Option<Rect> {
        self.action_bar().item_bounds(index)
    }

    pub fn interactive_item_bounds(&self, index: usize) -> Option<Rect> {
        self.action_bar().interactive_item_bounds(index)
    }

    pub fn hit_test(&self, point: Point) -> Option<usize> {
        self.action_bar().hit_test(point)
    }

    fn action_bar(&self) -> ActionBar {
        let selected_index = self.selected_index();
        let items = self
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                ActionBarItem::Button(
                    ActionBarButton::label(item.label.clone(), item.state).with_selection(
                        if selected_index == Some(index) {
                            ButtonSelection::Selected
                        } else {
                            ButtonSelection::Unselected
                        },
                    ),
                )
            })
            .collect();
        ActionBar::new(
            self.item_bounds,
            ActionBarOrientation::Vertical,
            items,
            ActionBarStyle::new(self.style.button_style.clone(), self.style.item_size),
        )
    }

    /// Paints the canonical dropdown and items with product-owned content in its header row.
    pub fn paint_with_header(
        &self,
        scene: &mut UiScene,
        paint_header: impl FnOnce(&mut UiScene, Rect),
    ) {
        self.paint_contents(scene, paint_header);
    }

    fn paint_contents(&self, scene: &mut UiScene, paint_header: impl FnOnce(&mut UiScene, Rect)) {
        let action_bar = self.action_bar();
        self.context_view.draw(scene, |scene, _content_bounds| {
            if let Some(header_bounds) = self.header_bounds {
                paint_header(scene, header_bounds);
            }
            action_bar.paint(scene);
        });
    }
}

impl Component for Dropdown {
    fn paint(&self, scene: &mut UiScene) {
        self.paint_contents(scene, |_scene, _bounds| {});
    }
}

#[cfg(test)]
#[path = "dropdown_tests.rs"]
mod tests;
