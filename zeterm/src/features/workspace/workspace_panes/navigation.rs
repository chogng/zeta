use zeta_ui::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle,
    ButtonSelection, ButtonState, Component, ComponentContext, ComponentElement, ComputedElement,
    Element, InteractionRegion, Rect, Size, UiScene,
};
use zui::ui::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, FocusBehavior, NavigationAxis,
    NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use crate::shell_interaction::{
    AGENT_SIDEBAR_NAVIGATION, AGENT_SIDEBAR_TOOLBAR, AgentSidebarPaneAction,
};
use crate::workspace_panes::AgentSidebarStyle;
use crate::workspace_panes::AgentSidebarView;

const ITEM_WIDTH: f32 = 64.0;

/// Horizontal Changes/Files pane switcher hosted by a Sidebar Part toolbar.
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

    fn child_interaction_regions(&self) -> Vec<InteractionRegion> {
        let mut regions = Vec::new();
        let navigation = NavigationGroupId::new(AGENT_SIDEBAR_NAVIGATION);
        for (index, action) in AgentSidebarPaneAction::ALL.into_iter().enumerate() {
            let bounds = self
                .action_bar
                .interactive_item_bounds(index)
                .expect("pane actions are enabled");
            regions.push(
                InteractionRegion::new(
                    "AgentSidebarPaneButton",
                    action.element_id(),
                    bounds,
                    AccessibilityRole::Button,
                    action.label(),
                )
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
        regions
    }
}

impl Component for AgentSidebarNavigation {
    fn element(&self) -> ComponentElement {
        Element::leaf("AgentSidebarNavigation")
            .in_bounds(self.bounds)
            .with_identity(AGENT_SIDEBAR_NAVIGATION)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                AGENT_SIDEBAR_NAVIGATION,
                element.bounds(),
                AccessibilityRole::Toolbar,
                "Agent sidebar panes",
            )
            .with_parent(AGENT_SIDEBAR_TOOLBAR),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        for region in self.child_interaction_regions() {
            context.draw_component(&region);
        }
        context.draw_component(&self.action_bar);
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_component(&self.action_bar);
    }
}
