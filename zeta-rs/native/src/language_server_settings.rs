use zeta_language_service::LanguageServerState;
use zeta_ui::{
    Border, Button, ButtonSelection, ButtonState, CaretVisibility, Color, Component,
    ComponentElement, CornerRadii, Element, FontWeight, InputBox, InputBoxState, PaintRect, Rect,
    TextBlock, TextInputLayoutEngine, TextStyle, UiScene,
};
use zui::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, ElementId, FocusBehavior,
    InteractionFrame, NavigationAxis, NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use crate::shell_interaction::WINDOW;
use crate::shell_style::ShellPalette;

#[path = "language_server_settings_model.rs"]
mod model;
pub(crate) use model::{LanguageServerSettingsState, LanguageServerSettingsTarget};

#[path = "language_server_settings_style.rs"]
mod presentation;
use presentation::{
    CONTENT_INSET, close_bounds, executable_bounds, input_style, mode_bounds, mode_button_style,
    mode_controls, primary_button_style, quiet_button_style, reset_bounds, save_bounds,
    server_bounds, server_controls,
};

const SETTINGS_SCOPE: u32 = 8;
pub(crate) const LANGUAGE_SERVER_SETTINGS: ElementId = ElementId::scoped(SETTINGS_SCOPE, 1);
pub(crate) const LANGUAGE_SERVER_SETTINGS_CLOSE: ElementId = ElementId::scoped(SETTINGS_SCOPE, 2);
pub(crate) const LANGUAGE_SERVER_MODE_DISABLED: ElementId = ElementId::scoped(SETTINGS_SCOPE, 3);
pub(crate) const LANGUAGE_SERVER_MODE_AUTOMATIC: ElementId = ElementId::scoped(SETTINGS_SCOPE, 4);
pub(crate) const LANGUAGE_SERVER_MODE_ENABLED: ElementId = ElementId::scoped(SETTINGS_SCOPE, 5);
pub(crate) const LANGUAGE_SERVER_EXECUTABLE_INPUT: ElementId = ElementId::scoped(SETTINGS_SCOPE, 6);
pub(crate) const LANGUAGE_SERVER_SETTINGS_RESET: ElementId = ElementId::scoped(SETTINGS_SCOPE, 7);
pub(crate) const LANGUAGE_SERVER_SETTINGS_SAVE: ElementId = ElementId::scoped(SETTINGS_SCOPE, 8);
pub(crate) const LANGUAGE_SERVER_RUST: ElementId = ElementId::scoped(SETTINGS_SCOPE, 9);
pub(crate) const LANGUAGE_SERVER_JSON: ElementId = ElementId::scoped(SETTINGS_SCOPE, 10);
pub(crate) const LANGUAGE_SERVER_BASH: ElementId = ElementId::scoped(SETTINGS_SCOPE, 11);

const PANEL_WIDTH: f32 = 560.0;
const PANEL_HEIGHT: f32 = 390.0;
const PANEL_MARGIN: f32 = 24.0;

pub(crate) struct LanguageServerSettings<'a> {
    viewport: Rect,
    panel: Rect,
    state: &'a LanguageServerSettingsState,
    palette: ShellPalette,
    executable_input: InputBox,
    dispatch: &'a UiDispatch,
    runtime_state: Option<&'a LanguageServerState>,
}

impl<'a> LanguageServerSettings<'a> {
    pub(crate) fn new(
        viewport: Rect,
        state: &'a LanguageServerSettingsState,
        caret_visibility: CaretVisibility,
        palette: ShellPalette,
        text_layout: &mut TextInputLayoutEngine,
        dispatch: &'a UiDispatch,
    ) -> Option<Self> {
        if !state.is_visible() {
            return None;
        }
        let width = PANEL_WIDTH.min((viewport.size.width - PANEL_MARGIN * 2.0).max(1.0));
        let height = PANEL_HEIGHT.min((viewport.size.height - PANEL_MARGIN * 2.0).max(1.0));
        let panel = Rect::from_xywh(
            viewport.origin.x + (viewport.size.width - width) * 0.5,
            viewport.origin.y + (viewport.size.height - height) * 0.5,
            width,
            height,
        );
        let input_state = if dispatch.is_focused(LANGUAGE_SERVER_EXECUTABLE_INPUT) {
            InputBoxState::Focused(caret_visibility)
        } else if dispatch.is_hovered(LANGUAGE_SERVER_EXECUTABLE_INPUT) {
            InputBoxState::Hovered
        } else {
            InputBoxState::Resting
        };
        let executable_input = InputBox::new(
            executable_bounds(panel),
            format!(
                "Optional absolute path to {}",
                state.selected_server().executable_name()
            ),
            input_state,
            input_style(palette),
            state.executable_input(),
            text_layout,
        );
        Some(Self {
            viewport,
            panel,
            state,
            palette,
            executable_input,
            dispatch,
            runtime_state: None,
        })
    }

