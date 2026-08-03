use zeta_icons::icons;
#[cfg(test)]
use zeta_ui::Point;
use zeta_ui::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle, Border,
    ButtonBackgrounds, ButtonState, ButtonStyle, Component, ComponentElement, CornerRadii, Edges,
    Element, Rect, Size, TextInputLayoutEngine, TextStyle, UiScene,
};

use crate::agent_composer::ComposerMode;
use crate::shell_interaction::{COMPOSER_MODE, COMPOSER_PANEL, CONTEXT_TOOLBAR, ContextAction};
use crate::shell_style::ShellPalette;
use crate::workspace_context::WorkspaceContext;
use zui::{
    AccessibilityRole, CursorFeedback, FocusBehavior, InteractionFrame, NavigationAxis,
    NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

const TOOLBAR_ITEM_HEIGHT: f32 = 24.0;
const TOOLBAR_ITEM_GAP: f32 = 6.0;
const TOOLBAR_FONT_SIZE: f32 = 12.0;
const TOOLBAR_LINE_HEIGHT: f32 = 16.0;
const TOOLBAR_HORIZONTAL_PADDING: f32 = 7.0;
const TOOLBAR_ICON_SIZE: f32 = 14.0;
const TOOLBAR_CONTENT_GAP: f32 = 4.0;
/// Product-owned action toolbar shared by command and future chat input surfaces.
pub(crate) struct InputContextToolbar {
    action_bar: ActionBar,
    accessibility_labels: Vec<String>,
    bounds: Rect,
}

impl InputContextToolbar {
    pub(crate) fn new(
        bounds: Rect,
        context: &WorkspaceContext,
        mode: ComposerMode,
        palette: ShellPalette,
        text_layout: &mut TextInputLayoutEngine,
        dispatch: &UiDispatch,
    ) -> Self {
        let labels = [
            match mode {
                ComposerMode::Agent => "Agent".to_owned(),
                ComposerMode::Shell => "Shell".to_owned(),
            },
            context.location_label().to_string(),
            context.working_directory_label().to_string(),
            context.git_branch_label().to_string(),
            context.diff_summary_label(),
        ];
        let accessibility_labels = vec![
            format!("Composer mode: {}", labels[0]),
            format!("Environment: {}", labels[1]),
            format!("Working directory: {}", labels[2]),
            format!("Git branch: {}", labels[3]),
            format!("Workspace {}", labels[4]),
        ];
        let natural_text_style =
            TextStyle::new(TOOLBAR_FONT_SIZE, palette.accent).with_line_height(TOOLBAR_LINE_HEIGHT);
        let natural_button_style = button_style(palette, natural_text_style.clone(), 1.0);
        let item_widths = labels
            .iter()
            .map(|label| {
                let text_width = text_layout.measure_text(label, &natural_text_style).width;
                natural_button_style.preferred_icon_and_label_width(text_width)
            })
            .collect::<Vec<_>>();
        let total_width =
            item_widths.iter().sum::<f32>() + TOOLBAR_ITEM_GAP * (item_widths.len() - 1) as f32;
        let scale = (bounds.size.width / total_width).clamp(0.0, 1.0);
        let scaled_text_style = TextStyle::new(TOOLBAR_FONT_SIZE * scale, palette.accent)
            .with_line_height(TOOLBAR_LINE_HEIGHT * scale);
        let button_style = button_style(palette, scaled_text_style, scale);
        let item = |target, icon, label: String, index: usize| {
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
                ActionBarButton::icon_and_label(icon, label, state)
                    .with_main_axis_extent(item_widths[index] * scale),
            )
        };
        let mode_icon = match mode {
            ComposerMode::Agent => icons::AGENT,
            ComposerMode::Shell => icons::LOCAL,
        };
        let items = vec![
            item(COMPOSER_MODE, mode_icon, labels[0].clone(), 0),
            item(
                ContextAction::ALL[0].element_id(),
                icons::LOCAL,
                labels[1].clone(),
                1,
            ),
            item(
                ContextAction::ALL[1].element_id(),
                icons::WORKING_DIRECTORY,
                labels[2].clone(),
                2,
            ),
            item(
                ContextAction::ALL[2].element_id(),
                icons::GIT_BRANCH,
                labels[3].clone(),
                3,
            ),
            item(
                ContextAction::ALL[3].element_id(),
                icons::DIFF,
                labels[4].clone(),
                4,
            ),
        ];
        Self {
            action_bar: ActionBar::new(
                bounds,
                ActionBarOrientation::Horizontal,
                items,
                ActionBarStyle::new(
                    button_style,
                    Size::new(item_widths[0] * scale, TOOLBAR_ITEM_HEIGHT),
                )
                .with_gap(TOOLBAR_ITEM_GAP * scale),
            ),
            accessibility_labels,
            bounds,
        }
    }

    pub(crate) fn register_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                CONTEXT_TOOLBAR,
                self.bounds,
                AccessibilityRole::Toolbar,
                "Input context",
            )
            .with_parent(COMPOSER_PANEL),
        );
        let navigation_group = NavigationGroupId::new(CONTEXT_TOOLBAR);
        if let Some(bounds) = self.action_bar.interactive_item_bounds(0) {
            frame.register(
                UiNode::new(
                    COMPOSER_MODE,
                    bounds,
                    AccessibilityRole::Button,
                    self.accessibility_labels[0].clone(),
                )
                .with_parent(CONTEXT_TOOLBAR)
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation_group, NavigationAxis::Horizontal),
            );
        }
        for (index, action) in ContextAction::ALL.into_iter().enumerate() {
            let item_index = index + 1;
            if let Some(bounds) = self.action_bar.interactive_item_bounds(item_index) {
                frame.register(
                    UiNode::new(
                        action.element_id(),
                        bounds,
                        AccessibilityRole::Button,
                        self.accessibility_labels[item_index].clone(),
                    )
                    .with_parent(CONTEXT_TOOLBAR)
                    .with_cursor(CursorFeedback::Pointer)
                    .with_focus(FocusBehavior::TabStop)
                    .with_action(NodeAction::Activate)
                    .with_navigation(navigation_group, NavigationAxis::Horizontal),
                );
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn item_bounds(&self, index: usize) -> Option<Rect> {
        self.action_bar.item_bounds(index)
    }

    #[cfg(test)]
    pub(crate) fn hit_test(&self, point: Point) -> Option<usize> {
        self.action_bar.hit_test(point)
    }
}

fn button_style(palette: ShellPalette, text_style: TextStyle, scale: f32) -> ButtonStyle {
    ButtonStyle::new(
        ButtonBackgrounds::new(palette.surface_raised)
            .with_hovered(palette.surface_hovered)
            .with_focused(palette.surface_hovered)
            .with_pressed(palette.border),
        text_style,
    )
    .with_border(Border::uniform(1.0, palette.border))
    .with_corner_radii(CornerRadii::uniform(4.0))
    .with_padding(Edges::new(
        0.0,
        TOOLBAR_HORIZONTAL_PADDING * scale,
        0.0,
        TOOLBAR_HORIZONTAL_PADDING * scale,
    ))
    .with_icon_size(TOOLBAR_ICON_SIZE * scale)
    .with_content_gap(TOOLBAR_CONTENT_GAP * scale)
}

impl Component for InputContextToolbar {
    fn element(&self) -> ComponentElement {
        Element::leaf("InputContextToolbar").in_bounds(self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_component(&self.action_bar);
    }
}

#[cfg(test)]
#[path = "input_context_toolbar_tests.rs"]
mod tests;
