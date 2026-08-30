//! Workbench Tab Container header, scrollable content, groups, and tabs.

use crate::{
    ActionBar, ActionBarItem, ActionBarOrientation, ActionBarStyle, ActionViewItem, Border,
    ButtonBackgrounds, ButtonState, ButtonStyle, CaretVisibility, Color, Component,
    ComponentContext, ComponentElement, ComputedElement, CornerRadii, Edges, Element, FontWeight,
    InteractionRegion, PaintIcon, PaintRect, Point, Rect, ScrollAxis, ScrollMetrics, ScrollState,
    ScrollView, ScrollbarPresentation, Size, Tab, TabBackgrounds, TabList, TabListOrientation,
    TabListStyle, TabSelection, TabState, TabStyle, TextBlock, TextInput, TextInputLayoutEngine,
    TextStyle, UiScene,
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
use super::tab_dirs_preview::TabDirsPreview;
#[cfg(test)]
use super::tab_dirs_preview::{
    dirs_preview_disclosure_id, dirs_preview_scroll_forward_id, dirs_preview_scroll_id,
};
use super::toolbar::TOOLBAR_HEIGHT;
use super::toolbar::TabContainerToolbar;
#[cfg(test)]
use crate::TabInputKey;
#[cfg(test)]
use crate::TabPart;
use crate::TabStatusKind;

use super::tab_mount::TabGroup;
use super::tab_mount::WorkbenchTab;
use super::tab_mount::WorkbenchTabKind;
pub use super::tab_mount::mounted_tab_element_id;
pub use super::tab_mount::tab_input_element_id;
pub use super::tab_mount::tab_intent_for_element;
pub use super::tab_mount::tab_key_for_element;
pub use super::tab_mount::workbench_tab_groups;

const PART_PADDING: f32 = 10.0;
const HEADER_CONTENT_GAP: f32 = 4.0;
const TAB_HEIGHT: f32 = 52.0;
const TAB_GAP: f32 = 6.0;
const GROUP_LABEL_HEIGHT: f32 = 20.0;
const GROUP_GAP: f32 = 8.0;
const TAB_CONTENT_PADDING: f32 = 8.0;
const TAB_INFORMATION_HEIGHT: f32 = 36.0;
const STATUS_CONTAINER_SIZE: f32 = TAB_INFORMATION_HEIGHT;
const STATUS_CONTENT_GAP: f32 = 10.0;
const STATUS_DOT_SIZE: f32 = 10.0;
const CLOSE_SIZE: f32 = 24.0;
const TAB_ACTION_GAP: f32 = 2.0;

struct TabContainerLayout {
    header: Rect,
    toolbar: Rect,
    content: Rect,
    list: Rect,
}

impl TabContainerLayout {
    fn for_bounds(bounds: Rect) -> Self {
        let header_height = PART_PADDING + TOOLBAR_HEIGHT + HEADER_CONTENT_GAP;
        let header = Rect::from_xywh(
            bounds.origin.x,
            bounds.origin.y,
            bounds.size.width,
            header_height.min(bounds.size.height),
        );
        let toolbar = Rect::from_xywh(
            bounds.origin.x,
            bounds.origin.y + PART_PADDING,
            bounds.size.width,
            TOOLBAR_HEIGHT.min((bounds.bottom() - bounds.origin.y - PART_PADDING).max(1.0)),
        );
        let content = Rect::from_xywh(
            bounds.origin.x,
            header.bottom(),
            bounds.size.width,
            (bounds.bottom() - header.bottom()).max(1.0),
        );
        let list = Rect::from_xywh(
            content.origin.x + PART_PADDING,
            content.origin.y,
            (content.size.width - PART_PADDING * 2.0).max(1.0),
            (content.size.height - PART_PADDING).max(1.0),
        );
        Self {
            header,
            toolbar,
            content,
            list,
        }
    }
}

struct GroupLayout {
    group_index: usize,
    list_id: ElementId,
    bounds: Rect,
    label_bounds: Option<Rect>,
    tab_list: TabList,
}

/// Application-owned container that projects browser-style Tab Groups at one UI mount.
pub struct TabContainer<'a> {
    bounds: Rect,
    viewport: Rect,
    layout: TabContainerLayout,
    toolbar: TabContainerToolbar,
    groups: Vec<TabGroup<'a>>,
    selected_id: ElementId,
    visible_action_bar_tab: Option<ElementId>,
    scroll: ScrollState,
    scrollbar_presentation: ScrollbarPresentation,
    style: WorkbenchUiStyle,
    dispatch: &'a UiDispatch,
}

