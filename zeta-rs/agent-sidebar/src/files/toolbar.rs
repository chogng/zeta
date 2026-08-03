use zeta_icons::icons;
use zeta_ui::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle,
    ButtonSelection, ButtonState, CaretVisibility, Component, ComponentElement, Element, Rect,
    SearchBox, Size, TextInputLayoutEngine, UiScene,
};
use zui::{
    AccessibilityRole, CursorFeedback, FocusBehavior, InteractionFrame, NavigationAxis,
    NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use super::FilesState;
use crate::AgentSidebarStyle;
use crate::shell_interaction::{
    AGENT_FILE_SEARCH_INPUT, AGENT_FILES_ACTION_BAR, AGENT_FILES_REFRESH, AGENT_FILES_SEARCH,
    AGENT_SIDEBAR_TOOLBAR,
};

const PADDING: f32 = 8.0;
const ACTION_SIZE: f32 = 28.0;
const STATUS_WIDTH: f32 = 62.0;
const ACTION_BAR_WIDTH: f32 = ACTION_SIZE * 2.0 + STATUS_WIDTH;

/// Files-owned functional toolbar for refresh, search, and search input.
pub struct FilesToolbar {
    bounds: Rect,
    search_box: Option<SearchBox>,
    search_value: String,
    action_bar: ActionBar,
}

impl FilesToolbar {
    pub fn new(
        bounds: Rect,
        navigation_bounds: Rect,
        files: &FilesState,
        upstream_distance: Option<(usize, usize)>,
        caret_visibility: CaretVisibility,
        palette: AgentSidebarStyle,
        text_layout: &mut TextInputLayoutEngine,
        dispatch: &UiDispatch,
    ) -> Self {
        let action_bounds = Rect::from_xywh(
            (bounds.right() - ACTION_BAR_WIDTH - PADDING).max(bounds.origin.x),
            bounds.origin.y + (bounds.size.height - ACTION_SIZE) * 0.5,
            ACTION_BAR_WIDTH.min(bounds.size.width),
            ACTION_SIZE,
        );
        let button_style = palette.toolbar_button_style();
        let state = |id| {
            if dispatch.is_pressed(id) {
                ButtonState::Pressed
            } else if dispatch.is_focused(id) {
                ButtonState::Focused
            } else if dispatch.is_hovered(id) {
                ButtonState::Hovered
            } else {
                ButtonState::Resting
            }
        };
        let distance = upstream_distance
            .map(|(ahead, behind)| format!("↑{ahead} ↓{behind}"))
            .unwrap_or_else(|| "↑— ↓—".to_string());
        let action_bar = ActionBar::new(
            action_bounds,
            ActionBarOrientation::Horizontal,
            vec![
                ActionBarItem::Button(
                    ActionBarButton::icon_and_label(
                        icons::REFRESH,
                        distance,
                        state(AGENT_FILES_REFRESH),
                    )
                    .with_main_axis_extent(ACTION_SIZE + STATUS_WIDTH),
                ),
                ActionBarItem::Button(
                    ActionBarButton::icon(icons::SEARCH, "Search files", state(AGENT_FILES_SEARCH))
                        .with_selection(if files.search_visible() {
                            ButtonSelection::Selected
                        } else {
                            ButtonSelection::Unselected
                        }),
                ),
            ],
            ActionBarStyle::new(button_style, Size::new(ACTION_SIZE, ACTION_SIZE)),
        );
        let search_box = files.search_visible().then(|| {
            let search_bounds = Rect::from_xywh(
                navigation_bounds.right() + PADDING,
                bounds.origin.y + 6.0,
                (action_bounds.origin.x - navigation_bounds.right() - PADDING * 2.0).max(1.0),
                (bounds.size.height - 12.0).max(1.0),
            );
            let search_state = if dispatch.is_focused(AGENT_FILE_SEARCH_INPUT) {
                zeta_ui::InputBoxState::Focused(caret_visibility)
            } else if dispatch.is_hovered(AGENT_FILE_SEARCH_INPUT) {
                zeta_ui::InputBoxState::Hovered
            } else {
                zeta_ui::InputBoxState::Resting
            };
            SearchBox::new(
                search_bounds,
                "Search files...",
                search_state,
                palette.search_style(),
                files.search_input(),
                text_layout,
            )
        });
        Self {
            bounds,
            search_box,
            search_value: files.search_input().text().to_owned(),
            action_bar,
        }
    }

    pub fn register_interactions(&self, frame: &mut InteractionFrame) {
        if let Some(search_box) = self.search_box.as_ref() {
            frame.register(
                UiNode::new(
                    AGENT_FILE_SEARCH_INPUT,
                    search_box.bounds(),
                    AccessibilityRole::TextInput,
                    "Search files",
                )
                .with_parent(AGENT_SIDEBAR_TOOLBAR)
                .with_cursor(CursorFeedback::Text)
                .with_focus(FocusBehavior::TabStop)
                .with_value(&self.search_value),
            );
        }
        frame.register(
            UiNode::new(
                AGENT_FILES_ACTION_BAR,
                self.action_bar.bounds(),
                AccessibilityRole::Toolbar,
                "Files actions",
            )
            .with_parent(AGENT_SIDEBAR_TOOLBAR),
        );
        let navigation = NavigationGroupId::new(AGENT_FILES_ACTION_BAR);
        for (index, (id, label)) in [
            (AGENT_FILES_REFRESH, "Refresh Git status"),
            (AGENT_FILES_SEARCH, "Search files"),
        ]
        .into_iter()
        .enumerate()
        {
            let bounds = self
                .action_bar
                .interactive_item_bounds(index)
                .expect("Files toolbar actions are enabled");
            frame.register(
                UiNode::new(id, bounds, AccessibilityRole::Button, label)
                    .with_parent(AGENT_FILES_ACTION_BAR)
                    .with_cursor(CursorFeedback::Pointer)
                    .with_focus(FocusBehavior::TabStop)
                    .with_action(NodeAction::Activate)
                    .with_navigation(navigation, NavigationAxis::Horizontal),
            );
        }
    }

    pub const fn search_caret_bounds(&self) -> Option<Rect> {
        match self.search_box.as_ref() {
            Some(search_box) => search_box.caret_bounds(),
            None => None,
        }
    }
}

impl Component for FilesToolbar {
    fn element(&self) -> ComponentElement {
        Element::leaf("FilesToolbar").in_bounds(self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        if let Some(search_box) = self.search_box.as_ref() {
            scene.draw_component(search_box);
        }
        scene.draw_component(&self.action_bar);
    }
}
