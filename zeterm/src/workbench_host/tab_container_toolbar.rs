//! Session search and creation controls for the body-mounted Tab Container.

use zeta_icons::icons;
use zeta_ui::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle,
    ButtonBackgrounds, ButtonState, ButtonStyle, CaretVisibility, Component, ComponentContext,
    ComponentElement, ComputedElement, CornerRadii, Edges, Element, InteractionRegion, Rect,
    SearchBox, Size, TextInput, TextInputLayoutEngine, TextStyle, UiScene,
};
use zui::ui::{
    AccessibilityRole, CursorFeedback, FocusBehavior, NavigationAxis, NavigationGroupId,
    NodeAction, UiDispatch, UiNode,
};

use crate::shell_interaction::{
    ADD_SESSION, SESSION_SEARCH_INPUT, TAB_CONTAINER, TAB_CONTAINER_ACTION_BAR,
    TAB_CONTAINER_TOOLBAR,
};
use crate::shell_style::ShellPalette;

pub(crate) const PART_PADDING: f32 = 10.0;
pub(crate) const TOOLBAR_HEIGHT: f32 = 24.0;
pub(crate) const TOOLBAR_CONTENT_GAP: f32 = 4.0;
const ACTION_SIZE: f32 = TOOLBAR_HEIGHT;
const TOOLBAR_GAP: f32 = 6.0;

/// Tab Container toolbar with a leading Session SearchBox and trailing creation ActionBar.
pub(crate) struct TabContainerToolbar {
    bounds: Rect,
    search_box: SearchBox,
    search_value: String,
    action_bar: ActionBar,
}

impl TabContainerToolbar {
    pub(crate) fn new(
        part_bounds: Rect,
        search_input: &TextInput,
        caret_visibility: CaretVisibility,
        palette: ShellPalette,
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
        let action_bounds = Rect::from_xywh(
            content_bounds.right() - ACTION_SIZE,
            content_bounds.origin.y,
            ACTION_SIZE,
            ACTION_SIZE,
        );
        let search_bounds = Rect::from_xywh(
            content_bounds.origin.x,
            content_bounds.origin.y,
            (action_bounds.origin.x - TOOLBAR_GAP - content_bounds.origin.x).max(1.0),
            content_bounds.size.height,
        );
        let search_state = if dispatch.is_focused(SESSION_SEARCH_INPUT) {
            zeta_ui::InputBoxState::Focused(caret_visibility)
        } else if dispatch.is_hovered(SESSION_SEARCH_INPUT) {
            zeta_ui::InputBoxState::Hovered
        } else {
            zeta_ui::InputBoxState::Resting
        };
        let button_state = if dispatch.is_pressed(ADD_SESSION) {
            ButtonState::Pressed
        } else if dispatch.is_focused(ADD_SESSION) {
            ButtonState::Focused
        } else if dispatch.is_hovered(ADD_SESSION) {
            ButtonState::Hovered
        } else {
            ButtonState::Resting
        };
        let button_backgrounds = ButtonBackgrounds::new(zeta_ui::Color::TRANSPARENT)
            .with_hovered(palette.session_tab_highlight)
            .with_focused(palette.session_tab_highlight)
            .with_pressed(palette.session_tab_highlight);
        let button_style = ButtonStyle::new(button_backgrounds, TextStyle::new(12.0, palette.text))
            .with_corner_radii(CornerRadii::uniform(4.0))
            .with_padding(Edges::uniform(3.0))
            .with_icon_size(18.0);
        Self {
            bounds,
            search_box: SearchBox::new(
                search_bounds,
                "Search sessions...",
                search_state,
                palette.session_search_style(),
                search_input,
                text_layout,
            ),
            search_value: search_input.text().to_owned(),
            action_bar: ActionBar::new(
                action_bounds,
                ActionBarOrientation::Horizontal,
                vec![ActionBarItem::Button(ActionBarButton::icon(
                    icons::ADD,
                    "Add new session",
                    button_state,
                ))],
                ActionBarStyle::new(button_style, Size::new(ACTION_SIZE, ACTION_SIZE)),
            ),
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
        let add_bounds = self
            .action_bar
            .interactive_item_bounds(0)
            .expect("Add session action is enabled");
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
            .with_children([action]),
        ]
    }

    pub(crate) const fn search_caret_bounds(&self) -> Option<Rect> {
        self.search_box.caret_bounds()
    }

    pub(crate) fn content_bounds(part_bounds: Rect) -> Rect {
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
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_component(&self.search_box);
        scene.draw_component(&self.action_bar);
    }
}

#[cfg(test)]
#[path = "tab_container_toolbar_tests.rs"]
mod tests;
