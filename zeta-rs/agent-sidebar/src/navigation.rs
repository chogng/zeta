use zeta_ui::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle,
    ButtonSelection, ButtonState, Component, ComponentElement, Element, Rect, Size, UiScene,
};
use zui::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, FocusBehavior, InteractionFrame,
    NavigationAxis, NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use crate::AgentSidebarStyle;
use crate::AgentSidebarView;
use crate::shell_interaction::{
    AGENT_SIDEBAR_NAVIGATION, AGENT_SIDEBAR_TOOLBAR, AgentSidebarPaneAction,
};

const ITEM_WIDTH: f32 = 64.0;

/// Horizontal pane switcher hosted by the Agent Sidebar toolbar.
pub struct AgentSidebarNavigation {
    bounds: Rect,
    action_bar: ActionBar,
    selected: AgentSidebarView,
}

impl AgentSidebarNavigation {
    pub fn bounds_in(toolbar: Rect) -> Rect {
        Rect::from_xywh(
            toolbar.origin.x,
            toolbar.origin.y,
            (ITEM_WIDTH * AgentSidebarPaneAction::ALL.len() as f32).min(toolbar.size.width),
            toolbar.size.height,
        )
    }

    pub fn new(
        bounds: Rect,
        selected: AgentSidebarView,
        palette: &AgentSidebarStyle,
        dispatch: &UiDispatch,
    ) -> Self {
        let button_style = palette.navigation_button_style();
        let items = AgentSidebarPaneAction::ALL
            .into_iter()
            .map(|action| {
                let target = action.element_id();
                let state = if dispatch.is_pressed(target) {
                    ButtonState::Pressed
                } else if dispatch.is_focused(target) {
                    ButtonState::Focused
                } else if dispatch.is_hovered(target) {
                    ButtonState::Hovered
                } else {
                    ButtonState::Resting
                };
                ActionBarItem::Button(
                    ActionBarButton::label(action.label(), state).with_selection(
                        if action.view() == selected {
                            ButtonSelection::Selected
                        } else {
                            ButtonSelection::Unselected
                        },
                    ),
                )
            })
            .collect();
        Self {
            bounds,
            selected,
            action_bar: ActionBar::new(
                bounds,
                ActionBarOrientation::Horizontal,
                items,
                ActionBarStyle::new(button_style, Size::new(ITEM_WIDTH, bounds.size.height)),
            ),
        }
    }

    pub fn register_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                AGENT_SIDEBAR_NAVIGATION,
                self.bounds,
                AccessibilityRole::Toolbar,
                "Agent sidebar panes",
            )
            .with_parent(AGENT_SIDEBAR_TOOLBAR),
        );
        let navigation = NavigationGroupId::new(AGENT_SIDEBAR_NAVIGATION);
        for (index, action) in AgentSidebarPaneAction::ALL.into_iter().enumerate() {
            let bounds = self
                .action_bar
                .interactive_item_bounds(index)
                .expect("pane actions are enabled");
            frame.register(
                UiNode::new(
                    action.element_id(),
                    bounds,
                    AccessibilityRole::Button,
                    action.label(),
                )
                .with_parent(AGENT_SIDEBAR_NAVIGATION)
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation, NavigationAxis::Horizontal)
                .with_selection(if action.view() == self.selected {
                    AccessibilitySelection::Selected
                } else {
                    AccessibilitySelection::Unselected
                }),
            );
        }
    }
}

impl Component for AgentSidebarNavigation {
    fn element(&self) -> ComponentElement {
        Element::leaf("AgentSidebarNavigation").in_bounds(self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_component(&self.action_bar);
    }
}
