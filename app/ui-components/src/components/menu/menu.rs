use crate::{
    AccessibilityRole, AccessibilitySelection, BoxShadow, Color, Component, ComponentContext,
    ComponentElement, ComputedElement, CornerRadii, CursorFeedback, Edges, Element, ElementId,
    FocusBehavior, NavigationAxis, NavigationGroupId, NodeAction, PaintRect, Point, Rect, Size,
    UiNode, UiScene,
};

use super::{
    ActionBar, ActionBarItem, ActionBarOrientation, ActionBarStyle, ActionViewItem,
    ButtonSelection, ButtonState, ButtonStyle, InteractionRegion,
};

const MENU_PADDING: f32 = 2.0;
const MENU_CORNER_RADIUS: f32 = 4.0;
const MENU_SHADOW: Color = Color::rgba(0, 0, 0, 64);
const MENU_SHADOW_OFFSET_Y: f32 = 4.0;
const MENU_SHADOW_BLUR_RADIUS: f32 = 26.352_942;

/// Stable host-owned identities used by one menu.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MenuIds {
    parent: ElementId,
    root: ElementId,
}

impl MenuIds {
    pub const fn new(parent: ElementId, root: ElementId) -> Self {
        Self { parent, root }
    }
}

/// One host-owned action presented by a [`Menu`].
#[derive(Clone, Debug, PartialEq)]
pub struct MenuItem {
    element: ElementId,
    label: String,
    state: ButtonState,
}

impl MenuItem {
    pub fn new(element: ElementId, label: impl Into<String>, state: ButtonState) -> Self {
        Self {
            element,
            label: label.into(),
            state,
        }
    }

    const fn is_enabled(&self) -> bool {
        !matches!(self.state, ButtonState::Disabled)
    }
}

/// Selection policy used when a menu is presented.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum MenuSelection {
    /// Selects the first enabled item. This is the default open-state behavior.
    #[default]
    FirstEnabled,
    /// Selects one item by its presentation index.
    Item(usize),
    /// Presents the menu without a selected item.
    None,
}

/// Shared surface and item presentation for a [`Menu`].
#[derive(Clone, Debug, PartialEq)]
pub struct MenuStyle {
    background: Color,
    button_style: ButtonStyle,
    item_size: Size,
    header_height: f32,
}

impl MenuStyle {
    pub fn new(background: Color, button_style: ButtonStyle, item_size: Size) -> Self {
        Self {
            background,
            button_style,
            item_size,
            header_height: 0.0,
        }
    }

    /// Reserves a leading row that the caller can fill through the menu composition methods.
    pub const fn with_header_height(mut self, header_height: f32) -> Self {
        self.header_height = header_height;
        self
    }
}

/// Reusable menu content with one interaction and accessibility tree.
///
/// Menu owns its surface, item layout, selection presentation, item interaction regions, and
/// accessibility semantics. The host owns open state, dismissal, focus restoration, and command
/// execution. Anchored placement belongs to [`super::ContextMenu`].
#[derive(Clone, Debug, PartialEq)]
pub struct Menu {
    bounds: Rect,
    content_bounds: Rect,
    item_bounds: Rect,
    header_bounds: Option<Rect>,
    items: Vec<MenuItem>,
    ids: MenuIds,
    accessibility_label: String,
    style: MenuStyle,
    selection: MenuSelection,
}

