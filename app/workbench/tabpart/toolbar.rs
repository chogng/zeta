//! Session search and creation controls for body-mounted Workbench tabs.

use crate::{
    ActionBar, ActionBarItem, ActionBarOrientation, ActionBarStyle, ActionViewItem,
    ButtonBackgrounds, ButtonState, ButtonStyle, CaretVisibility, Component, ComponentContext,
    ComponentElement, ComputedElement, ContextMenu, ContextMenuItem, ContextMenuStyle,
    ContextViewAnchorAlignment, ContextViewPlacement, CornerRadii, Edges, Element,
    InteractionRegion, Rect, SearchBox, Size, TextInput, TextInputLayoutEngine, TextStyle, UiScene,
};
use zeta_icons::icons;
use zui::ui::{
    AccessibilityExpansion, AccessibilityRole, CursorFeedback, FocusBehavior, NavigationAxis,
    NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use super::WorkbenchUiStyle;
use super::identity::{
    ADD_SESSION, SESSION_SEARCH_INPUT, TAB_CONTAINER, TAB_CONTAINER_ACTION_BAR,
    TAB_CONTAINER_TOGGLE, TAB_CONTAINER_TOOLBAR, TAB_LAYOUT_MENU, TAB_LAYOUT_MENU_TRIGGER,
};

pub const PART_PADDING: f32 = 10.0;
pub const TOOLBAR_HEIGHT: f32 = 24.0;
pub const TOOLBAR_CONTENT_GAP: f32 = 4.0;
const ACTION_SIZE: f32 = TOOLBAR_HEIGHT;
const TOOLBAR_GAP: f32 = 6.0;
const ACTION_GAP: f32 = 4.0;
const LAYOUT_MENU_WIDTH: f32 = 184.0;
const LAYOUT_MENU_ITEM_HEIGHT: f32 = 30.0;
const LAYOUT_MENU_GAP: f32 = 2.0;
const LAYOUT_MENU_MARGIN: f32 = 6.0;

/// Tab Container toolbar with a leading Session SearchBox and trailing creation ActionBar.
pub struct TabContainerToolbar {
    bounds: Rect,
    search_box: SearchBox,
    search_value: String,
    action_bar: ActionBar,
    layout_menu: Option<TabLayoutMenu>,
}

impl TabContainerToolbar {
    pub fn new(
        part_bounds: Rect,
        search_input: &TextInput,
        caret_visibility: CaretVisibility,
        style: WorkbenchUiStyle,
        text_layout: &mut TextInputLayoutEngine,
        dispatch: &UiDispatch,
    ) -> Self {
        let bounds = Self::toolbar_bounds(part_bounds);
        let content_bounds = Rect::from_xywh(
            bounds.origin.x + PART_PADDING,
            bounds.origin.y,
            (bounds.size.width - PART_PADDING * 2.0).max(1.0),
            bounds.size.height,
        );
        let action_width = ACTION_SIZE * 2.0 + ACTION_GAP;
        let action_bounds = Rect::from_xywh(
            content_bounds.right() - action_width,
            content_bounds.origin.y,
            action_width,
            ACTION_SIZE,
        );
        let search_bounds = Rect::from_xywh(
            content_bounds.origin.x,
            content_bounds.origin.y,
            (action_bounds.origin.x - TOOLBAR_GAP - content_bounds.origin.x).max(1.0),
            content_bounds.size.height,
        );
        let search_state = if dispatch.is_focused(SESSION_SEARCH_INPUT) {
            crate::InputBoxState::Focused(caret_visibility)
        } else if dispatch.is_hovered(SESSION_SEARCH_INPUT) {
            crate::InputBoxState::Hovered
        } else {
            crate::InputBoxState::Resting
        };
        let layout_button_state = button_state(dispatch, TAB_LAYOUT_MENU_TRIGGER);
        let add_button_state = if dispatch.is_pressed(ADD_SESSION) {
            ButtonState::Pressed
        } else if dispatch.is_focused(ADD_SESSION) {
            ButtonState::Focused
        } else if dispatch.is_hovered(ADD_SESSION) {
            ButtonState::Hovered
        } else {
            ButtonState::Resting
        };
        let button_backgrounds = ButtonBackgrounds::new(crate::Color::TRANSPARENT)
            .with_hovered(style.selected)
            .with_focused(style.selected)
            .with_pressed(style.selected);
        let button_style = ButtonStyle::new(button_backgrounds, TextStyle::new(12.0, style.text))
            .with_corner_radii(CornerRadii::uniform(4.0))
            .with_padding(Edges::uniform(3.0))
            .with_icon_size(18.0);
        let action_bar = ActionBar::new(
            action_bounds,
            ActionBarOrientation::Horizontal,
            vec![
                ActionBarItem::Action(ActionViewItem::icon(
                    icons::LAYOUT,
                    "Configure tab layout",
                    layout_button_state,
                )),
                ActionBarItem::Action(ActionViewItem::icon(
                    style.add_icon,
                    "Add new session",
                    add_button_state,
                )),
            ],
            ActionBarStyle::new(button_style, Size::new(ACTION_SIZE, ACTION_SIZE))
                .with_gap(ACTION_GAP),
        );
        let layout_menu = dispatch.is_focused(TAB_LAYOUT_MENU_TRIGGER).then(|| {
            TabLayoutMenu::new(
                part_bounds,
                action_bar
                    .interactive_item_bounds(0)
                    .expect("Tab layout action is enabled"),
                style.clone(),
                dispatch,
            )
        });
        Self {
            bounds,
            search_box: SearchBox::new(
                search_bounds,
                "Search sessions...",
                search_state,
                style.search,
                search_input,
                text_layout,
            ),
            search_value: search_input.text().to_owned(),
            action_bar,
            layout_menu,
        }
    }

    fn child_interaction_regions(&self) -> Vec<InteractionRegion> {
        let search = InteractionRegion::new(
            "SessionSearchInput",
            SESSION_SEARCH_INPUT,
            self.search_box.bounds(),
            AccessibilityRole::TextInput,
            "Search sessions",
        )
        .with_cursor(CursorFeedback::Text)
        .with_focus(FocusBehavior::TabStop)
        .with_value(&self.search_value);
        let layout_bounds = self
            .action_bar
            .interactive_item_bounds(0)
            .expect("Tab layout action is enabled");
        let add_bounds = self
            .action_bar
            .interactive_item_bounds(1)
            .expect("Add session action is enabled");
        let layout = InteractionRegion::new(
            "TabLayoutMenuButton",
            TAB_LAYOUT_MENU_TRIGGER,
            layout_bounds,
            AccessibilityRole::Button,
            "Configure tab layout",
        )
        .with_cursor(CursorFeedback::Pointer)
        .with_focus(FocusBehavior::TabStop)
        .with_action(NodeAction::Activate)
        .with_expansion(if self.layout_menu.is_some() {
            AccessibilityExpansion::Expanded
        } else {
            AccessibilityExpansion::Collapsed
        })
        .with_navigation(
            NavigationGroupId::new(TAB_CONTAINER_ACTION_BAR),
            NavigationAxis::Horizontal,
        );
        let action = InteractionRegion::new(
            "AddSessionButton",
            ADD_SESSION,
            add_bounds,
            AccessibilityRole::Button,
            "Add new session",
        )
        .with_cursor(CursorFeedback::Pointer)
        .with_focus(FocusBehavior::TabStop)
        .with_action(NodeAction::Activate)
        .with_navigation(
            NavigationGroupId::new(TAB_CONTAINER_ACTION_BAR),
            NavigationAxis::Horizontal,
        );
        vec![
            search,
            InteractionRegion::new(
                "SessionActions",
                TAB_CONTAINER_ACTION_BAR,
                self.action_bar.bounds(),
                AccessibilityRole::Toolbar,
                "Session actions",
            )
            .with_children([layout, action]),
        ]
    }

    pub const fn search_caret_bounds(&self) -> Option<Rect> {
        self.search_box.caret_bounds()
    }

    pub fn content_bounds(part_bounds: Rect) -> Rect {
        Rect::from_xywh(
            part_bounds.origin.x + PART_PADDING,
            part_bounds.origin.y + PART_PADDING + TOOLBAR_HEIGHT + TOOLBAR_CONTENT_GAP,
            (part_bounds.size.width - PART_PADDING * 2.0).max(1.0),
            (part_bounds.size.height - PART_PADDING * 2.0 - TOOLBAR_HEIGHT - TOOLBAR_CONTENT_GAP)
                .max(1.0),
        )
    }

    fn toolbar_bounds(part_bounds: Rect) -> Rect {
        Rect::from_xywh(
            part_bounds.origin.x,
            part_bounds.origin.y + PART_PADDING,
            part_bounds.size.width,
            TOOLBAR_HEIGHT,
        )
    }
}

impl Component for TabContainerToolbar {
    fn element(&self) -> ComponentElement {
        Element::leaf("TabContainerToolbar")
            .padding(Edges::new(0.0, PART_PADDING, 0.0, PART_PADDING))
            .in_bounds(self.bounds)
            .with_identity(TAB_CONTAINER_TOOLBAR)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                TAB_CONTAINER_TOOLBAR,
                element.bounds(),
                AccessibilityRole::Toolbar,
                "Sessions toolbar",
            )
            .with_parent(TAB_CONTAINER),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        for region in self.child_interaction_regions() {
            context.draw_component(&region);
        }
        context.draw_component(&self.search_box);
        context.draw_component(&self.action_bar);
        if let Some(layout_menu) = &self.layout_menu {
            context.draw_component(layout_menu);
        }
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_component(&self.search_box);
        scene.draw_component(&self.action_bar);
        if let Some(layout_menu) = &self.layout_menu {
            scene.draw_component(layout_menu);
        }
    }
}

