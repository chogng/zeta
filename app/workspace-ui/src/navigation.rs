use zeta_ui::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle,
    ButtonSelection, ButtonState, Component, ComponentContext, ComponentElement, ComputedElement,
    Element, InteractionRegion, Rect, Size, UiScene,
};
use zui::ui::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, FocusBehavior, NavigationAxis,
    NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use crate::WorkspacePaneSelection;
use crate::WorkspacePaneStyle;
use crate::WorkspacePaneView;
use crate::interaction::{WORKSPACE_PANE_NAVIGATION, WORKSPACE_PANE_TOOLBAR};

const ITEM_WIDTH: f32 = 64.0;

/// Horizontal Changes/Files switcher hosted by a Workspace Pane toolbar.
pub struct WorkspacePaneNavigation {
    bounds: Rect,
    action_bar: ActionBar,
    selected: WorkspacePaneView,
}

impl WorkspacePaneNavigation {
    pub fn bounds_in(toolbar: Rect) -> Rect {
        Rect::from_xywh(
            toolbar.origin.x,
            toolbar.origin.y,
            (ITEM_WIDTH * WorkspacePaneSelection::ALL.len() as f32).min(toolbar.size.width),
            toolbar.size.height,
        )
    }

    pub fn new(
        bounds: Rect,
        selected: WorkspacePaneView,
        palette: &WorkspacePaneStyle,
        dispatch: &UiDispatch,
    ) -> Self {
        let button_style = palette.navigation_button_style();
        let items = WorkspacePaneSelection::ALL
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
        let navigation = NavigationGroupId::new(WORKSPACE_PANE_NAVIGATION);
        for (index, action) in WorkspacePaneSelection::ALL.into_iter().enumerate() {
            let bounds = self
                .action_bar
                .interactive_item_bounds(index)
                .expect("pane actions are enabled");
            regions.push(
                InteractionRegion::new(
                    "WorkspacePaneButton",
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

impl Component for WorkspacePaneNavigation {
    fn element(&self) -> ComponentElement {
        Element::leaf("WorkspacePaneNavigation")
            .in_bounds(self.bounds)
            .with_identity(WORKSPACE_PANE_NAVIGATION)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                WORKSPACE_PANE_NAVIGATION,
                element.bounds(),
                AccessibilityRole::Toolbar,
                "Workspace panes",
            )
            .with_parent(WORKSPACE_PANE_TOOLBAR),
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
