//! Workbench tab-list projection for Sessions and Settings.

use zeta_icons::icons;
use zeta_ui::{
    Color, Component, ComponentContext, ComponentElement, ComputedElement, CornerRadii, Element,
    FontWeight, InteractionRegion, PaintIcon, PaintRect, Rect, Size, Tab, TabBackgrounds, TabList,
    TabListOrientation, TabListStyle, TabSelection, TabState, TabStyle, TextBlock, TextStyle,
    UiScene,
};
use zui::ui::{
    AccessibilityRole, AccessibilitySelection, CursorFeedback, ElementId, FocusBehavior,
    NavigationAxis, NavigationGroupId, NodeAction, UiDispatch, UiNode,
};

use crate::shell_interaction::{
    SESSION_SIDEBAR, SESSION_TAB_LIST, SETTINGS_WORKBENCH_TAB, session_tab_id,
};
use crate::shell_style::ShellPalette;
use crate::tab_input::TabInput;
use crate::tab_input::TabInputKey;

const TAB_HEIGHT: f32 = 52.0;
const TAB_CONTENT_PADDING: f32 = 8.0;
const TAB_INFORMATION_HEIGHT: f32 = 36.0;
const STATUS_CONTAINER_SIZE: f32 = TAB_INFORMATION_HEIGHT;
const STATUS_CONTENT_GAP: f32 = 10.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkbenchTabKind {
    Session,
    Settings,
}

#[derive(Clone, Copy)]
pub(crate) struct WorkbenchTab<'a> {
    id: ElementId,
    kind: WorkbenchTabKind,
    name: &'a str,
    workspace: &'a str,
    status_label: &'a str,
}

/// Resolves the UI identity for the selected TabInput without making that UI identity part of the
/// product selection state. A missing input still points at the bootstrap row used before the
/// authoritative catalog arrives.
pub(crate) fn tab_input_element_id(
    inputs: &[TabInput],
    selected: Option<&TabInputKey>,
) -> ElementId {
    let mut session_index = 0;
    for input in inputs {
        let id = if input.is_settings() {
            SETTINGS_WORKBENCH_TAB
        } else {
            session_tab_id(session_index)
        };
        if selected.is_some_and(|selected| input.key() == selected) {
            return id;
        }
        if input.is_session() {
            session_index += 1;
        }
    }
    crate::shell_interaction::ACTIVE_SESSION_TAB
}

impl<'a> WorkbenchTab<'a> {
    pub(crate) fn from_input(index: usize, input: &'a TabInput) -> Self {
        if input.is_settings() {
            Self::settings(SETTINGS_WORKBENCH_TAB)
        } else {
            Self::new(
                session_tab_id(index),
                input.title(),
                input.workspace(),
                input.status_label(),
            )
        }
    }

    pub(crate) const fn new(
        id: ElementId,
        name: &'a str,
        workspace: &'a str,
        status_label: &'a str,
    ) -> Self {
        Self {
            id,
            kind: WorkbenchTabKind::Session,
            name,
            workspace,
            status_label,
        }
    }

    pub(crate) const fn settings(id: ElementId) -> Self {
        Self {
            id,
            kind: WorkbenchTabKind::Settings,
            name: "Settings",
            workspace: "Application",
            status_label: "",
        }
    }
}

/// Product-owned vertical TabList for sessions and singleton workbench destinations.
pub(crate) struct WorkbenchTabList<'a> {
    bounds: Rect,
    tabs: &'a [WorkbenchTab<'a>],
    selected_id: ElementId,
    palette: ShellPalette,
    dispatch: &'a UiDispatch,
}

impl<'a> WorkbenchTabList<'a> {
    pub(crate) fn new(
        bounds: Rect,
        tabs: &'a [WorkbenchTab<'a>],
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
            let label = match tab.kind {
                WorkbenchTabKind::Session => {
                    format!("{}, {}, {}", tab.name, tab.workspace, tab.status_label)
                }
                WorkbenchTabKind::Settings => "Settings".to_owned(),
            };
            regions.push(
                InteractionRegion::new(
                    "WorkbenchTab",
                    tab.id,
                    tab_bounds,
                    AccessibilityRole::Tab,
                    label,
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

            // Every workbench item shares the same large circular background. The item-specific
            // conversation/Agent state or destination icon is painted inside this stable frame.
            scene.draw_rect(
                PaintRect::new(status_bounds, self.palette.surface)
                    .with_corner_radii(CornerRadii::uniform(STATUS_CONTAINER_SIZE * 0.5)),
            );

            if tab.kind == WorkbenchTabKind::Settings {
                let icon_size = 18.0;
                let icon_bounds = Rect::from_xywh(
                    status_bounds.origin.x + (status_bounds.size.width - icon_size) * 0.5,
                    status_bounds.origin.y + (status_bounds.size.height - icon_size) * 0.5,
                    icon_size,
                    icon_size,
                );
                scene.draw_icon(PaintIcon::new(
                    icons::GEAR,
                    icon_bounds,
                    self.palette.text_muted,
                ));
            }

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

impl Component for WorkbenchTabList<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("WorkbenchTabList")
            .in_bounds(self.bounds)
            .with_identity(SESSION_TAB_LIST)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                SESSION_TAB_LIST,
                element.bounds(),
                AccessibilityRole::TabList,
                "Workbench navigation",
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