fn button_state(dispatch: &UiDispatch, id: zui::ui::ElementId) -> ButtonState {
    if dispatch.is_pressed(id) {
        ButtonState::Pressed
    } else if dispatch.is_focused(id) {
        ButtonState::Focused
    } else if dispatch.is_hovered(id) {
        ButtonState::Hovered
    } else {
        ButtonState::Resting
    }
}

struct TabLayoutMenu {
    context_menu: ContextMenu,
}

impl TabLayoutMenu {
    fn new(viewport: Rect, anchor: Rect, style: WorkbenchUiStyle, dispatch: &UiDispatch) -> Self {
        let selected = ButtonBackgrounds::new(style.selected)
            .with_hovered(style.selected)
            .with_focused(style.selected)
            .with_pressed(style.border);
        let item_style = ButtonStyle::new(
            ButtonBackgrounds::new(crate::Color::TRANSPARENT),
            TextStyle::new(13.0, style.text).with_line_height(18.0),
        )
        .with_selected_backgrounds(selected)
        .with_corner_radii(CornerRadii::uniform(2.0))
        .with_padding(Edges::new(0.0, 10.0, 0.0, 10.0));
        let context_menu = ContextMenu::new(
            viewport,
            anchor,
            vec![ContextMenuItem::new(
                "Move tabs to titlebar",
                button_state(dispatch, TAB_CONTAINER_TOGGLE),
            )],
            ContextMenuStyle::new(
                style.surface,
                item_style,
                Size::new(LAYOUT_MENU_WIDTH, LAYOUT_MENU_ITEM_HEIGHT),
            )
            .with_placement(
                ContextViewPlacement::new()
                    .with_alignment(ContextViewAnchorAlignment::End)
                    .with_gap(LAYOUT_MENU_GAP)
                    .with_viewport_margin(LAYOUT_MENU_MARGIN),
            ),
        );
        Self { context_menu }
    }

    fn item_region(&self) -> InteractionRegion {
        InteractionRegion::new(
            "TabLayoutMenuItem",
            TAB_CONTAINER_TOGGLE,
            self.context_menu
                .interactive_item_bounds(0)
                .expect("Tab layout menu command is enabled"),
            AccessibilityRole::MenuItem,
            "Move tabs to titlebar",
        )
        .with_parent(TAB_LAYOUT_MENU)
        .with_cursor(CursorFeedback::Pointer)
        .with_action(NodeAction::Activate)
    }
}

impl Component for TabLayoutMenu {
    fn element(&self) -> ComponentElement {
        Element::leaf("TabLayoutMenu")
            .in_bounds(self.context_menu.bounds())
            .with_identity(TAB_LAYOUT_MENU)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                TAB_LAYOUT_MENU,
                element.bounds(),
                AccessibilityRole::Menu,
                "Tab layout",
            )
            .with_parent(TAB_CONTAINER_TOOLBAR),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        context.draw_component(&self.item_region());
        context.draw_component(&self.context_menu);
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_component(&self.context_menu);
    }
}

#[cfg(test)]
#[path = "toolbar_tests.rs"]
mod tests;
