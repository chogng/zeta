use zeta_icons::icons;
use zeta_ui::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle, Border,
    ButtonBackgrounds, ButtonState, ButtonStyle, Component, ComponentElement, CornerRadii, Edges,
    Element, PaintRect, Rect, Size, TextStyle, UiScene,
};

use crate::agent_sidebar::AgentSidebarState;
use crate::session_sidebar::SessionSidebarState;
use crate::shell_interaction::{
    AGENT_SIDEBAR_TOGGLE, LANGUAGE_SERVER_SETTINGS_TOGGLE, SESSION_SIDEBAR_TOGGLE, TITLEBAR, WINDOW,
};
use crate::shell_style::ShellPalette;
use zeta_ui_dispatch::{
    AccessibilityRole, CursorFeedback, FocusBehavior, InteractionFrame, NodeAction, UiDispatch,
    UiNode,
};
use zeta_winit::WindowControlInsets;

pub(crate) const TITLEBAR_HEIGHT: f32 = 32.0;
const TITLEBAR_ACTION_GAP: f32 = 8.0;
const TOGGLE_SIZE: f32 = 24.0;
const TOGGLE_ICON_SIZE: f32 = 18.0;

/// Product-owned draggable titlebar for the single terminal surface.
pub(crate) struct Titlebar {
    bounds: Rect,
    palette: ShellPalette,
    left_action_bar: ActionBar,
    right_action_bar: ActionBar,
    session_toggle_label: &'static str,
    language_server_settings_label: &'static str,
    agent_toggle_label: &'static str,
}

impl Titlebar {
    pub(crate) fn new(
        bounds: Rect,
        palette: ShellPalette,
        session_sidebar: SessionSidebarState,
        agent_sidebar: AgentSidebarState,
        window_control_insets: WindowControlInsets,
        dispatch: &UiDispatch,
    ) -> Self {
        let content_left = bounds.origin.x + window_control_insets.left();
        let content_right = (bounds.right() - window_control_insets.right()).max(content_left);
        let session_toggle_x = (content_left + TITLEBAR_ACTION_GAP)
            .min((content_right - TOGGLE_SIZE).max(content_left));
        let session_toggle_bounds = Rect::from_xywh(
            session_toggle_x,
            bounds.origin.y + (bounds.size.height - TOGGLE_SIZE) / 2.0,
            TOGGLE_SIZE,
            TOGGLE_SIZE,
        );
        let agent_toggle_max_x = (content_right - TOGGLE_SIZE).max(content_left);
        let agent_toggle_x = (content_right - TITLEBAR_ACTION_GAP - TOGGLE_SIZE)
            .max(session_toggle_bounds.right() + TITLEBAR_ACTION_GAP)
            .min(agent_toggle_max_x);
        let agent_toggle_bounds = Rect::from_xywh(
            agent_toggle_x,
            bounds.origin.y + (bounds.size.height - TOGGLE_SIZE) / 2.0,
            TOGGLE_SIZE,
            TOGGLE_SIZE,
        );
        let settings_toggle_state = if dispatch.is_pressed(LANGUAGE_SERVER_SETTINGS_TOGGLE) {
            ButtonState::Pressed
        } else if dispatch.is_focused(LANGUAGE_SERVER_SETTINGS_TOGGLE) {
            ButtonState::Focused
        } else if dispatch.is_hovered(LANGUAGE_SERVER_SETTINGS_TOGGLE) {
            ButtonState::Hovered
        } else {
            ButtonState::Resting
        };
        let session_toggle_state = if dispatch.is_pressed(SESSION_SIDEBAR_TOGGLE) {
            ButtonState::Pressed
        } else if dispatch.is_focused(SESSION_SIDEBAR_TOGGLE) {
            ButtonState::Focused
        } else if dispatch.is_hovered(SESSION_SIDEBAR_TOGGLE) {
            ButtonState::Hovered
        } else {
            ButtonState::Resting
        };
        let agent_toggle_state = if dispatch.is_pressed(AGENT_SIDEBAR_TOGGLE) {
            ButtonState::Pressed
        } else if dispatch.is_focused(AGENT_SIDEBAR_TOGGLE) {
            ButtonState::Focused
        } else if dispatch.is_hovered(AGENT_SIDEBAR_TOGGLE) {
            ButtonState::Hovered
        } else {
            ButtonState::Resting
        };
        let session_toggle_label = if session_sidebar.is_expanded() {
            "Collapse sessions sidebar"
        } else {
            "Expand sessions sidebar"
        };
        let session_toggle_icon = if session_sidebar.is_expanded() {
            icons::LAYOUT_SIDEBAR_LEFT
        } else {
            icons::LAYOUT_SIDEBAR_LEFT_OFF_EMPTY
        };
        let agent_toggle_label = if agent_sidebar.is_expanded() {
            "Collapse agent sidebar"
        } else {
            "Expand agent sidebar"
        };
        let agent_toggle_icon = if agent_sidebar.is_expanded() {
            icons::LAYOUT_SIDEBAR_RIGHT
        } else {
            icons::LAYOUT_SIDEBAR_RIGHT_OFF_EMPTY
        };
        let language_server_settings_label = "Language server settings";
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
        Self {
            bounds,
            palette,
            left_action_bar: ActionBar::new(
                session_toggle_bounds,
                ActionBarOrientation::Horizontal,
                vec![ActionBarItem::Button(ActionBarButton::icon(
                    session_toggle_icon,
                    session_toggle_label,
                    session_toggle_state,
                ))],
                ActionBarStyle::new(button_style.clone(), Size::new(TOGGLE_SIZE, TOGGLE_SIZE)),
            ),
            right_action_bar: ActionBar::new(
                Rect::from_xywh(
                    agent_toggle_bounds.origin.x - TOGGLE_SIZE - TITLEBAR_ACTION_GAP,
                    agent_toggle_bounds.origin.y,
                    TOGGLE_SIZE * 2.0 + TITLEBAR_ACTION_GAP,
                    TOGGLE_SIZE,
                ),
                ActionBarOrientation::Horizontal,
                vec![
                    ActionBarItem::Button(ActionBarButton::icon(
                        icons::SETTINGS,
                        language_server_settings_label,
                        settings_toggle_state,
                    )),
                    ActionBarItem::Button(ActionBarButton::icon(
                        agent_toggle_icon,
                        agent_toggle_label,
                        agent_toggle_state,
                    )),
                ],
                ActionBarStyle::new(button_style, Size::new(TOGGLE_SIZE, TOGGLE_SIZE))
                    .with_gap(TITLEBAR_ACTION_GAP),
            ),
            session_toggle_label,
            language_server_settings_label,
            agent_toggle_label,
        }
    }