impl<'a> TabContainer<'a> {
    pub fn new(
        bounds: Rect,
        search_input: &TextInput,
        caret_visibility: CaretVisibility,
        groups: Vec<TabGroup<'a>>,
        selected_id: ElementId,
        style: WorkbenchUiStyle,
        text_layout: &mut TextInputLayoutEngine,
        dispatch: &'a UiDispatch,
    ) -> Self {
        let layout = TabContainerLayout::for_bounds(bounds);
        let toolbar = TabContainerToolbar::new(
            layout.toolbar,
            search_input,
            caret_visibility,
            style.clone(),
            text_layout,
            dispatch,
        );
        Self {
            bounds,
            viewport: bounds,
            layout,
            toolbar,
            groups,
            selected_id,
            visible_action_bar_tab: None,
            scroll: ScrollState::default(),
            scrollbar_presentation: ScrollbarPresentation::default(),
            style,
            dispatch,
        }
    }

    /// Supplies the window viewport used to place overlays outside the Tab Container bounds.
    pub fn with_viewport(mut self, viewport: Rect) -> Self {
        self.viewport = viewport;
        self
    }

    /// Keeps one mounted tab's action bar visible independently of pointer and focus state.
    pub fn with_visible_action_bar(mut self, tab: ElementId) -> Self {
        self.visible_action_bar_tab = Some(tab);
        self
    }

    /// Supplies retained vertical scroll state for the body-mounted tab list.
    pub const fn with_scroll_state(mut self, scroll: ScrollState) -> Self {
        self.scroll = scroll;
        self
    }

    /// Supplies the retained scrollbar visibility and interaction state.
    pub const fn with_scrollbar_presentation(
        mut self,
        presentation: ScrollbarPresentation,
    ) -> Self {
        self.scrollbar_presentation = presentation;
        self
    }

    /// Returns the body-mounted list viewport and content extents.
    pub fn scroll_metrics(&self) -> ScrollMetrics {
        self.scroll_view().metrics()
    }

    #[cfg(test)]
    pub fn from_tab_part(
        bounds: Rect,
        tab_part: &'a TabPart,
        selected: Option<&TabInputKey>,
        style: WorkbenchUiStyle,
        dispatch: &'a UiDispatch,
    ) -> Self {
        let mut text_layout = TextInputLayoutEngine::new();
        Self::new(
            bounds,
            &TextInput::new(),
            CaretVisibility::Visible,
            workbench_tab_groups(tab_part, |_| true),
            tab_input_element_id(tab_part, selected),
            style,
            &mut text_layout,
            dispatch,
        )
    }

    pub const fn search_caret_bounds(&self) -> Option<Rect> {
        self.toolbar.search_caret_bounds()
    }

    fn group_layouts(&self) -> Vec<GroupLayout> {
        let mut cursor = self.scroll_view().viewport().content_origin().y;
        self.groups
            .iter()
            .enumerate()
            .map(|(group_index, group)| {
                let group_top = cursor;
                let label_bounds = group.label.map(|_| {
                    let bounds = Rect::from_xywh(
                        self.layout.list.origin.x + TAB_CONTENT_PADDING,
                        cursor,
                        (self.layout.list.size.width - TAB_CONTENT_PADDING * 2.0).max(0.0),
                        GROUP_LABEL_HEIGHT,
                    );
                    cursor = bounds.bottom();
                    bounds
                });
                let visible_tabs = if group.collapsed { 0 } else { group.tabs.len() };
                let height = visible_tabs as f32 * TAB_HEIGHT
                    + visible_tabs.saturating_sub(1) as f32 * TAB_GAP;
                let list_bounds = Rect::from_xywh(
                    self.layout.list.origin.x,
                    cursor,
                    self.layout.list.size.width,
                    height,
                );
                cursor = list_bounds.bottom();
                let group_bottom = cursor.max(label_bounds.map_or(group_top, Rect::bottom));
                cursor += GROUP_GAP;
                GroupLayout {
                    group_index,
                    list_id: super::identity::tab_group_list_id(group.id),
                    bounds: Rect::from_xywh(
                        self.layout.list.origin.x,
                        group_top,
                        self.layout.list.size.width,
                        (group_bottom - group_top).max(0.0),
                    ),
                    label_bounds,
                    tab_list: self.tab_list(group, list_bounds),
                }
            })
            .collect()
    }

