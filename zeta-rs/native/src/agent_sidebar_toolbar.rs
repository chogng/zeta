use zeta_icons::icons;
use zeta_ui::{
    ActionBar, ActionBarButton, ActionBarItem, ActionBarOrientation, ActionBarStyle, Border,
    ButtonBackgrounds, ButtonSelection, ButtonState, ButtonStyle, CaretVisibility, Component,
    CornerRadii, Edges, PaintRect, Rect, SearchBox, Size, TextInputLayoutEngine, TextStyle,
    UiScene,
};
use zeta_ui_dispatch::{
    AccessibilityRole, CursorFeedback, FocusBehavior, InteractionFrame, NavigationAxis,
    NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use crate::agent_sidebar_navigation::AgentSidebarNavigation;
use crate::agent_sidebar_workspace::{AgentSidebarView, AgentSidebarWorkspace};
use crate::shell_interaction::{
    AGENT_FILE_SEARCH_INPUT, AGENT_FILES_ACTION_BAR, AGENT_FILES_REFRESH, AGENT_FILES_SEARCH,
    AGENT_SIDEBAR, AGENT_SIDEBAR_TOOLBAR,
};
use crate::shell_style::ShellPalette;
use crate::workspace_context::WorkspaceContext;

const PADDING: f32 = 8.0;
const ACTION_SIZE: f32 = 28.0;
const STATUS_WIDTH: f32 = 62.0;
const ACTION_BAR_WIDTH: f32 = ACTION_SIZE * 2.0 + STATUS_WIDTH;

/// Top toolbar containing the pane switcher and active-pane actions.
pub(crate) struct AgentSidebarToolbar {
    bounds: Rect,
    palette: ShellPalette,
    navigation: AgentSidebarNavigation,
    search_box: Option<SearchBox>,
    search_value: String,
    action_bar: Option<ActionBar>,
}

impl AgentSidebarToolbar {
    pub(crate) fn new(
        bounds: Rect,
        workspace: &AgentSidebarWorkspace,
        context: &WorkspaceContext,
        caret_visibility: CaretVisibility,
        palette: ShellPalette,
        text_layout: &mut TextInputLayoutEngine,
        dispatch: &UiDispatch,
    ) -> Self {
        let navigation = AgentSidebarNavigation::new(
            AgentSidebarNavigation::bounds_in(bounds),
            workspace.active_view(),
            palette,
            dispatch,
        );
        if workspace.active_view() == AgentSidebarView::Changes {
            return Self {
                bounds,
                palette,
                navigation,
                search_box: None,
                search_value: String::new(),
                action_bar: None,
            };
        }
        let action_bounds = Rect::from_xywh(
            (bounds.right() - ACTION_BAR_WIDTH - PADDING).max(bounds.origin.x),
            bounds.origin.y + (bounds.size.height - ACTION_SIZE) * 0.5,
            ACTION_BAR_WIDTH.min(bounds.size.width),
            ACTION_SIZE,
        );
        let button_backgrounds = ButtonBackgrounds::new(zeta_ui::Color::TRANSPARENT)
            .with_hovered(palette.surface_hovered)
            .with_focused(palette.surface_hovered)
            .with_pressed(palette.session_tab_highlight);
        let selected_backgrounds = ButtonBackgrounds::new(palette.session_tab_highlight)
            .with_hovered(palette.session_tab_highlight)
            .with_focused(palette.session_tab_highlight)
            .with_pressed(palette.session_tab_highlight);
        let button_style = ButtonStyle::new(button_backgrounds, TextStyle::new(11.0, palette.text))
            .with_selected_backgrounds(selected_backgrounds)
            .with_corner_radii(CornerRadii::uniform(4.0))
            .with_padding(Edges::uniform(4.0))
            .with_icon_size(16.0);
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
        let distance = context
            .upstream_distance()
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
                        .with_selection(if workspace.search_visible() {
                            ButtonSelection::Selected
                        } else {
                            ButtonSelection::Unselected
                        }),
                ),
            ],
            ActionBarStyle::new(button_style, Size::new(ACTION_SIZE, ACTION_SIZE)),
        );
        let search_box = workspace.search_visible().then(|| {
            let navigation_bounds = AgentSidebarNavigation::bounds_in(bounds);
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
                palette.session_search_style(),
                workspace.file_search_input(),
                text_layout,
            )
        });
        Self {
            bounds,
            palette,
            navigation,
            search_box,
            search_value: workspace.file_search_input().text().to_owned(),
            action_bar: Some(action_bar),
        }
    }

    pub(crate) fn register_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                AGENT_SIDEBAR_TOOLBAR,
                self.bounds,
                AccessibilityRole::Toolbar,
                "Agent sidebar toolbar",
            )
            .with_parent(AGENT_SIDEBAR),
        );
        self.navigation.register_interactions(frame);
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
        let Some(action_bar) = self.action_bar.as_ref() else {
            return;
        };
        frame.register(
            UiNode::new(
                AGENT_FILES_ACTION_BAR,
                action_bar.bounds(),
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
            let item_index = index;
            let bounds = action_bar
                .interactive_item_bounds(item_index)
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

    pub(crate) const fn search_caret_bounds(&self) -> Option<Rect> {
        match self.search_box.as_ref() {
            Some(search_box) => search_box.caret_bounds(),
            None => None,
        }
    }
}

impl Component for AgentSidebarToolbar {
    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(
            PaintRect::new(self.bounds, self.palette.surface_raised).with_border(Border::new(
                Edges::new(0.0, 0.0, 1.0, 0.0),
                self.palette.border,
            )),
        );
        self.navigation.paint(scene);
        if let Some(search_box) = self.search_box.as_ref() {
            search_box.paint(scene);
        }
        if let Some(action_bar) = self.action_bar.as_ref() {
            action_bar.paint(scene);
        }
    }
}
