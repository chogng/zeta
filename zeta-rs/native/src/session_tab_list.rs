use zeta_ui::{
    Color, Component, ComponentElement, CornerRadii, Element, FontWeight, PaintRect, Rect, Size,
    Tab, TabBackgrounds, TabList, TabListOrientation, TabListStyle, TabSelection, TabState,
    TabStyle, TextBlock, TextStyle, UiScene,
};
use zui::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, ElementId, FocusBehavior,
    InteractionFrame, NavigationAxis, NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use crate::shell_interaction::{SESSION_SIDEBAR, SESSION_TAB_LIST};
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

    pub(crate) fn register_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                SESSION_TAB_LIST,
                self.bounds,
                AccessibilityRole::TabList,
                "Terminal sessions",
            )
            .with_parent(SESSION_SIDEBAR),
        );
        let tab_list = self.tab_list();
        for (index, tab) in self.tabs.iter().enumerate() {
            let tab_bounds = tab_list.tab_bounds(index).expect("registered tab");
            frame.register(
                UiNode::new(
                    tab.id,
                    tab_bounds,
                    AccessibilityRole::Tab,
                    format!("{}, {}, {}", tab.name, tab.workspace, tab.status_label),
                )
                .with_parent(SESSION_TAB_LIST)
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
}

impl Component for SessionTabList<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("SessionTabList").in_bounds(self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        let tab_list = self.tab_list();
        scene.draw_component(&tab_list);
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

#[cfg(test)]
#[path = "session_tab_list_tests.rs"]
mod tests;