    pub(crate) const fn with_runtime_state(
        mut self,
        runtime_state: &'a LanguageServerState,
    ) -> Self {
        self.runtime_state = Some(runtime_state);
        self
    }

    pub(crate) fn register_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                LANGUAGE_SERVER_SETTINGS,
                self.panel,
                AccessibilityRole::Group,
                "Language server settings",
            )
            .with_parent(WINDOW),
        );
        frame.set_modal_root(LANGUAGE_SERVER_SETTINGS);
        let navigation = NavigationGroupId::new(LANGUAGE_SERVER_SETTINGS);
        self.register_button(
            frame,
            LANGUAGE_SERVER_SETTINGS_CLOSE,
            close_bounds(self.panel),
            "Close language server settings",
            navigation,
        );
        for (id, target, label) in server_controls() {
            frame.register(
                UiNode::new(
                    id,
                    server_bounds(self.panel, target),
                    AccessibilityRole::Button,
                    label,
                )
                .with_parent(LANGUAGE_SERVER_SETTINGS)
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation, NavigationAxis::Horizontal)
                .with_selection(if self.state.selected_server() == target {
                    AccessibilitySelection::Selected
                } else {
                    AccessibilitySelection::Unselected
                }),
            );
        }
        for (id, mode, label) in mode_controls() {
            frame.register(
                UiNode::new(
                    id,
                    mode_bounds(self.panel, mode),
                    AccessibilityRole::Button,
                    label,
                )
                .with_parent(LANGUAGE_SERVER_SETTINGS)
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation, NavigationAxis::Horizontal)
                .with_selection(if self.state.mode() == mode {
                    AccessibilitySelection::Selected
                } else {
                    AccessibilitySelection::Unselected
                }),
            );
        }
        frame.register(
            UiNode::new(
                LANGUAGE_SERVER_EXECUTABLE_INPUT,
                self.executable_input.bounds(),
                AccessibilityRole::TextInput,
                format!(
                    "{} executable override",
                    self.state.selected_server().executable_name()
                ),
            )
            .with_parent(LANGUAGE_SERVER_SETTINGS)
            .with_cursor(CursorFeedback::Text)
            .with_focus(FocusBehavior::TabStop)
            .with_value(self.state.executable_input().text()),
        );
        self.register_button(
            frame,
            LANGUAGE_SERVER_SETTINGS_RESET,
            reset_bounds(self.panel),
            "Reset selected language server settings to product defaults",
            navigation,
        );
        self.register_button(
            frame,
            LANGUAGE_SERVER_SETTINGS_SAVE,
            save_bounds(self.panel),
            "Save selected language server settings",
            navigation,
        );
    }

    fn register_button(
        &self,
        frame: &mut InteractionFrame,
        id: ElementId,
        bounds: Rect,
        label: &str,
        navigation: NavigationGroupId,
    ) {
        frame.register(
            UiNode::new(id, bounds, AccessibilityRole::Button, label)
                .with_parent(LANGUAGE_SERVER_SETTINGS)
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation, NavigationAxis::Horizontal),
        );
    }

    pub(crate) const fn executable_caret_bounds(&self) -> Option<Rect> {
        self.executable_input.caret_bounds()
    }

    fn button_state(&self, id: ElementId, enabled: bool) -> ButtonState {
        if !enabled {
            ButtonState::Disabled
        } else if self.dispatch.is_pressed(id) {
            ButtonState::Pressed
        } else if self.dispatch.is_focused(id) {
            ButtonState::Focused
        } else if self.dispatch.is_hovered(id) {
            ButtonState::Hovered
        } else {
            ButtonState::Resting
        }
    }
}

