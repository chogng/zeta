use super::WorkbenchUiStyle;
use crate::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle, Border,
    ButtonBackgrounds, ButtonState, ButtonStyle, Component, ComponentContext, ComponentElement,
    ComputedElement, CornerRadii, Edges, Element, InteractionRegion, PaintRect, Rect, Size,
    TextStyle, UiScene,
};

use super::identity::{TAB_CONTAINER_TOGGLE, TITLEBAR, WINDOW, WORKSPACE_PANE_TOGGLE};
use super::tabs::TabContainer;
use super::tabs::TabContainerPlacement;
use zeta_workbench::PaneInputKind;
use zeta_workbench::TabInputKey;
use zeta_workbench::TabPart;
use zui::ui::{AccessibilityRole, CursorFeedback, FocusBehavior, NodeAction, UiDispatch, UiNode};

pub const TITLEBAR_HEIGHT: f32 = 32.0;
const TITLEBAR_ACTION_GAP: f32 = 8.0;
const TOGGLE_SIZE: f32 = 24.0;
const TOGGLE_ICON_SIZE: f32 = 18.0;

/// Logical space reserved for platform window controls on each titlebar edge.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TitlebarInsets {
    left: f32,
    right: f32,
}

impl TitlebarInsets {
    pub const NONE: Self = Self::new(0.0, 0.0);

    pub const fn new(left: f32, right: f32) -> Self {
        Self { left, right }
    }
}

/// Product-owned draggable titlebar for the single terminal surface.
pub struct Titlebar<'a> {
    bounds: Rect,
    style: WorkbenchUiStyle,
    left_action_bar: ActionBar,
    right_action_bar: ActionBar,
    tab_container: TabContainer<'a>,
    tab_container_toggle_label: &'static str,
    workspace_toggle_label: &'static str,
}

impl<'a> Titlebar<'a> {
    pub fn new(
        bounds: Rect,
        style: WorkbenchUiStyle,
        tab_part: &'a TabPart,
        active_tab: Option<&TabInputKey>,
        tabs_expanded: bool,
        active_pane_kind: Option<PaneInputKind>,
        window_control_insets: TitlebarInsets,
        dispatch: &'a UiDispatch,
    ) -> Self {
        let content_left = bounds.origin.x + window_control_insets.left;
        let content_right = (bounds.right() - window_control_insets.right).max(content_left);
        let tab_container_toggle_x = (content_left + TITLEBAR_ACTION_GAP)
            .min((content_right - TOGGLE_SIZE).max(content_left));
        let tab_container_toggle_bounds = Rect::from_xywh(
            tab_container_toggle_x,
            bounds.origin.y + (bounds.size.height - TOGGLE_SIZE) / 2.0,
            TOGGLE_SIZE,
            TOGGLE_SIZE,
        );
        let workspace_toggle_max_x = (content_right - TOGGLE_SIZE).max(content_left);
        let workspace_toggle_x = (content_right - TITLEBAR_ACTION_GAP - TOGGLE_SIZE)
            .max(tab_container_toggle_bounds.right() + TITLEBAR_ACTION_GAP)
            .min(workspace_toggle_max_x);
        let workspace_toggle_bounds = Rect::from_xywh(
            workspace_toggle_x,
            bounds.origin.y + (bounds.size.height - TOGGLE_SIZE) / 2.0,
            TOGGLE_SIZE,
            TOGGLE_SIZE,
        );
        let tab_container_toggle_state = if dispatch.is_pressed(TAB_CONTAINER_TOGGLE) {
            ButtonState::Pressed
        } else if dispatch.is_focused(TAB_CONTAINER_TOGGLE) {
            ButtonState::Focused
        } else if dispatch.is_hovered(TAB_CONTAINER_TOGGLE) {
            ButtonState::Hovered
        } else {
            ButtonState::Resting
        };
        let workspace_toggle_state = if dispatch.is_pressed(WORKSPACE_PANE_TOGGLE) {
            ButtonState::Pressed
        } else if dispatch.is_focused(WORKSPACE_PANE_TOGGLE) {
            ButtonState::Focused
        } else if dispatch.is_hovered(WORKSPACE_PANE_TOGGLE) {
            ButtonState::Hovered
        } else {
            ButtonState::Resting
        };
        let tab_container_toggle_label = if tabs_expanded {
            "Collapse tabs"
        } else {
            "Expand tabs"
        };
        let tab_container_toggle_icon = if tabs_expanded {
            style.tabs_expanded_icon
        } else {
            style.tabs_collapsed_icon
        };
        let workspace_pane_visible = matches!(
            active_pane_kind,
            Some(PaneInputKind::Files | PaneInputKind::Diff)
        );
        let workspace_toggle_label = if workspace_pane_visible {
            "Show agent workspace"
        } else {
            "Show workspace files"
        };
        let workspace_toggle_icon = if workspace_pane_visible {
            style.workspace_visible_icon
        } else {
            style.workspace_hidden_icon
        };
        let button_style = ButtonStyle::new(
            ButtonBackgrounds::new(style.surface_raised)
                .with_hovered(style.surface_hovered)
                .with_focused(style.surface_hovered)
                .with_pressed(style.border),
            TextStyle::new(12.0, style.text_muted),
        )
        .with_corner_radii(CornerRadii::uniform(4.0))
        .with_padding(Edges::uniform(3.0))
        .with_icon_size(TOGGLE_ICON_SIZE);
        let left_action_bar = ActionBar::new(
            tab_container_toggle_bounds,
            ActionBarOrientation::Horizontal,
            vec![ActionBarItem::Button(ActionBarButton::icon(
                tab_container_toggle_icon,
                tab_container_toggle_label,
                tab_container_toggle_state,
            ))],
            ActionBarStyle::new(button_style.clone(), Size::new(TOGGLE_SIZE, TOGGLE_SIZE)),
        );
        let right_action_bar = ActionBar::new(
            workspace_toggle_bounds,
            ActionBarOrientation::Horizontal,
            vec![ActionBarItem::Button(ActionBarButton::icon(
                workspace_toggle_icon,
                workspace_toggle_label,
                workspace_toggle_state,
            ))],
            ActionBarStyle::new(button_style, Size::new(TOGGLE_SIZE, TOGGLE_SIZE)),
        );
        let tabs_left = left_action_bar.bounds().right() + TITLEBAR_ACTION_GAP;
        let tabs_right = (right_action_bar.bounds().origin.x - TITLEBAR_ACTION_GAP).max(tabs_left);
        let tabs_bounds = Rect::from_xywh(
            tabs_left,
            bounds.origin.y,
            tabs_right - tabs_left,
            bounds.size.height,
        );
        Self {
            bounds,
            style: style.clone(),
            left_action_bar,
            right_action_bar,
            tab_container: TabContainer::from_tab_part(
                tabs_bounds,
                tabs_bounds,
                tab_part,
                active_tab,
                TabContainerPlacement::Titlebar,
                style,
                dispatch,
            ),
            tab_container_toggle_label,
            workspace_toggle_label,
        }
    }

