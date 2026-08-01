use zeta_icons::icons;
use zeta_ui::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle,
    ButtonBackgrounds, ButtonState, ButtonStyle, CaretVisibility, Component, ComponentInspection,
    CornerRadii, Edges, Rect, SearchBox, Size, TextInput, TextInputLayoutEngine, TextStyle,
    UiScene,
};
use zeta_ui_dispatch::{
    AccessibilityRole, CursorFeedback, FocusBehavior, InteractionFrame, NavigationAxis,
    NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use crate::shell_interaction::{
    ADD_SESSION, SESSION_SEARCH_INPUT, SESSION_SIDEBAR, SESSION_SIDEBAR_ACTION_BAR,
    SESSION_SIDEBAR_TOOLBAR,
};
use crate::shell_style::ShellPalette;

pub(crate) const SIDEBAR_PADDING: f32 = 10.0;
pub(crate) const TOOLBAR_HEIGHT: f32 = 24.0;
pub(crate) const TOOLBAR_CONTENT_GAP: f32 = 4.0;
const ACTION_SIZE: f32 = TOOLBAR_HEIGHT;
const TOOLBAR_GAP: f32 = 6.0;

/// Sessions toolbar that composes a leading SearchBox and trailing ActionBar.
pub(crate) struct SessionSidebarToolbar {
    bounds: Rect,
    search_box: SearchBox,
    search_value: String,
    action_bar: ActionBar,
}

impl SessionSidebarToolbar {
    pub(crate) fn new(
        sidebar_bounds: Rect,
        search_input: &TextInput,
        caret_visibility: CaretVisibility,
        palette: ShellPalette,
        text_layout: &mut TextInputLayoutEngine,
        dispatch: &UiDispatch,
    ) -> Self {
        let bounds = Self::toolbar_bounds(sidebar_bounds);
        let content_bounds = Rect::from_xywh(
            bounds.origin.x + SIDEBAR_PADDING,
            bounds.origin.y,
            (bounds.size.width - SIDEBAR_PADDING * 2.0).max(1.0),
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

    pub(crate) fn register_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                SESSION_SIDEBAR_TOOLBAR,
                self.bounds,
                AccessibilityRole::Toolbar,
                "Sessions toolbar",
            )
            .with_parent(SESSION_SIDEBAR),
        );
        frame.register(
            UiNode::new(
                SESSION_SEARCH_INPUT,
                self.search_box.bounds(),
                AccessibilityRole::TextInput,
                "Search sessions",
            )
            .with_parent(SESSION_SIDEBAR_TOOLBAR)
            .with_cursor(CursorFeedback::Text)
            .with_focus(FocusBehavior::TabStop)
            .with_value(&self.search_value),
        );
        frame.register(
            UiNode::new(
                SESSION_SIDEBAR_ACTION_BAR,
                self.action_bar.bounds(),
                AccessibilityRole::Toolbar,
                "Session actions",
            )
            .with_parent(SESSION_SIDEBAR_TOOLBAR),
        );
        let add_bounds = self
            .action_bar
            .interactive_item_bounds(0)
            .expect("Add session action is enabled");
        frame.register(
            UiNode::new(
                ADD_SESSION,
                add_bounds,
                AccessibilityRole::Button,
                "Add new session",
            )
            .with_parent(SESSION_SIDEBAR_ACTION_BAR)
            .with_cursor(CursorFeedback::Pointer)
            .with_focus(FocusBehavior::TabStop)
            .with_action(NodeAction::Activate)
            .with_navigation(
                NavigationGroupId::new(SESSION_SIDEBAR_ACTION_BAR),
                NavigationAxis::Horizontal,
            ),
        );
    }

    pub(crate) const fn search_caret_bounds(&self) -> Option<Rect> {
        self.search_box.caret_bounds()
    }

    pub(crate) fn content_bounds(sidebar_bounds: Rect) -> Rect {
        Rect::from_xywh(
            sidebar_bounds.origin.x + SIDEBAR_PADDING,
            sidebar_bounds.origin.y + SIDEBAR_PADDING + TOOLBAR_HEIGHT + TOOLBAR_CONTENT_GAP,
            (sidebar_bounds.size.width - SIDEBAR_PADDING * 2.0).max(1.0),
            (sidebar_bounds.size.height
                - SIDEBAR_PADDING * 2.0
                - TOOLBAR_HEIGHT
                - TOOLBAR_CONTENT_GAP)
                .max(1.0),
        )
    }

    fn toolbar_bounds(sidebar_bounds: Rect) -> Rect {
        Rect::from_xywh(
            sidebar_bounds.origin.x,
            sidebar_bounds.origin.y + SIDEBAR_PADDING,
            sidebar_bounds.size.width,
            TOOLBAR_HEIGHT,
        )
    }
}

impl Component for SessionSidebarToolbar {
    fn inspection(&self) -> ComponentInspection {
        ComponentInspection::new("SessionSidebarToolbar", self.bounds).with_padding(Edges::new(
            0.0,
            SIDEBAR_PADDING,
            0.0,
            SIDEBAR_PADDING,
        ))
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_component(&self.search_box);
        scene.draw_component(&self.action_bar);
    }
}

#[cfg(test)]
#[path = "session_sidebar_toolbar_tests.rs"]
mod tests;