    fn content_height(&self) -> f32 {
        self.groups
            .iter()
            .map(|group| {
                let label_height = group.label.map_or(0.0, |_| GROUP_LABEL_HEIGHT);
                let visible_tabs = if group.collapsed { 0 } else { group.tabs.len() };
                let tabs_height = visible_tabs as f32 * TAB_HEIGHT
                    + visible_tabs.saturating_sub(1) as f32 * TAB_GAP;
                label_height + tabs_height + GROUP_GAP
            })
            .sum::<f32>()
            + PART_PADDING
    }

    pub(crate) fn scroll_view(&self) -> ScrollView {
        ScrollView::new(
            self.layout.content,
            Size::new(self.layout.content.size.width, self.content_height()),
            self.scroll,
            ScrollAxis::Vertical,
            self.style.scroll_view,
        )
        .with_scrollbar_presentation(self.scrollbar_presentation)
    }

    fn tab_list(&self, group: &TabGroup<'_>, bounds: Rect) -> TabList {
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
        TabList::new(
            bounds,
            TabListOrientation::Vertical,
            tabs,
            TabListStyle::new(tab_style, Size::new(bounds.size.width, TAB_HEIGHT))
                .with_gap(TAB_GAP),
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

    fn compose_visible_groups(&self, context: &mut ComponentContext<'_, '_>) {
        for layout in self.group_layouts() {
            if layout.bounds.intersection(self.layout.list).is_empty() {
                continue;
            }
            context.draw_component(&TabGroupView {
                container: self,
                layout,
            });
        }
    }

    fn compose_dirs_preview(&self, context: &mut ComponentContext<'_, '_>) {
        let hovered = self.group_layouts().into_iter().find_map(|layout| {
            self.groups[layout.group_index]
                .tabs
                .iter()
                .enumerate()
                .find_map(|(index, tab)| {
                    (tab.kind == WorkbenchTabKind::Session && self.dispatch.is_hovered(tab.id))
                        .then(|| {
                            (
                                tab,
                                layout
                                    .tab_list
                                    .tab_bounds(index)
                                    .expect("hovered tab has layout bounds"),
                            )
                        })
                })
        });
        let Some((tab, tab_bounds)) = hovered else {
            return;
        };
        context.draw_component(&TabDirsPreview::new(
            self.viewport,
            tab_bounds,
            tab,
            self.style.clone(),
            self.dispatch,
        ));
    }

    fn tab_region(
        &self,
        tab: &WorkbenchTab<'_>,
        bounds: Rect,
        list_id: ElementId,
    ) -> InteractionRegion {
        let label = match tab.kind {
            WorkbenchTabKind::Session => {
                format!("{}, {}, {}", tab.name, tab.location, tab.status.label())
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
            zui::ui::NavigationAxis::Vertical,
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
        let size = CLOSE_SIZE;
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
        16.0
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

    fn paint_group_label(&self, scene: &mut UiScene, layout: &GroupLayout) {
        let group = &self.groups[layout.group_index];
        if let (Some(label), Some(bounds)) = (group.label, layout.label_bounds) {
            scene.draw_text(TextBlock::new(
                label,
                Point::new(bounds.origin.x, bounds.origin.y + 2.0),
                bounds.size,
                TextStyle::new(11.0, self.style.colors.muted_foreground)
                    .with_weight(FontWeight::Bold)
                    .with_line_height(16.0),
            ));
        }
    }

    fn paint_tabs(&self, scene: &mut UiScene, layout: &GroupLayout) {
        for (index, tab) in self.groups[layout.group_index].tabs.iter().enumerate() {
            let tab_bounds = layout.tab_list.tab_bounds(index).expect("painted tab");
            self.paint_tab(scene, tab, tab_bounds);
        }
    }

    fn paint_tab(&self, scene: &mut UiScene, tab: &WorkbenchTab<'_>, tab_bounds: Rect) {
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
            tab.location,
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
            TabStatusKind::Warning => self.style.colors.warning,
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

struct TabContainerHeader<'a> {
    bounds: Rect,
    toolbar: &'a TabContainerToolbar,
}

impl Component for TabContainerHeader<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("TabContainerHeader").in_bounds(self.bounds)
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        context.draw_component(self.toolbar);
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_component(self.toolbar);
    }
}

struct TabContainerContent<'container, 'tabs> {
    container: &'container TabContainer<'tabs>,
}

impl Component for TabContainerContent<'_, '_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("TabContainerContent").in_bounds(self.container.layout.content)
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        self.container
            .scroll_view()
            .draw_components(context, |context, _viewport| {
                self.container.compose_visible_groups(context)
            });
        self.container.compose_dirs_preview(context);
    }

    fn paint(&self, scene: &mut UiScene) {
        self.container
            .scroll_view()
            .draw(scene, |scene, _viewport| {
                for layout in self.container.group_layouts() {
                    scene.draw_component(&TabGroupView {
                        container: self.container,
                        layout,
                    });
                }
            });
    }
}

struct TabGroupView<'container, 'tabs> {
    container: &'container TabContainer<'tabs>,
    layout: GroupLayout,
}

impl Component for TabGroupView<'_, '_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("TabGroup")
            .in_bounds(self.layout.bounds.intersection(self.container.layout.list))
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        self.container
            .paint_group_label(context.scene_mut(), &self.layout);
        context.draw_component(&WorkbenchTabListView {
            container: self.container,
            layout: &self.layout,
        });
    }

