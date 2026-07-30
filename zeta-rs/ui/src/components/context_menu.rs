use crate::{
    BoxShadow, Color, Component, ContextView, ContextViewPlacement, ContextViewStyle, CornerRadii,
    PaintRect, Point, Rect, Size, UiScene,
};

use super::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle,
    ButtonSelection, ButtonState, ButtonStyle,
};

const MENU_PADDING: f32 = 2.0;
const MENU_CORNER_RADIUS: f32 = 4.0;
const MENU_AMBIENT_SHADOW: Color = Color::rgba(0, 0, 0, 24);
const MENU_AMBIENT_SHADOW_OFFSET_Y: f32 = 1.0;
const MENU_AMBIENT_SHADOW_BLUR_RADIUS: f32 = 10.0;
const MENU_KEY_SHADOW: Color = Color::rgba(0, 0, 0, 36);
const MENU_KEY_SHADOW_OFFSET_Y: f32 = 4.0;
const MENU_KEY_SHADOW_BLUR_RADIUS: f32 = 6.0;

/// One label item projected into a [`ContextMenu`].
#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenuItem {
    label: String,
    state: ButtonState,
}

impl ContextMenuItem {
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

/// Selection policy used when a context menu is presented.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum ContextMenuSelection {
    /// Selects the first enabled item. This is the default open-state behavior.
    #[default]
    FirstEnabled,
    /// Selects one item by its presentation index.
    Item(usize),
    /// Presents the menu without a selected item.
    None,
}

/// Shared surface, item, and anchor presentation for a [`ContextMenu`].
#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenuStyle {
    background: Color,
    button_style: ButtonStyle,
    item_size: Size,
    placement: ContextViewPlacement,
}

impl ContextMenuStyle {
    pub fn new(background: Color, button_style: ButtonStyle, item_size: Size) -> Self {
        Self {
            background,
            button_style,
            item_size,
            placement: ContextViewPlacement::new(),
        }
    }

    pub const fn with_placement(mut self, placement: ContextViewPlacement) -> Self {
        self.placement = placement;
        self
    }
}

/// Presentation-only anchored menu surface shared by product context menus.
///
/// ContextMenu composes [`ContextView`] placement with a shadowed, borderless menu surface and a
/// vertical [`ActionBar`]. The menu surface owns its canonical 2px padding and 4px corner radius.
/// The product host owns retained open state, selected identity, input routing, accessibility,
/// dismissal, and command execution.
#[derive(Clone, Debug, PartialEq)]
pub struct ContextMenu {
    context_view: ContextView,
    surface_bounds: Rect,
    menu_bounds: Rect,
    items: Vec<ContextMenuItem>,
    style: ContextMenuStyle,
    selection: ContextMenuSelection,
}

impl ContextMenu {
    pub fn new(
        viewport: Rect,
        anchor: Rect,
        items: Vec<ContextMenuItem>,
        style: ContextMenuStyle,
    ) -> Self {
        let menu_size = Size::new(
            style.item_size.width.max(0.0),
            style.item_size.height.max(0.0) * items.len() as f32,
        );
        let surface_size = Size::new(
            menu_size.width + MENU_PADDING * 2.0,
            menu_size.height + MENU_PADDING * 2.0,
        );
        let context_view = ContextView::new(
            viewport,
            anchor,
            surface_size,
            style.placement,
            ContextViewStyle::new(Color::TRANSPARENT),
        );
        let surface_bounds = context_view.content_bounds();
        let menu_bounds = inset_rect(surface_bounds, MENU_PADDING);
        Self {
            context_view,
            surface_bounds,
            menu_bounds,
            items,
            style,
            selection: ContextMenuSelection::default(),
        }
    }

    pub const fn with_selection(mut self, selection: ContextMenuSelection) -> Self {
        self.selection = selection;
        self
    }

    /// Returns the interactive menu surface, excluding its visual shadow.
    pub const fn bounds(&self) -> Rect {
        self.surface_bounds
    }

    /// Returns the item layout bounds inset by the menu's canonical 2px padding.
    pub const fn content_bounds(&self) -> Rect {
        self.menu_bounds
    }

    pub fn selected_index(&self) -> Option<usize> {
        match self.selection {
            ContextMenuSelection::FirstEnabled => {
                self.items.iter().position(ContextMenuItem::is_enabled)
            }
            ContextMenuSelection::Item(index) => self
                .items
                .get(index)
                .filter(|item| item.is_enabled())
                .map(|_| index),
            ContextMenuSelection::None => None,
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
            self.menu_bounds,
            ActionBarOrientation::Vertical,
            items,
            ActionBarStyle::new(self.style.button_style.clone(), self.style.item_size),
        )
    }
}

impl Component for ContextMenu {
    fn paint(&self, scene: &mut UiScene) {
        let action_bar = self.action_bar();
        self.context_view
            .draw_overflow(scene, |scene, _content_bounds| {
                scene.draw_rect(
                    PaintRect::new(self.surface_bounds, Color::TRANSPARENT)
                        .with_shadow(
                            BoxShadow::new(MENU_AMBIENT_SHADOW)
                                .with_offset(Point::new(0.0, MENU_AMBIENT_SHADOW_OFFSET_Y))
                                .with_blur_radius(MENU_AMBIENT_SHADOW_BLUR_RADIUS),
                        )
                        .with_corner_radii(CornerRadii::uniform(MENU_CORNER_RADIUS)),
                );
                scene.draw_rect(
                    PaintRect::new(self.surface_bounds, self.style.background)
                        .with_shadow(
                            BoxShadow::new(MENU_KEY_SHADOW)
                                .with_offset(Point::new(0.0, MENU_KEY_SHADOW_OFFSET_Y))
                                .with_blur_radius(MENU_KEY_SHADOW_BLUR_RADIUS),
                        )
                        .with_corner_radii(CornerRadii::uniform(MENU_CORNER_RADIUS)),
                );
                action_bar.paint(scene);
            });
    }
}

fn inset_rect(bounds: Rect, inset: f32) -> Rect {
    Rect::from_xywh(
        bounds.origin.x + inset,
        bounds.origin.y + inset,
        (bounds.size.width - inset * 2.0).max(0.0),
        (bounds.size.height - inset * 2.0).max(0.0),
    )
}

#[cfg(test)]
#[path = "context_menu_tests.rs"]
mod tests;