    fn child_interaction_regions(&self) -> Vec<InteractionRegion> {
        vec![
            InteractionRegion::new(
                "TabContainerToggle",
                TAB_CONTAINER_TOGGLE,
                self.left_action_bar
                    .interactive_item_bounds(0)
                    .expect("Tab Container toggle is enabled"),
                AccessibilityRole::Button,
                self.tab_container_toggle_label,
            )
            .with_cursor(CursorFeedback::Pointer)
            .with_focus(FocusBehavior::TabStop)
            .with_action(NodeAction::Activate),
            InteractionRegion::new(
                "WorkspacePaneToggle",
                WORKSPACE_PANE_TOGGLE,
                self.right_action_bar
                    .interactive_item_bounds(0)
                    .expect("workspace pane toggle is enabled"),
                AccessibilityRole::Button,
                self.workspace_toggle_label,
            )
            .with_cursor(CursorFeedback::Pointer)
            .with_focus(FocusBehavior::TabStop)
            .with_action(NodeAction::Activate),
        ]
    }
}

impl Component for Titlebar<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("Titlebar")
            .in_bounds(self.bounds)
            .with_identity(TITLEBAR)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                TITLEBAR,
                element.bounds(),
                AccessibilityRole::Group,
                "Window titlebar",
            )
            .with_parent(WINDOW)
            .with_action(NodeAction::StartWindowDrag),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        for region in self.child_interaction_regions() {
            context.draw_component(&region);
        }
        context.scene_mut().draw_rect(
            PaintRect::new(self.bounds, self.style.surface_raised).with_border(Border::new(
                Edges::new(0.0, 0.0, 1.0, 0.0),
                self.style.border,
            )),
        );
        context.draw_component(&self.tab_container);
        context.draw_component(&self.left_action_bar);
        context.draw_component(&self.right_action_bar);
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(
            PaintRect::new(self.bounds, self.style.surface_raised).with_border(Border::new(
                Edges::new(0.0, 0.0, 1.0, 0.0),
                self.style.border,
            )),
        );
        scene.draw_component(&self.tab_container);
        scene.draw_component(&self.left_action_bar);
        scene.draw_component(&self.right_action_bar);
    }
}

#[cfg(test)]
#[path = "titlebar_tests.rs"]
mod tests;
