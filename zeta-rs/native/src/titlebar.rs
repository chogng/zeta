use zeta_icons::icons;
use zeta_ui::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle, Border,
    ButtonBackgrounds, ButtonState, ButtonStyle, Component, CornerRadii, Edges, PaintRect, Rect,
    Size, TextStyle, UiScene,
};

use crate::shell_interaction::{SIDEBAR_TOGGLE, SessionSidebarState, TITLEBAR, WINDOW};
use crate::shell_style::ShellPalette;
use zeta_ui_dispatch::{
    AccessibilityRole, CursorFeedback, FocusBehavior, InteractionFrame, NodeAction, UiDispatch,
    UiNode,
};
use zeta_winit::WindowControlInsets;

pub(crate) const TITLEBAR_HEIGHT: f32 = 32.0;
const TITLEBAR_ACTION_GAP: f32 = 8.0;
const TOGGLE_SIZE: f32 = 28.0;
const TOGGLE_ICON_SIZE: f32 = 18.0;

/// Product-owned draggable titlebar for the single terminal surface.
pub(crate) struct Titlebar {
    bounds: Rect,
    palette: ShellPalette,
    action_bar: ActionBar,
    toggle_label: &'static str,
}

impl Titlebar {
    pub(crate) fn new(
        bounds: Rect,
        palette: ShellPalette,
        session_sidebar: SessionSidebarState,
        window_control_insets: WindowControlInsets,
        dispatch: &UiDispatch,
    ) -> Self {
        let content_left = bounds.origin.x + window_control_insets.left();
        let content_right = (bounds.right() - window_control_insets.right()).max(content_left);
        let toggle_x = (content_left + TITLEBAR_ACTION_GAP)
            .min((content_right - TOGGLE_SIZE).max(content_left));
        let toggle_bounds = Rect::from_xywh(
            toggle_x,
            bounds.origin.y + (bounds.size.height - TOGGLE_SIZE) / 2.0,
            TOGGLE_SIZE,
            TOGGLE_SIZE,
        );
        let state = if dispatch.is_pressed(SIDEBAR_TOGGLE) {
            ButtonState::Pressed
        } else if dispatch.is_focused(SIDEBAR_TOGGLE) {
            ButtonState::Focused
        } else if dispatch.is_hovered(SIDEBAR_TOGGLE) {
            ButtonState::Hovered
        } else {
            ButtonState::Resting
        };
        let toggle_label = if session_sidebar.is_expanded() {
            "Collapse sessions sidebar"
        } else {
            "Expand sessions sidebar"
        };
        let icon = if session_sidebar.is_expanded() {
            icons::LAYOUT_SIDEBAR_LEFT_OFF
        } else {
            icons::LAYOUT_SIDEBAR_LEFT_EMPTY
        };
        let button_style = ButtonStyle::new(
            ButtonBackgrounds::new(palette.surface_raised)
                .with_hovered(palette.surface_hovered)
                .with_focused(palette.surface_hovered)
                .with_pressed(palette.border),
            TextStyle::new(12.0, palette.text_muted),
        )
        .with_corner_radii(CornerRadii::uniform(4.0))
        .with_padding(Edges::uniform(5.0))
        .with_icon_size(TOGGLE_ICON_SIZE);
        Self {
            bounds,
            palette,
            action_bar: ActionBar::new(
                toggle_bounds,
                ActionBarOrientation::Horizontal,
                vec![ActionBarItem::Button(ActionBarButton::icon(
                    icon,
                    toggle_label,
                    state,
                ))],
                ActionBarStyle::new(button_style, Size::new(TOGGLE_SIZE, TOGGLE_SIZE)),
            ),
            toggle_label,
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
        if let Some(bounds) = self.action_bar.interactive_item_bounds(0) {
            frame.register(
                UiNode::new(
                    SIDEBAR_TOGGLE,
                    bounds,
                    AccessibilityRole::Button,
                    self.toggle_label,
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
    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(
            PaintRect::new(self.bounds, self.palette.surface_raised).with_border(Border::new(
                Edges::new(0.0, 0.0, 1.0, 0.0),
                self.palette.border,
            )),
        );
        self.action_bar.paint(scene);
    }
}

#[cfg(test)]
#[path = "titlebar_tests.rs"]
mod tests;
