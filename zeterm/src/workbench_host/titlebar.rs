use zeta_icons::icons;
use zeta_ui::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle, Border,
    ButtonBackgrounds, ButtonState, ButtonStyle, Component, ComponentContext, ComponentElement,
    ComputedElement, CornerRadii, Edges, Element, InteractionRegion, PaintRect, Rect, Size,
    TextStyle, UiScene,
};

use crate::shell_interaction::{TAB_CONTAINER_TOGGLE, TITLEBAR, WINDOW, WORKSPACE_PANE_TOGGLE};
use crate::shell_style::ShellPalette;
use crate::workbench_host::PaneInputKind;
use crate::workbench_host::TabContainerState;
use crate::workbench_host::TabInputKey;
use crate::workbench_host::TabPart;
use crate::workbench_host::tab_container::TabContainer;
use crate::workbench_host::tab_container::TabContainerPlacement;
use zui::ui::{AccessibilityRole, CursorFeedback, FocusBehavior, NodeAction, UiDispatch, UiNode};
use zui::window::WindowControlInsets;

pub(crate) const TITLEBAR_HEIGHT: f32 = 32.0;
const TITLEBAR_ACTION_GAP: f32 = 8.0;
const TOGGLE_SIZE: f32 = 24.0;
const TOGGLE_ICON_SIZE: f32 = 18.0;

/// Product-owned draggable titlebar for the single terminal surface.
pub(crate) struct Titlebar<'a> {
    bounds: Rect,
    palette: ShellPalette,
    left_action_bar: ActionBar,
    right_action_bar: ActionBar,
    tab_container: TabContainer<'a>,
    tab_container_toggle_label: &'static str,
    workspace_toggle_label: &'static str,
}

impl<'a> Titlebar<'a> {
    pub(crate) fn new(
        bounds: Rect,
        palette: ShellPalette,
        tab_part: &'a TabPart,
        active_tab: Option<&TabInputKey>,
        tab_container: TabContainerState,
        active_pane_kind: Option<PaneInputKind>,
        window_control_insets: WindowControlInsets,
        dispatch: &'a UiDispatch,
    ) -> Self {
        let content_left = bounds.origin.x + window_control_insets.left();
        let content_right = (bounds.right() - window_control_insets.right()).max(content_left);
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
        let tab_container_toggle_label = if tab_container.is_expanded() {
            "Collapse tabs"
        } else {
            "Expand tabs"
        };
        let tab_container_toggle_icon = if tab_container.is_expanded() {
            icons::LAYOUT_SIDEBAR_LEFT
        } else {
            icons::LAYOUT_SIDEBAR_LEFT_OFF_EMPTY
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
            icons::LAYOUT_SIDEBAR_RIGHT
        } else {
            icons::LAYOUT_SIDEBAR_RIGHT_OFF_EMPTY
        };
        let button_style = ButtonStyle::new(
            ButtonBackgrounds::new(palette.surface_raised)
                .with_hovered(palette.surface_hovered)
                .with_focused(palette.surface_hovered)
                .with_pressed(palette.border),
            TextStyle::new(12.0, palette.text_muted),
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
            palette,
            left_action_bar,
            right_action_bar,
            tab_container: TabContainer::from_tab_part(
                tabs_bounds,
                tabs_bounds,
                tab_part,
                active_tab,
                TabContainerPlacement::Titlebar,
                palette,
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
            PaintRect::new(self.bounds, self.palette.surface_raised).with_border(Border::new(
                Edges::new(0.0, 0.0, 1.0, 0.0),
                self.palette.border,
            )),
        );
        context.draw_component(&self.tab_container);
        context.draw_component(&self.left_action_bar);
        context.draw_component(&self.right_action_bar);
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(
            PaintRect::new(self.bounds, self.palette.surface_raised).with_border(Border::new(
                Edges::new(0.0, 0.0, 1.0, 0.0),
                self.palette.border,
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
