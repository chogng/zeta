//! Workbench tabs rendered in the body or titlebar.

use crate::{
    Color, Component, ComponentContext, ComponentElement, ComputedElement, CornerRadii, Element,
    FontWeight, InteractionRegion, PaintIcon, PaintRect, Point, Rect, Size, Tab, TabBackgrounds,
    TabList, TabListStyle, TabSelection, TabState, TabStyle, TextBlock, TextStyle, UiScene,
};
use zui::ui::AccessibilityRole;
use zui::ui::AccessibilitySelection;
use zui::ui::CursorFeedback;
use zui::ui::ElementId;
use zui::ui::FocusBehavior;
use zui::ui::NavigationGroupId;
use zui::ui::NodeAction;
use zui::ui::UiDispatch;
use zui::ui::UiNode;

use super::WorkbenchUiStyle;
use crate::TabInputKey;
use crate::TabPart;
use crate::TabStatusKind;

pub use super::tab_mount::TabContainerPlacement;
pub use super::tab_mount::WorkbenchTab;
pub use super::tab_mount::WorkbenchTabGroup;
use super::tab_mount::WorkbenchTabKind;
pub use super::tab_mount::tab_input_element_id;
pub use super::tab_mount::tab_intent_for_element;
pub use super::tab_mount::tab_key_for_element;
pub use super::tab_mount::workbench_tab_groups;

const BODY_TAB_HEIGHT: f32 = 52.0;
const BODY_TAB_GAP: f32 = 6.0;
const BODY_GROUP_LABEL_HEIGHT: f32 = 20.0;
const BODY_GROUP_GAP: f32 = 8.0;
const TITLEBAR_TAB_WIDTH: f32 = 140.0;
const TITLEBAR_TAB_HEIGHT: f32 = 24.0;
const TITLEBAR_TAB_GAP: f32 = 4.0;
const TITLEBAR_GROUP_LABEL_WIDTH: f32 = 72.0;
const TITLEBAR_GROUP_GAP: f32 = 8.0;
const TAB_CONTENT_PADDING: f32 = 8.0;
const TAB_INFORMATION_HEIGHT: f32 = 36.0;
const STATUS_CONTAINER_SIZE: f32 = TAB_INFORMATION_HEIGHT;
const STATUS_CONTENT_GAP: f32 = 10.0;
const STATUS_DOT_SIZE: f32 = 10.0;
const BODY_CLOSE_SIZE: f32 = 24.0;
const TITLEBAR_CLOSE_SIZE: f32 = 18.0;
const TITLEBAR_PIN_SIZE: f32 = 12.0;

struct GroupLayout<'a> {
    group: &'a WorkbenchTabGroup<'a>,
    list_id: ElementId,
    label_bounds: Option<Rect>,
    tab_list: TabList,
}

/// Product-owned container that projects browser-style Tab Groups at one UI mount.
pub struct TabContainer<'a> {
    bounds: Rect,
    content_bounds: Rect,
    groups: Vec<WorkbenchTabGroup<'a>>,
    selected_id: ElementId,
    placement: TabContainerPlacement,
    style: WorkbenchUiStyle,
    dispatch: &'a UiDispatch,
}

impl<'a> TabContainer<'a> {
    pub fn new(
        mut bounds: Rect,
        content_bounds: Rect,
        groups: Vec<WorkbenchTabGroup<'a>>,
        selected_id: ElementId,
        placement: TabContainerPlacement,
        style: WorkbenchUiStyle,
        dispatch: &'a UiDispatch,
    ) -> Self {
        match placement {
            TabContainerPlacement::Body => bounds = content_bounds,
            TabContainerPlacement::Titlebar => {
                let width = groups
                    .iter()
                    .enumerate()
                    .map(|(index, group)| {
                        let leading_gap = if index > 0 { TITLEBAR_GROUP_GAP } else { 0.0 };
                        let label_width = group
                            .label
                            .map_or(0.0, |_| TITLEBAR_GROUP_LABEL_WIDTH + TITLEBAR_TAB_GAP);
                        let visible_tabs = if group.collapsed { 0 } else { group.tabs.len() };
                        let tabs_width = visible_tabs as f32 * TITLEBAR_TAB_WIDTH
                            + visible_tabs.saturating_sub(1) as f32 * TITLEBAR_TAB_GAP;
                        leading_gap + label_width + tabs_width
                    })
                    .sum::<f32>()
                    .min(content_bounds.size.width);
                bounds.size.width = width;
            }
        }
        Self {
            bounds,
            content_bounds,
            groups,
            selected_id,
            placement,
            style,
            dispatch,
        }
    }

