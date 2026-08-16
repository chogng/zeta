use std::path::Path;
use std::path::PathBuf;

use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_ui::{
    Color, Component, ComponentContext, ComponentElement, ComputedElement, CornerRadii, Element,
    FontWeight, InteractionRegion, PaintRect, Rect, Size, Tab, TabBackgrounds, TabList,
    TabListOrientation, TabListStyle, TabSelection, TabState, TabStyle, TextBlock, TextStyle,
    UiScene,
};
use zui::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, ElementId, FocusBehavior,
    NavigationAxis, NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use crate::shell_interaction::{SESSION_SIDEBAR, SESSION_TAB_LIST, session_tab_id};
use crate::shell_style::ShellPalette;

const TAB_HEIGHT: f32 = 52.0;
const TAB_CONTENT_PADDING: f32 = 8.0;
const TAB_INFORMATION_HEIGHT: f32 = 36.0;
const STATUS_CONTAINER_SIZE: f32 = TAB_INFORMATION_HEIGHT;
const STATUS_CONTENT_GAP: f32 = 10.0;

#[derive(Clone, Copy)]
pub(crate) struct SessionTab<'a> {
    id: ElementId,
    name: &'a str,
    workspace: &'a str,
    status_label: &'a str,
}

/// Product-owned presentation record for one App Server session tab.
///
/// The record keeps only the tab identity and labels needed by the shell projection. Session
/// lifecycle and Thread state remain owned by the App Server session adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SessionTabState {
    id: ElementId,
    session_id: SessionId,
    title: String,
    workspace: String,
    workspace_root: Option<PathBuf>,
    status_label: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SessionTabUpsert {
    Added(ElementId),
    Updated(ElementId),
}

impl SessionTabState {
    pub(crate) fn new(
        id: ElementId,
        session_id: SessionId,
        title: impl Into<String>,
        workspace: impl Into<String>,
        status_label: impl Into<String>,
    ) -> Self {
        Self {
            id,
            session_id,
            title: title.into(),
            workspace: workspace.into(),
            workspace_root: None,
            status_label: status_label.into(),
        }
    }

    pub(crate) const fn id(&self) -> ElementId {
        self.id
    }

    pub(crate) fn session_id(&self) -> &SessionId {
        &self.session_id
    }

    pub(crate) fn title(&self) -> &str {
        &self.title
    }

    pub(crate) fn workspace(&self) -> &str {
        &self.workspace
    }

    pub(crate) fn workspace_root(&self) -> Option<&Path> {
        self.workspace_root.as_deref()
    }

    pub(crate) fn status_label(&self) -> &str {
        &self.status_label
    }

    pub(crate) fn update_labels(
        &mut self,
        title: impl Into<String>,
        workspace: impl Into<String>,
        status_label: impl Into<String>,
    ) {
        self.title = title.into();
        self.workspace = workspace.into();
        self.status_label = status_label.into();
    }

    pub(crate) fn update_status(&mut self, status_label: impl Into<String>) {
        self.status_label = status_label.into();
    }

    fn update_workspace_root(&mut self, workspace_root: Option<PathBuf>) {
        self.workspace_root = workspace_root;
    }
}

/// Adds or updates the product-owned tab projection for one authoritative App Server Session.
///
/// The selected tab is updated together with the projection because a Session snapshot is only
/// published after the Agent worker has made that Session the active subscription. Terminal pane
/// allocation remains owned by [`crate::terminal_workspace::TerminalWorkspace`], so this helper
/// only establishes the stable UI identity that the pane can bind to.
pub(crate) fn upsert_session_tab(
    tabs: &mut Vec<SessionTabState>,
    selected: &mut ElementId,
    session: &Session,
    workspace: &str,
) -> SessionTabUpsert {
    if let Some(tab) = tabs
        .iter_mut()
        .find(|tab| tab.session_id() == &session.session_id)
    {
        let tab_id = tab.id();
        tab.update_labels(
            session.title.clone(),
            workspace_label(session, workspace),
            "Active",
        );
        tab.update_workspace_root(
            session
                .workspace
                .as_ref()
                .map(|binding| binding.root.clone()),
        );
        *selected = tab_id;
        return SessionTabUpsert::Updated(tab_id);
    }

    let tab_id = session_tab_id(tabs.len());
    let mut tab = SessionTabState::new(
        tab_id,
        session.session_id.clone(),
        session.title.clone(),
        workspace_label(session, workspace),
        "Active",
    );
    tab.update_workspace_root(
        session
            .workspace
            .as_ref()
            .map(|binding| binding.root.clone()),
    );
    tabs.push(tab);
    *selected = tab_id;
    SessionTabUpsert::Added(tab_id)
}