    fn paint(&self, scene: &mut UiScene) {
        self.container.paint_group_label(scene, &self.layout);
        scene.draw_component(&WorkbenchTabListView {
            container: self.container,
            layout: &self.layout,
        });
    }
}

struct WorkbenchTabListView<'container, 'layout, 'tabs> {
    container: &'container TabContainer<'tabs>,
    layout: &'layout GroupLayout,
}

impl Component for WorkbenchTabListView<'_, '_, '_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("TabList")
            .gap(TAB_GAP)
            .in_bounds(
                self.layout
                    .tab_list
                    .bounds()
                    .intersection(self.container.layout.list),
            )
            .with_identity(self.layout.list_id)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        let group = &self.container.groups[self.layout.group_index];
        Some(UiNode::new(
            self.layout.list_id,
            element.bounds(),
            AccessibilityRole::TabList,
            group.label.unwrap_or("Workbench tabs"),
        ))
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        self.layout.tab_list.paint(context.scene_mut());
        let group = &self.container.groups[self.layout.group_index];
        if !group.collapsed {
            for (index, tab) in group.tabs.iter().enumerate() {
                let tab_bounds = self
                    .layout
                    .tab_list
                    .tab_bounds(index)
                    .expect("registered tab")
                    .intersection(self.container.layout.list);
                if tab_bounds.is_empty() {
                    continue;
                }
                context.draw_component(&self.container.tab_region(
                    tab,
                    tab_bounds,
                    self.layout.list_id,
                ));
                if self.container.tab_action_bar_visible(tab) {
                    for region in self.container.action_regions(tab, tab_bounds) {
                        context.draw_component(&region);
                    }
                }
            }
        }
        self.container.paint_tabs(context.scene_mut(), self.layout);
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_component(&self.layout.tab_list);
        self.container.paint_tabs(scene, self.layout);
    }
}

impl Component for TabContainer<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("TabContainer")
            .in_bounds(self.bounds)
            .with_identity(super::identity::TAB_CONTAINER)
    }

    fn interaction_node(&self, element: &ComputedElement) -> Option<UiNode> {
        Some(
            UiNode::new(
                super::identity::TAB_CONTAINER,
                element.bounds(),
                AccessibilityRole::Group,
                "Workbench tabs",
            )
            .with_parent(super::identity::WINDOW),
        )
    }

    fn compose(&self, context: &mut ComponentContext<'_, '_>, _element: &ComputedElement) {
        context.scene_mut().draw_rect(
            PaintRect::new(self.bounds, self.style.colors.side_bar_background).with_border(
                Border::new(Edges::new(0.0, 1.0, 0.0, 0.0), self.style.colors.border),
            ),
        );
        context.draw_component(&TabContainerHeader {
            bounds: self.layout.header,
            toolbar: &self.toolbar,
        });
        context.draw_component(&TabContainerContent { container: self });
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(
            PaintRect::new(self.bounds, self.style.colors.side_bar_background).with_border(
                Border::new(Edges::new(0.0, 1.0, 0.0, 0.0), self.style.colors.border),
            ),
        );
        scene.draw_component(&TabContainerHeader {
            bounds: self.layout.header,
            toolbar: &self.toolbar,
        });
        scene.draw_component(&TabContainerContent { container: self });
    }
}

#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tests;