    pub fn from_tab_part(
        bounds: Rect,
        content_bounds: Rect,
        tab_part: &'a TabPart,
        selected: Option<&TabInputKey>,
        placement: TabContainerPlacement,
        style: WorkbenchUiStyle,
        dispatch: &'a UiDispatch,
    ) -> Self {
        Self::new(
            bounds,
            content_bounds,
            workbench_tab_groups(tab_part, placement, |_| true),
            tab_input_element_id(tab_part, selected, placement),
            placement,
            style,
            dispatch,
        )
    }

    fn group_layouts(&self) -> Vec<GroupLayout<'_>> {
        let mut cursor = match self.placement {
            TabContainerPlacement::Body => self.content_bounds.origin.y,
            TabContainerPlacement::Titlebar => self.content_bounds.origin.x,
        };
        self.groups
            .iter()
            .map(|group| {
                let label_bounds = group.label.map(|_| match self.placement {
                    TabContainerPlacement::Body => {
                        let bounds = Rect::from_xywh(
                            self.content_bounds.origin.x + TAB_CONTENT_PADDING,
                            cursor,
                            (self.content_bounds.size.width - TAB_CONTENT_PADDING * 2.0).max(0.0),
                            BODY_GROUP_LABEL_HEIGHT,
                        );
                        cursor = bounds.bottom();
                        bounds
                    }
                    TabContainerPlacement::Titlebar => {
                        let bounds = Rect::from_xywh(
                            cursor,
                            self.content_bounds.origin.y,
                            TITLEBAR_GROUP_LABEL_WIDTH,
                            self.content_bounds.size.height,
                        );
                        cursor = bounds.right() + TITLEBAR_TAB_GAP;
                        bounds
                    }
                });
                let visible_tabs = if group.collapsed { 0 } else { group.tabs.len() };
                let list_bounds = match self.placement {
                    TabContainerPlacement::Body => {
                        let height = visible_tabs as f32 * BODY_TAB_HEIGHT
                            + visible_tabs.saturating_sub(1) as f32 * BODY_TAB_GAP;
                        let bounds = Rect::from_xywh(
                            self.content_bounds.origin.x,
                            cursor,
                            self.content_bounds.size.width,
                            height,
                        );
                        cursor = bounds.bottom() + BODY_GROUP_GAP;
                        bounds
                    }
                    TabContainerPlacement::Titlebar => {
                        let width = visible_tabs as f32 * TITLEBAR_TAB_WIDTH
                            + visible_tabs.saturating_sub(1) as f32 * TITLEBAR_TAB_GAP;
                        let bounds = Rect::from_xywh(
                            cursor,
                            self.content_bounds.origin.y
                                + (self.content_bounds.size.height - TITLEBAR_TAB_HEIGHT) * 0.5,
                            width.min((self.content_bounds.right() - cursor).max(0.0)),
                            TITLEBAR_TAB_HEIGHT,
                        );
                        cursor = bounds.right() + TITLEBAR_GROUP_GAP;
                        bounds
                    }
                };
                GroupLayout {
                    group,
                    list_id: self.placement.group_list_id(group.id),
                    label_bounds,
                    tab_list: self.tab_list(group, list_bounds),
                }
            })
            .collect()
    }

    fn tab_list(&self, group: &WorkbenchTabGroup<'_>, bounds: Rect) -> TabList {
        let highlight = self.style.selected;
        let backgrounds = TabBackgrounds::new(Color::TRANSPARENT)
            .with_hovered(highlight)
            .with_focused(highlight)
            .with_pressed(highlight);
        let tab_style = TabStyle::new(backgrounds)
            .with_selected_backgrounds(TabBackgrounds::new(highlight))
            .with_corner_radii(CornerRadii::uniform(4.0));
        let tabs = if group.collapsed {
            Vec::new()
        } else {
            group
                .tabs
                .iter()
                .map(|tab| {
                    Tab::new(self.tab_state(tab.id)).with_selection(if tab.id == self.selected_id {
                        TabSelection::Selected
                    } else {
                        TabSelection::Unselected
                    })
                })
                .collect()
        };
        let (size, gap) = match self.placement {
            TabContainerPlacement::Body => {
                (Size::new(bounds.size.width, BODY_TAB_HEIGHT), BODY_TAB_GAP)
            }
            TabContainerPlacement::Titlebar => (
                Size::new(TITLEBAR_TAB_WIDTH, TITLEBAR_TAB_HEIGHT),
                TITLEBAR_TAB_GAP,
            ),
        };
        TabList::new(
            bounds,
            self.placement.orientation(),
            tabs,
            TabListStyle::new(tab_style, size).with_gap(gap),
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

    fn compose_groups(&self, context: &mut ComponentContext<'_, '_>) {
        for layout in self.group_layouts() {
            let list_bounds = layout.tab_list.bounds().intersection(self.content_bounds);
            if list_bounds.is_empty() {
                continue;
            }
            context.draw_component(&InteractionRegion::new(
                "WorkbenchTabGroup",
                layout.list_id,
                list_bounds,
                AccessibilityRole::TabList,
                layout.group.label.unwrap_or("Workbench tabs"),
            ));
            if !layout.group.collapsed {
                for (index, tab) in layout.group.tabs.iter().enumerate() {
                    let tab_bounds = layout
                        .tab_list
                        .tab_bounds(index)
                        .expect("registered tab")
                        .intersection(self.content_bounds);
                    if tab_bounds.is_empty() {
                        continue;
                    }
                    context.draw_component(&self.tab_region(tab, tab_bounds, layout.list_id));
                    context.draw_component(&self.close_region(tab, tab_bounds));
                }
            }
            context.scene_mut().with_clip(self.content_bounds, |scene| {
                self.paint_group(scene, &layout)
            });
        }
    }

    fn tab_region(
        &self,
        tab: &WorkbenchTab<'_>,
        bounds: Rect,
        list_id: ElementId,
    ) -> InteractionRegion {
        let label = match tab.kind {
            WorkbenchTabKind::Session => {
                format!("{}, {}, {}", tab.name, tab.workspace, tab.status.label())
            }
            WorkbenchTabKind::Settings => "Settings".to_owned(),
        };
        InteractionRegion::new(
            "WorkbenchTab",
            tab.id,
            bounds,
            AccessibilityRole::Tab,
            label,
        )
        .with_parent(list_id)
        .with_cursor(CursorFeedback::Pointer)
        .with_focus(FocusBehavior::TabStop)
        .with_action(NodeAction::Activate)
        .with_navigation(
            NavigationGroupId::new(list_id),
            self.placement.navigation_axis(),
        )
        .with_selection(if tab.id == self.selected_id {
            AccessibilitySelection::Selected
        } else {
            AccessibilitySelection::Unselected
        })
    }

    fn close_region(&self, tab: &WorkbenchTab<'_>, tab_bounds: Rect) -> InteractionRegion {
        InteractionRegion::new(
            "WorkbenchTabClose",
            tab.close_id,
            self.close_bounds(tab_bounds),
            AccessibilityRole::Button,
            format!("Close {}", tab.name),
        )
        .with_parent(tab.id)
        .with_cursor(CursorFeedback::Pointer)
        .with_focus(FocusBehavior::TabStop)
        .with_action(NodeAction::Activate)
    }

    fn close_bounds(&self, tab_bounds: Rect) -> Rect {
        let size = match self.placement {
            TabContainerPlacement::Body => BODY_CLOSE_SIZE,
            TabContainerPlacement::Titlebar => TITLEBAR_CLOSE_SIZE,
        };
        Rect::from_xywh(
            tab_bounds.right() - TAB_CONTENT_PADDING - size,
            tab_bounds.origin.y + (tab_bounds.size.height - size) * 0.5,
            size,
            size,
        )
    }

    fn paint_group(&self, scene: &mut UiScene, layout: &GroupLayout<'_>) {
        if let (Some(label), Some(bounds)) = (layout.group.label, layout.label_bounds) {
            scene.draw_text(TextBlock::new(
                label,
                Point::new(bounds.origin.x, bounds.origin.y + 2.0),
                bounds.size,
                TextStyle::new(11.0, self.style.text_muted)
                    .with_weight(FontWeight::Bold)
                    .with_line_height(16.0),
            ));
        }
        scene.draw_component(&layout.tab_list);
        if !layout.group.collapsed {
            self.paint_tabs(scene, layout);
        }
    }

    fn paint_tabs(&self, scene: &mut UiScene, layout: &GroupLayout<'_>) {
        for (index, tab) in layout.group.tabs.iter().enumerate() {
            let tab_bounds = layout.tab_list.tab_bounds(index).expect("painted tab");
            match self.placement {
                TabContainerPlacement::Body => self.paint_body_tab(scene, tab, tab_bounds),
                TabContainerPlacement::Titlebar => self.paint_titlebar_tab(scene, tab, tab_bounds),
            }
        }
    }

    fn paint_body_tab(&self, scene: &mut UiScene, tab: &WorkbenchTab<'_>, tab_bounds: Rect) {
        let status_bounds = Rect::from_xywh(
            tab_bounds.origin.x + TAB_CONTENT_PADDING,
            tab_bounds.origin.y + (tab_bounds.size.height - STATUS_CONTAINER_SIZE) * 0.5,
            STATUS_CONTAINER_SIZE,
            STATUS_CONTAINER_SIZE,
        );
        scene.draw_rect(
            PaintRect::new(status_bounds, self.style.surface)
                .with_corner_radii(CornerRadii::uniform(STATUS_CONTAINER_SIZE * 0.5)),
        );
        if tab.kind == WorkbenchTabKind::Settings {
            self.paint_settings_icon(scene, status_bounds, 18.0);
        } else if tab.pinned {
            self.paint_pinned_status(scene, tab, status_bounds);
        } else {
            self.paint_status_dot(scene, tab, status_bounds);
        }
        let text_x = status_bounds.right() + STATUS_CONTENT_GAP;
        let text_right = self.close_bounds(tab_bounds).origin.x - 6.0;
        let text_width = (text_right - text_x).max(1.0);
        scene.draw_text(TextBlock::new(
            tab.name,
            Point::new(text_x, tab_bounds.origin.y + 7.0),
            Size::new(text_width, 18.0),
            TextStyle::new(13.0, self.style.text).with_weight(FontWeight::Bold),
        ));
        scene.draw_text(TextBlock::new(
            tab.workspace,
            Point::new(text_x, tab_bounds.origin.y + 27.0),
            Size::new(text_width, 15.0),
            TextStyle::new(11.0, self.style.text_muted).with_line_height(15.0),
        ));
        self.paint_close_icon(scene, tab_bounds);
    }

    fn paint_titlebar_tab(&self, scene: &mut UiScene, tab: &WorkbenchTab<'_>, tab_bounds: Rect) {
        let mut text_x = tab_bounds.origin.x + TAB_CONTENT_PADDING;
        if tab.kind == WorkbenchTabKind::Settings {
            let icon_bounds = Rect::from_xywh(
                text_x,
                tab_bounds.origin.y + (tab_bounds.size.height - 16.0) * 0.5,
                16.0,
                16.0,
            );
            self.paint_settings_icon(scene, icon_bounds, 16.0);
            text_x = icon_bounds.right() + 6.0;
        } else {
            let status_bounds = Rect::from_xywh(
                text_x,
                tab_bounds.origin.y + (tab_bounds.size.height - STATUS_DOT_SIZE) * 0.5,
                STATUS_DOT_SIZE,
                STATUS_DOT_SIZE,
            );
            self.paint_status_dot(scene, tab, status_bounds);
            text_x = status_bounds.right() + 6.0;
            if tab.pinned {
                let pin_bounds = Rect::from_xywh(
                    text_x,
                    tab_bounds.origin.y + (tab_bounds.size.height - TITLEBAR_PIN_SIZE) * 0.5,
                    TITLEBAR_PIN_SIZE,
                    TITLEBAR_PIN_SIZE,
                );
                scene.draw_icon(PaintIcon::new(
                    self.style.pinned_icon,
                    pin_bounds,
                    self.style.text_muted,
                ));
                text_x = pin_bounds.right() + 4.0;
            }
        }
        let text_right = self.close_bounds(tab_bounds).origin.x - 4.0;
        scene.draw_text(TextBlock::new(
            tab.name,
            Point::new(text_x, tab_bounds.origin.y + 3.0),
            Size::new((text_right - text_x).max(1.0), 18.0),
            TextStyle::new(12.0, self.style.text).with_line_height(18.0),
        ));
        self.paint_close_icon(scene, tab_bounds);
    }

    fn paint_pinned_status(&self, scene: &mut UiScene, tab: &WorkbenchTab<'_>, bounds: Rect) {
        let icon_size = 16.0;
        let icon_bounds = Rect::from_xywh(
            bounds.origin.x + (bounds.size.width - icon_size) * 0.5,
            bounds.origin.y + (bounds.size.height - icon_size) * 0.5,
            icon_size,
            icon_size,
        );
        scene.draw_icon(PaintIcon::new(
            self.style.pinned_icon,
            icon_bounds,
            self.style.text_muted,
        ));
        let dot_bounds = Rect::from_xywh(
            bounds.right() - STATUS_DOT_SIZE,
            bounds.bottom() - STATUS_DOT_SIZE,
            STATUS_DOT_SIZE,
            STATUS_DOT_SIZE,
        );
        self.paint_status_dot(scene, tab, dot_bounds);
    }

    fn paint_status_dot(&self, scene: &mut UiScene, tab: &WorkbenchTab<'_>, bounds: Rect) {
        let dot_bounds = Rect::from_xywh(
            bounds.origin.x + (bounds.size.width - STATUS_DOT_SIZE) * 0.5,
            bounds.origin.y + (bounds.size.height - STATUS_DOT_SIZE) * 0.5,
            STATUS_DOT_SIZE,
            STATUS_DOT_SIZE,
        );
        scene.draw_rect(
            PaintRect::new(dot_bounds, self.status_color(tab.status.kind()))
                .with_corner_radii(CornerRadii::uniform(STATUS_DOT_SIZE * 0.5)),
        );
    }

    const fn status_color(&self, kind: TabStatusKind) -> Color {
        match kind {
            TabStatusKind::Idle => self.style.text_muted,
            TabStatusKind::Busy => self.style.accent,
            TabStatusKind::Attention | TabStatusKind::Warning => self.style.warning,
            TabStatusKind::Success => self.style.success,
            TabStatusKind::Error => self.style.error,
        }
    }

    fn paint_close_icon(&self, scene: &mut UiScene, tab_bounds: Rect) {
        let bounds = self.close_bounds(tab_bounds);
        let size = match self.placement {
            TabContainerPlacement::Body => 16.0,
            TabContainerPlacement::Titlebar => 14.0,
        };
        let icon_bounds = Rect::from_xywh(
            bounds.origin.x + (bounds.size.width - size) * 0.5,
            bounds.origin.y + (bounds.size.height - size) * 0.5,
            size,
            size,
        );
        scene.draw_icon(PaintIcon::new(
            self.style.close_icon,
            icon_bounds,
            self.style.text_muted,
        ));
    }

    fn paint_settings_icon(&self, scene: &mut UiScene, bounds: Rect, icon_size: f32) {
        let icon_bounds = Rect::from_xywh(
            bounds.origin.x + (bounds.size.width - icon_size) * 0.5,
            bounds.origin.y + (bounds.size.height - icon_size) * 0.5,
            icon_size,
            icon_size,
        );
        scene.draw_icon(PaintIcon::new(
            self.style.settings_icon,
            icon_bounds,
            self.style.text_muted,
        ));
    }
}

impl Component for TabContainer<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("TabContainer")
            .in_bounds(self.bounds)
            .with_identity(self.placement.container_id())
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                self.placement.container_id(),
                element.bounds(),
                AccessibilityRole::Group,
                "Workbench tabs",
            )
            .with_parent(self.placement.parent_id()),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        self.compose_groups(context);
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.with_clip(self.content_bounds, |scene| {
            for layout in self.group_layouts() {
                self.paint_group(scene, &layout);
            }
        });
    }
}

#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tests;