impl Menu {
    pub fn new(
        bounds: Rect,
        accessibility_label: impl Into<String>,
        items: Vec<MenuItem>,
        ids: MenuIds,
        style: MenuStyle,
    ) -> Self {
        let content_bounds = inset_rect(bounds, MENU_PADDING);
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
            bounds,
            content_bounds,
            item_bounds,
            header_bounds,
            items,
            ids,
            accessibility_label: accessibility_label.into(),
            style,
            selection: MenuSelection::default(),
        }
    }

    pub(crate) fn desired_size(item_count: usize, style: &MenuStyle) -> Size {
        Size::new(
            style.item_size.width.max(0.0) + MENU_PADDING * 2.0,
            style.header_height.max(0.0)
                + style.item_size.height.max(0.0) * item_count as f32
                + MENU_PADDING * 2.0,
        )
    }

    pub const fn with_selection(mut self, selection: MenuSelection) -> Self {
        self.selection = selection;
        self
    }

    pub const fn root(&self) -> ElementId {
        self.ids.root
    }

    /// Returns the interactive menu surface, excluding its visual shadow.
    pub const fn bounds(&self) -> Rect {
        self.bounds
    }

    /// Returns the item layout bounds inset by the menu's canonical padding.
    pub const fn content_bounds(&self) -> Rect {
        self.content_bounds
    }

    /// Returns the caller-owned leading row, when one was reserved by the style.
    pub const fn header_bounds(&self) -> Option<Rect> {
        self.header_bounds
    }

    pub fn selected_index(&self) -> Option<usize> {
        match self.selection {
            MenuSelection::FirstEnabled => self.items.iter().position(MenuItem::is_enabled),
            MenuSelection::Item(index) => self
                .items
                .get(index)
                .filter(|item| item.is_enabled())
                .map(|_| index),
            MenuSelection::None => None,
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

    /// Paints the canonical menu shell and items with caller-owned content in its header row.
    pub fn paint_with_header(
        &self,
        scene: &mut UiScene,
        paint_header: impl FnOnce(&mut UiScene, Rect),
    ) {
        scene.with_element(self.element_tree(), |scene, _element| {
            self.paint_contents(scene, paint_header)
        });
    }

    /// Composes the menu and caller-owned header content through one interaction tree.
    pub fn draw_components_with_header(
        &self,
        context: &mut ComponentContext<'_, '_>,
        draw_header: impl FnOnce(&mut ComponentContext<'_, '_>, Rect),
    ) {
        context.with_component(self, |context, _element| {
            self.compose_contents(context, draw_header)
        });
    }

    fn action_bar(&self) -> ActionBar {
        let selected_index = self.selected_index();
        let items = self
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                ActionBarItem::Action(
                    ActionViewItem::label(item.label.clone(), item.state).with_selection(
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

    fn interaction_regions(&self) -> Vec<InteractionRegion> {
        let navigation = NavigationGroupId::new(self.ids.root);
        self.items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let bounds = self.interactive_item_bounds(index)?;
                Some(
                    InteractionRegion::new(
                        "MenuItem",
                        item.element,
                        bounds,
                        AccessibilityRole::MenuItem,
                        item.label.clone(),
                    )
                    .with_cursor(CursorFeedback::Pointer)
                    .with_focus(FocusBehavior::TabStop)
                    .with_action(NodeAction::Activate)
                    .with_navigation(navigation, NavigationAxis::Vertical)
                    .with_selection(if self.selected_index() == Some(index) {
                        AccessibilitySelection::Selected
                    } else {
                        AccessibilitySelection::Unselected
                    }),
                )
            })
            .collect()
    }

    fn element_tree(&self) -> ComponentElement {
        Element::leaf("Menu")
            .padding(Edges::uniform(MENU_PADDING))
            .corner_radii(CornerRadii::uniform(MENU_CORNER_RADIUS))
            .in_bounds(self.bounds)
            .with_identity(self.ids.root)
    }

    fn paint_surface(&self, scene: &mut UiScene) {
        scene.draw_rect(
            PaintRect::new(self.bounds, self.style.background)
                .with_shadow(
                    BoxShadow::new(MENU_SHADOW)
                        .with_offset(Point::new(0.0, MENU_SHADOW_OFFSET_Y))
                        .with_blur_radius(MENU_SHADOW_BLUR_RADIUS),
                )
                .with_corner_radii(CornerRadii::uniform(MENU_CORNER_RADIUS)),
        );
    }

    fn paint_contents(&self, scene: &mut UiScene, paint_header: impl FnOnce(&mut UiScene, Rect)) {
        self.paint_surface(scene);
        if let Some(header_bounds) = self.header_bounds {
            paint_header(scene, header_bounds);
        }
        scene.draw_component(&self.action_bar());
    }

    fn compose_contents(
        &self,
        context: &mut ComponentContext<'_, '_>,
        draw_header: impl FnOnce(&mut ComponentContext<'_, '_>, Rect),
    ) {
        self.paint_surface(context.scene_mut());
        if let Some(header_bounds) = self.header_bounds {
            draw_header(context, header_bounds);
        }
        for region in self.interaction_regions() {
            context.draw_component(&region);
        }
        context.draw_component(&self.action_bar());
    }
}

impl Component for Menu {
    fn element(&self) -> ComponentElement {
        self.element_tree()
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                self.ids.root,
                element.bounds(),
                AccessibilityRole::Menu,
                self.accessibility_label.clone(),
            )
            .with_parent(self.ids.parent),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        self.compose_contents(context, |_context, _bounds| {});
    }

    fn paint_element(&self, scene: &mut UiScene, _element: &ComputedElement) {
        self.paint_contents(scene, |_scene, _bounds| {});
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
#[path = "menu_tests.rs"]
mod tests;