/// Adds or updates one catalog entry without changing the active tab selection.
pub(crate) fn upsert_session_catalog_tab(
    tabs: &mut Vec<SessionTabState>,
    session: &Session,
    workspace: &str,
) -> SessionTabUpsert {
    if let Some(tab) = tabs
        .iter_mut()
        .find(|tab| tab.session_id() == &session.session_id)
    {
        let tab_id = tab.id();
        tab.update_labels(
            session.title.clone(),
            workspace_label(session, workspace),
            "Active",
        );
        tab.update_workspace_root(
            session
                .workspace
                .as_ref()
                .map(|binding| binding.root.clone()),
        );
        return SessionTabUpsert::Updated(tab_id);
    }

    let tab_id = session_tab_id(tabs.len());
    let mut tab = SessionTabState::new(
        tab_id,
        session.session_id.clone(),
        session.title.clone(),
        workspace_label(session, workspace),
        "Active",
    );
    tab.update_workspace_root(
        session
            .workspace
            .as_ref()
            .map(|binding| binding.root.clone()),
    );
    tabs.push(tab);
    SessionTabUpsert::Added(tab_id)
}

fn workspace_label<'a>(session: &'a Session, fallback: &'a str) -> String {
    session
        .workspace
        .as_ref()
        .and_then(|binding| binding.root.file_name())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| fallback.to_owned())
}

impl<'a> SessionTab<'a> {
    pub(crate) const fn new(
        id: ElementId,
        name: &'a str,
        workspace: &'a str,
        status_label: &'a str,
    ) -> Self {
        Self {
            id,
            name,
            workspace,
            status_label,
        }
    }
}

/// Product-owned vertical TabList for real terminal sessions.
pub(crate) struct SessionTabList<'a> {
    bounds: Rect,
    tabs: &'a [SessionTab<'a>],
    selected_id: ElementId,
    palette: ShellPalette,
    dispatch: &'a UiDispatch,
}