impl Component for LanguageServerSettings<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("LanguageServerSettings").in_bounds(self.panel)
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(PaintRect::new(self.viewport, Color::rgba(0, 0, 0, 70)));
        scene.draw_rect(
            PaintRect::new(self.panel, self.palette.surface)
                .with_border(Border::uniform(1.0, self.palette.border))
                .with_corner_radii(CornerRadii::uniform(8.0)),
        );
        draw_text(
            scene,
            "Language Servers",
            Rect::from_xywh(
                self.panel.origin.x + CONTENT_INSET,
                self.panel.origin.y + 20.0,
                self.panel.size.width - CONTENT_INSET * 2.0,
                24.0,
            ),
            TextStyle::new(18.0, self.palette.text)
                .with_line_height(24.0)
                .with_weight(FontWeight::Bold),
        );
        draw_text(
            scene,
            self.state.selected_server().executable_name(),
            Rect::from_xywh(
                self.panel.origin.x + CONTENT_INSET,
                self.panel.origin.y + 111.0,
                240.0,
                20.0,
            ),
            TextStyle::new(14.0, self.palette.text)
                .with_line_height(20.0)
                .with_weight(FontWeight::Bold),
        );
        if let Some((label, color)) = self.runtime_status() {
            draw_text(
                scene,
                &label,
                Rect::from_xywh(
                    self.panel.right() - CONTENT_INSET - 190.0,
                    self.panel.origin.y + 111.0,
                    190.0,
                    20.0,
                ),
                TextStyle::new(12.0, color).with_line_height(20.0),
            );
        }
        draw_text(
            scene,
            "Automatic discovers the executable from the Native process environment.",
            Rect::from_xywh(
                self.panel.origin.x + CONTENT_INSET,
                self.panel.origin.y + 136.0,
                self.panel.size.width - CONTENT_INSET * 2.0,
                18.0,
            ),
            TextStyle::new(12.0, self.palette.text_muted).with_line_height(18.0),
        );
        for (id, target, label) in server_controls() {
            let button = Button::new(
                server_bounds(self.panel, target),
                label,
                self.button_state(id, true),
                mode_button_style(self.palette),
            )
            .with_selection(if self.state.selected_server() == target {
                ButtonSelection::Selected
            } else {
                ButtonSelection::Unselected
            });
            scene.draw_component(&button);
        }
        for (id, mode, label) in mode_controls() {
            let button = Button::new(
                mode_bounds(self.panel, mode),
                label,
                self.button_state(id, true),
                mode_button_style(self.palette),
            )
            .with_selection(if self.state.mode() == mode {
                ButtonSelection::Selected
            } else {
                ButtonSelection::Unselected
            });
            scene.draw_component(&button);
        }
        draw_text(
            scene,
            "Executable override",
            Rect::from_xywh(
                self.panel.origin.x + CONTENT_INSET,
                self.panel.origin.y + 210.0,
                200.0,
                18.0,
            ),
            TextStyle::new(12.0, self.palette.text_muted).with_line_height(18.0),
        );
        scene.draw_component(&self.executable_input);
        if let Some((message, is_error)) = self.state.status_message() {
            draw_text(
                scene,
                message,
                Rect::from_xywh(
                    self.panel.origin.x + CONTENT_INSET,
                    self.panel.origin.y + 282.0,
                    self.panel.size.width - CONTENT_INSET * 2.0,
                    18.0,
                ),
                TextStyle::new(
                    12.0,
                    if is_error {
                        self.palette.error
                    } else {
                        self.palette.text_muted
                    },
                )
                .with_line_height(18.0),
            );
        }
        scene.draw_component(&Button::new(
            close_bounds(self.panel),
            "×",
            self.button_state(LANGUAGE_SERVER_SETTINGS_CLOSE, true),
            quiet_button_style(self.palette),
        ));
        scene.draw_component(&Button::new(
            reset_bounds(self.panel),
            "Reset to Default",
            self.button_state(LANGUAGE_SERVER_SETTINGS_RESET, self.state.can_reset()),
            quiet_button_style(self.palette),
        ));
        scene.draw_component(&Button::new(
            save_bounds(self.panel),
            "Save",
            self.button_state(LANGUAGE_SERVER_SETTINGS_SAVE, self.state.can_save()),
            primary_button_style(self.palette),
        ));
    }
}

impl LanguageServerSettings<'_> {
    fn runtime_status(&self) -> Option<(String, Color)> {
        let (label, color) = match self.runtime_state? {
            LanguageServerState::Starting => ("Starting…".into(), self.palette.text_muted),
            LanguageServerState::Ready => ("Ready".into(), self.palette.accent),
            LanguageServerState::BackingOff {
                attempt,
                retry_after,
            } => (
                format!("Restart {attempt} in {} ms", retry_after.as_millis()),
                self.palette.warning,
            ),
            LanguageServerState::CrashLoop {
                restart_attempts, ..
            } => (
                format!("Crash loop after {restart_attempts} restarts"),
                self.palette.error,
            ),
            LanguageServerState::Failed(_) => ("Failed".into(), self.palette.error),
            LanguageServerState::Stopped => ("Stopped".into(), self.palette.text_muted),
        };
        Some((label, color))
    }
}

fn draw_text(scene: &mut UiScene, text: &str, bounds: Rect, style: TextStyle) {
    scene.draw_text(TextBlock::new(text, bounds.origin, bounds.size, style));
}

#[cfg(test)]
#[path = "language_server_settings_tests.rs"]
mod tests;