    pub(crate) fn register_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                TITLEBAR,
                self.bounds,
                AccessibilityRole::Group,
                "Window titlebar",
            )
            .with_parent(WINDOW)
            .with_action(NodeAction::StartWindowDrag),
        );
        if let Some(bounds) = self.left_action_bar.interactive_item_bounds(0) {
            frame.register(
                UiNode::new(
                    SESSION_SIDEBAR_TOGGLE,
                    bounds,
                    AccessibilityRole::Button,
                    self.session_toggle_label,
                )
                .with_parent(TITLEBAR)
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate),
            );
        }
        if let Some(bounds) = self.right_action_bar.interactive_item_bounds(0) {
            frame.register(
                UiNode::new(
                    LANGUAGE_SERVER_SETTINGS_TOGGLE,
                    bounds,
                    AccessibilityRole::Button,
                    self.language_server_settings_label,
                )
                .with_parent(TITLEBAR)
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate),
            );
        }
        if let Some(bounds) = self.right_action_bar.interactive_item_bounds(1) {
            frame.register(
                UiNode::new(
                    AGENT_SIDEBAR_TOGGLE,
                    bounds,
                    AccessibilityRole::Button,
                    self.agent_toggle_label,
                )
                .with_parent(TITLEBAR)
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate),
            );
        }
    }
}

impl Component for Titlebar {
    fn element(&self) -> ComponentElement {
        Element::leaf("Titlebar").in_bounds(self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(
            PaintRect::new(self.bounds, self.palette.surface_raised).with_border(Border::new(
                Edges::new(0.0, 0.0, 1.0, 0.0),
                self.palette.border,
            )),
        );
        scene.draw_component(&self.left_action_bar);
        scene.draw_component(&self.right_action_bar);
    }
}

#[cfg(test)]
#[path = "titlebar_tests.rs"]
mod tests;