impl<'a> SessionTabList<'a> {
    pub(crate) fn new(
        bounds: Rect,
        tabs: &'a [SessionTab<'a>],
        selected_id: ElementId,
        palette: ShellPalette,
        dispatch: &'a UiDispatch,
    ) -> Self {
        Self {
            bounds,
            tabs,
            selected_id,
            palette,
            dispatch,
        }
    }

    fn child_interaction_regions(&self) -> Vec<InteractionRegion> {
        let mut regions = Vec::new();
        let tab_list = self.tab_list();
        for (index, tab) in self.tabs.iter().enumerate() {
            let tab_bounds = tab_list.tab_bounds(index).expect("registered tab");
            regions.push(
                InteractionRegion::new(
                    "SessionTab",
                    tab.id,
                    tab_bounds,
                    AccessibilityRole::Tab,
                    format!("{}, {}, {}", tab.name, tab.workspace, tab.status_label),
                )
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(
                    NavigationGroupId::new(SESSION_TAB_LIST),
                    NavigationAxis::Vertical,
                )
                .with_selection(if tab.id == self.selected_id {
                    AccessibilitySelection::Selected
                } else {
                    AccessibilitySelection::Unselected
                }),
            );
        }
        regions
    }

    fn tab_list(&self) -> TabList {
        let highlight = self.palette.session_tab_highlight;
        let backgrounds = TabBackgrounds::new(Color::TRANSPARENT)
            .with_hovered(highlight)
            .with_focused(highlight)
            .with_pressed(highlight);
        let selected_backgrounds = TabBackgrounds::new(highlight);
        let tab_style = TabStyle::new(backgrounds)
            .with_selected_backgrounds(selected_backgrounds)
            .with_corner_radii(CornerRadii::uniform(4.0));
        let tabs = self
            .tabs
            .iter()
            .map(|tab| {
                Tab::new(self.tab_state(tab.id)).with_selection(if tab.id == self.selected_id {
                    TabSelection::Selected
                } else {
                    TabSelection::Unselected
                })
            })
            .collect();
        TabList::new(
            self.bounds,
            TabListOrientation::Vertical,
            tabs,
            TabListStyle::new(tab_style, Size::new(self.bounds.size.width, TAB_HEIGHT))
                .with_gap(6.0),
        )
    }

    fn tab_state(&self, id: ElementId) -> TabState {
        if self.dispatch.is_pressed(id) {
            TabState::Pressed
        } else if self.dispatch.is_focused(id) {
            TabState::Focused
        } else if self.dispatch.is_hovered(id) {
            TabState::Hovered
        } else {
            TabState::Resting
        }
    }

    fn paint_status(&self, scene: &mut UiScene, tab_list: &TabList) {
        for (index, tab) in self.tabs.iter().enumerate() {
            let tab_bounds = tab_list.tab_bounds(index).expect("painted tab");
            let status_bounds = Rect::from_xywh(
                tab_bounds.origin.x + TAB_CONTENT_PADDING,
                tab_bounds.origin.y + (tab_bounds.size.height - STATUS_CONTAINER_SIZE) * 0.5,
                STATUS_CONTAINER_SIZE,
                STATUS_CONTAINER_SIZE,
            );
            // Keep this white status container independent from Session lifecycle data. Planning,
            // Thinking, Editing, and any later Session states can project their own SVG inside it
            // once the App Server exposes an authoritative typed activity status.
            scene.draw_rect(
                PaintRect::new(status_bounds, self.palette.surface)
                    .with_corner_radii(CornerRadii::uniform(STATUS_CONTAINER_SIZE * 0.5)),
            );

            let text_x = status_bounds.right() + STATUS_CONTENT_GAP;
            let text_width = (tab_bounds.right() - text_x - TAB_CONTENT_PADDING).max(1.0);
            scene.draw_text(TextBlock::new(
                tab.name,
                zeta_ui::Point::new(text_x, tab_bounds.origin.y + 7.0),
                zeta_ui::Size::new(text_width, 18.0),
                TextStyle::new(13.0, self.palette.text).with_weight(FontWeight::Bold),
            ));
            scene.draw_text(TextBlock::new(
                tab.workspace,
                zeta_ui::Point::new(text_x, tab_bounds.origin.y + 27.0),
                zeta_ui::Size::new(text_width, 15.0),
                TextStyle::new(11.0, self.palette.text_muted).with_line_height(15.0),
            ));
        }
    }
}

impl Component for SessionTabList<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("SessionTabList")
            .in_bounds(self.bounds)
            .with_identity(SESSION_TAB_LIST)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                SESSION_TAB_LIST,
                element.bounds(),
                AccessibilityRole::TabList,
                "Terminal sessions",
            )
            .with_parent(SESSION_SIDEBAR),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        for region in self.child_interaction_regions() {
            context.draw_component(&region);
        }
        let tab_list = self.tab_list();
        context.draw_component(&tab_list);
        self.paint_status(context.scene_mut(), &tab_list);
    }

    fn paint(&self, scene: &mut UiScene) {
        let tab_list = self.tab_list();
        scene.draw_component(&tab_list);
        self.paint_status(scene, &tab_list);
    }
}

#[cfg(test)]
#[path = "session_tab_list_tests.rs"]
mod tests;
