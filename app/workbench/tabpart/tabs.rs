//! Workbench tabs rendered in the body or titlebar.

use crate::{
    ActionBar, ActionBarItem, ActionBarOrientation, ActionBarStyle, ActionViewItem,
    ButtonBackgrounds, ButtonState, ButtonStyle, Color, Component, ComponentContext,
    ComponentElement, ComputedElement, CornerRadii, Edges, Element, FontWeight, InteractionRegion,
    PaintIcon, PaintRect, Point, Rect, Size, Tab, TabBackgrounds, TabList, TabListStyle,
    TabSelection, TabState, TabStyle, TextBlock, TextStyle, UiScene,
};
use zeta_icons::icons;
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
pub use super::tab_mount::mounted_tab_element_id;
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
const TAB_ACTION_GAP: f32 = 2.0;

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
    visible_action_bar_tab: Option<ElementId>,
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
            visible_action_bar_tab: None,
            placement,
            style,
            dispatch,
        }
    }

    /// Keeps one mounted tab's action bar visible independently of pointer and focus state.
    pub fn with_visible_action_bar(mut self, tab: ElementId) -> Self {
        self.visible_action_bar_tab = Some(tab);
        self
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
        let backgrounds = TabBackgrounds::new(Color::TRANSPARENT)
            .with_hovered(self.style.colors.tab_hover_background)
            .with_focused(self.style.colors.tab_hover_background)
            .with_pressed(self.style.colors.tab_hover_background);
        let tab_style = TabStyle::new(backgrounds)
            .with_selected_backgrounds(TabBackgrounds::new(self.style.colors.tab_active_background))
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
                    if self.tab_action_bar_visible(tab) {
                        for region in self.action_regions(tab, tab_bounds) {
                            context.draw_component(&region);
                        }
                    }
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

    fn action_regions(&self, tab: &WorkbenchTab<'_>, tab_bounds: Rect) -> [InteractionRegion; 2] {
        let action_bar = self.tab_action_bar(tab, tab_bounds);
        [
            InteractionRegion::new(
                "WorkbenchTabActionsButton",
                tab.action_id,
                action_bar
                    .interactive_item_bounds(0)
                    .expect("tab actions button is enabled"),
                AccessibilityRole::Button,
                format!("Actions for {}", tab.name),
            )
            .with_parent(tab.id)
            .with_cursor(CursorFeedback::Pointer)
            .with_focus(FocusBehavior::TabStop)
            .with_action(NodeAction::Activate),
            InteractionRegion::new(
                "WorkbenchTabClose",
                tab.close_id,
                action_bar
                    .interactive_item_bounds(1)
                    .expect("tab close button is enabled"),
                AccessibilityRole::Button,
                format!("Close {}", tab.name),
            )
            .with_parent(tab.id)
            .with_cursor(CursorFeedback::Pointer)
            .with_focus(FocusBehavior::TabStop)
            .with_action(NodeAction::Activate),
        ]
    }

    fn tab_action_bar(&self, tab: &WorkbenchTab<'_>, tab_bounds: Rect) -> ActionBar {
        let size = match self.placement {
            TabContainerPlacement::Body => BODY_CLOSE_SIZE,
            TabContainerPlacement::Titlebar => TITLEBAR_CLOSE_SIZE,
        };
        let width = size * 2.0 + TAB_ACTION_GAP;
        let bounds = Rect::from_xywh(
            tab_bounds.right() - TAB_CONTENT_PADDING - width,
            tab_bounds.origin.y + (tab_bounds.size.height - size) * 0.5,
            width,
            size,
        );
        let button_style = ButtonStyle::new(
            ButtonBackgrounds::new(Color::TRANSPARENT)
                .with_hovered(self.style.colors.control_hover_background)
                .with_focused(Color::TRANSPARENT)
                .with_pressed(self.style.colors.border),
            TextStyle::new(12.0, self.style.colors.muted_foreground),
        )
        .with_corner_radii(CornerRadii::uniform(4.0))
        .with_padding(Edges::uniform(3.0))
        .with_icon_size(self.action_icon_size());
        ActionBar::new(
            bounds,
            ActionBarOrientation::Horizontal,
            vec![
                ActionBarItem::Action(ActionViewItem::icon(
                    icons::ELLIPSIS,
                    format!("Actions for {}", tab.name),
                    self.button_state(tab.action_id),
                )),
                ActionBarItem::Action(ActionViewItem::icon(
                    self.style.close_icon,
                    format!("Close {}", tab.name),
                    self.button_state(tab.close_id),
                )),
            ],
            ActionBarStyle::new(button_style, Size::new(size, size)).with_gap(TAB_ACTION_GAP),
        )
    }

    fn tab_action_bar_visible(&self, tab: &WorkbenchTab<'_>) -> bool {
        self.visible_action_bar_tab == Some(tab.id)
            || self.dispatch.is_hovered(tab.id)
            || self.dispatch.is_focused(tab.id)
            || self.dispatch.is_hovered(tab.action_id)
            || self.dispatch.is_hovered(tab.close_id)
            || self.dispatch.is_focused(tab.action_id)
            || self.dispatch.is_focused(tab.close_id)
            || self.dispatch.is_pressed(tab.action_id)
            || self.dispatch.is_pressed(tab.close_id)
    }

    fn paint_tab_action_bar(&self, scene: &mut UiScene, tab: &WorkbenchTab<'_>, tab_bounds: Rect) {
        let action_bar = self.tab_action_bar(tab, tab_bounds);
        scene.draw_rect(
            PaintRect::new(action_bar.bounds(), self.style.colors.action_bar_background)
                .with_corner_radii(CornerRadii::uniform(4.0)),
        );
        scene.draw_component(&action_bar);
    }

    fn action_icon_size(&self) -> f32 {
        match self.placement {
            TabContainerPlacement::Body => 16.0,
            TabContainerPlacement::Titlebar => 12.0,
        }
    }

    fn pinned_action_icon_bounds(&self, tab: &WorkbenchTab<'_>, tab_bounds: Rect) -> Rect {
        let close_bounds = self
            .tab_action_bar(tab, tab_bounds)
            .item_bounds(1)
            .expect("tab close slot");
        let icon_size = self.action_icon_size();
        Rect::from_xywh(
            close_bounds.origin.x + (close_bounds.size.width - icon_size) * 0.5,
            close_bounds.origin.y + (close_bounds.size.height - icon_size) * 0.5,
            icon_size,
            icon_size,
        )
    }

    fn paint_pinned_action_status(
        &self,
        scene: &mut UiScene,
        tab: &WorkbenchTab<'_>,
        tab_bounds: Rect,
    ) {
        scene.draw_icon(PaintIcon::new(
            self.style.pinned_icon,
            self.pinned_action_icon_bounds(tab, tab_bounds),
            self.style.colors.muted_foreground,
        ));
    }

    fn button_state(&self, id: ElementId) -> ButtonState {
        if self.dispatch.is_pressed(id) {
            ButtonState::Pressed
        } else if self.dispatch.is_focused(id) {
            ButtonState::Focused
        } else if self.dispatch.is_hovered(id) {
            ButtonState::Hovered
        } else {
            ButtonState::Resting
        }
    }

    fn paint_group(&self, scene: &mut UiScene, layout: &GroupLayout<'_>) {
        if let (Some(label), Some(bounds)) = (layout.group.label, layout.label_bounds) {
            scene.draw_text(TextBlock::new(
                label,
                Point::new(bounds.origin.x, bounds.origin.y + 2.0),
                bounds.size,
                TextStyle::new(11.0, self.style.colors.muted_foreground)
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
        let action_bar_visible = self.tab_action_bar_visible(tab);
        let status_bounds = Rect::from_xywh(
            tab_bounds.origin.x + TAB_CONTENT_PADDING,
            tab_bounds.origin.y + (tab_bounds.size.height - STATUS_CONTAINER_SIZE) * 0.5,
            STATUS_CONTAINER_SIZE,
            STATUS_CONTAINER_SIZE,
        );
        scene.draw_rect(
            PaintRect::new(status_bounds, self.style.colors.content_background)
                .with_corner_radii(CornerRadii::uniform(STATUS_CONTAINER_SIZE * 0.5)),
        );
        if tab.kind == WorkbenchTabKind::Settings {
            self.paint_settings_icon(scene, status_bounds, 18.0);
        } else {
            self.paint_status_dot(scene, tab, status_bounds);
        }
        let text_x = status_bounds.right() + STATUS_CONTENT_GAP;
        let text_right = if action_bar_visible {
            self.tab_action_bar(tab, tab_bounds).bounds().origin.x - 6.0
        } else if tab.pinned {
            self.pinned_action_icon_bounds(tab, tab_bounds).origin.x - 6.0
        } else {
            tab_bounds.right() - TAB_CONTENT_PADDING
        };
        let text_width = (text_right - text_x).max(1.0);
        scene.draw_text(TextBlock::new(
            tab.name,
            Point::new(text_x, tab_bounds.origin.y + 7.0),
            Size::new(text_width, 18.0),
            TextStyle::new(13.0, self.style.colors.foreground).with_weight(FontWeight::Bold),
        ));
        scene.draw_text(TextBlock::new(
            tab.workspace,
            Point::new(text_x, tab_bounds.origin.y + 27.0),
            Size::new(text_width, 15.0),
            TextStyle::new(11.0, self.style.colors.foreground).with_line_height(15.0),
        ));
        if action_bar_visible {
            self.paint_tab_action_bar(scene, tab, tab_bounds);
        } else if tab.pinned {
            self.paint_pinned_action_status(scene, tab, tab_bounds);
        }
    }

    fn paint_titlebar_tab(&self, scene: &mut UiScene, tab: &WorkbenchTab<'_>, tab_bounds: Rect) {
        let action_bar_visible = self.tab_action_bar_visible(tab);
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
        }
        let text_right = if action_bar_visible {
            self.tab_action_bar(tab, tab_bounds).bounds().origin.x - 4.0
        } else if tab.pinned {
            self.pinned_action_icon_bounds(tab, tab_bounds).origin.x - 4.0
        } else {
            tab_bounds.right() - TAB_CONTENT_PADDING
        };
        scene.draw_text(TextBlock::new(
            tab.name,
            Point::new(text_x, tab_bounds.origin.y + 3.0),
            Size::new((text_right - text_x).max(1.0), 18.0),
            TextStyle::new(12.0, self.style.colors.foreground).with_line_height(18.0),
        ));
        if action_bar_visible {
            self.paint_tab_action_bar(scene, tab, tab_bounds);
        } else if tab.pinned {
            self.paint_pinned_action_status(scene, tab, tab_bounds);
        }
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
            TabStatusKind::Idle => self.style.colors.muted_foreground,
            TabStatusKind::Busy => self.style.colors.accent,
            TabStatusKind::Attention | TabStatusKind::Warning => self.style.colors.warning,
            TabStatusKind::Success => self.style.colors.success,
            TabStatusKind::Error => self.style.colors.error,
        }
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
            self.style.colors.muted_foreground,
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
