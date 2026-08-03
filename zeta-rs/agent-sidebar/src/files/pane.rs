use zeta_icons::icons;
use zeta_ui::{
    Component, ComponentElement, Element, ListView, PaintIcon, PaintRect, Rect, TextBlock,
    TextStyle, TreeItemExpansion, TreeItemLayout, TreeView, TreeViewStyle, UiScene,
};
use zui::UiDispatch;
use zui::{
    AccessibilityExpansion, AccessibilityRole, CursorFeedback, FocusBehavior, InteractionFrame,
    NavigationAxis, NavigationGroupId, NodeAction, UiNode,
};

use super::FILE_LIST_ROW_HEIGHT;
use super::FilesState;

use zeta_ui::Color;
use zeta_ui::ScrollViewStyle;
use zui::ElementId;

pub const EXPLORER_PANE: ElementId = ElementId::scoped(1, 28);
const HORIZONTAL_PADDING: f32 = 10.0;
const ICON_SIZE: f32 = 14.0;
const SEARCH_RESULT_SCOPE: u32 = 6;
const OVERSCAN_ITEMS: usize = 2;

#[derive(Clone, Copy)]
enum FileRowState {
    Resting,
    Hovered,
    Selected,
}

/// Product file tree and fuzzy path results hosted by the Files pane.
pub struct FilesPaneStyle {
    pub surface: Color,
    pub selected_background: Color,
    pub hovered_background: Color,
    pub text: Color,
    pub text_muted: Color,
    pub scroll_view: ScrollViewStyle,
}

/// Files-pane rendering and interaction registration over retained `FilesState`.
pub struct FilesPane<'a> {
    bounds: Rect,
    files: &'a FilesState,
    parent: ElementId,
    style: &'a FilesPaneStyle,
    dispatch: &'a UiDispatch,
}

impl<'a> FilesPane<'a> {
    pub const fn new(
        bounds: Rect,
        files: &'a FilesState,
        parent: ElementId,
        style: &'a FilesPaneStyle,
        dispatch: &'a UiDispatch,
    ) -> Self {
        Self {
            bounds,
            files,
            parent,
            style,
            dispatch,
        }
    }

    pub fn register_interactions(&self, frame: &mut InteractionFrame) {
        if self.showing_search_results() {
            self.register_search_interactions(frame);
        } else {
            self.register_tree_interactions(frame);
        }
    }

    fn register_tree_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(EXPLORER_PANE, self.bounds, AccessibilityRole::Tree, "Files")
                .with_parent(self.parent),
        );
        let tree = self.tree_view();
        let navigation = NavigationGroupId::new(EXPLORER_PANE);
        for index in tree.visible_range() {
            let Some(row) = self.files.tree_row(index) else {
                continue;
            };
            let entry = row.entry();
            let Some(layout) = tree.item_layout(index) else {
                continue;
            };
            let expansion = if !entry.is_directory() {
                AccessibilityExpansion::NotApplicable
            } else if entry.is_expanded() {
                AccessibilityExpansion::Expanded
            } else {
                AccessibilityExpansion::Collapsed
            };
            frame.register(
                UiNode::new(
                    entry.element_id(),
                    layout.bounds(),
                    AccessibilityRole::TreeItem,
                    entry.label(),
                )
                .with_parent(EXPLORER_PANE)
                .with_cursor(CursorFeedback::Pointer)
                .with_focus(FocusBehavior::TabStop)
                .with_action(NodeAction::Activate)
                .with_navigation(navigation, NavigationAxis::Vertical)
                .with_level(row.depth() + 1)
                .with_expansion(expansion),
            );
        }
    }

    fn register_search_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                EXPLORER_PANE,
                self.bounds,
                AccessibilityRole::List,
                "File search results",
            )
            .with_parent(self.parent),
        );
        let list = self.search_list_view();
        for index in list.visible_range() {
            let Some(path) = self.files.search_matches().get(index) else {
                continue;
            };
            frame.register(
                UiNode::new(
                    search_result_element_id(index),
                    list.item_bounds(index).expect("visible file search item"),
                    AccessibilityRole::ListItem,
                    path.to_string_lossy().replace('\\', "/"),
                )
                .with_parent(EXPLORER_PANE),
            );
        }
    }

    fn tree_view(&self) -> TreeView<'_> {
        TreeView::new(
            self.bounds,
            self.files.tree_items(),
            self.files.scroll_state(),
            TreeViewStyle::new(self.style.scroll_view, FILE_LIST_ROW_HEIGHT),
        )
        .with_overscan_items(OVERSCAN_ITEMS)
    }

    fn search_list_view(&self) -> ListView {
        ListView::new(
            self.bounds,
            self.files.search_matches().len(),
            FILE_LIST_ROW_HEIGHT,
            self.files.scroll_state(),
            self.style.scroll_view,
        )
        .with_overscan_items(OVERSCAN_ITEMS)
    }

    fn showing_search_results(&self) -> bool {
        self.files.search_visible() && !self.files.search_input().text().trim().is_empty()
    }
}

impl Component for FilesPane<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("FilesPane").in_bounds(self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(PaintRect::new(self.bounds, self.style.surface));
        let focused = self.dispatch.focused();
        let selected = self.files.selected_element();
        if self.files.item_count() == 0 {
            draw_empty(
                scene,
                self.bounds,
                self.style,
                if self.showing_search_results() {
                    "No matching files"
                } else {
                    "No files loaded"
                },
            );
            return;
        }
        if self.showing_search_results() {
            self.search_list_view().draw(scene, |scene, item| {
                let Some(path) = self.files.search_matches().get(item.index()) else {
                    return;
                };
                let element = search_result_element_id(item.index());
                draw_search_row(
                    scene,
                    item.bounds(),
                    &path.to_string_lossy().replace('\\', "/"),
                    self.style,
                    file_row_state(focused == Some(element), self.dispatch.is_hovered(element)),
                );
            });
        } else {
            self.tree_view().draw(scene, |scene, layout| {
                let Some(row) = self.files.tree_row(layout.index()) else {
                    return;
                };
                let element = row.entry().element_id();
                draw_tree_row(
                    scene,
                    layout,
                    row.entry().label(),
                    self.style,
                    file_row_state(
                        selected == Some(element) || focused == Some(element),
                        self.dispatch.is_hovered(element),
                    ),
                );
            });
        }
    }
}

fn draw_tree_row(
    scene: &mut UiScene,
    layout: TreeItemLayout,
    label: &str,
    style: &FilesPaneStyle,
    state: FileRowState,
) {
    draw_row_background(scene, layout.bounds(), style, state);
    if let Some(disclosure_bounds) = layout.disclosure_bounds() {
        let icon = match layout.item().expansion() {
            TreeItemExpansion::Expanded => icons::CHEVRON_DOWN,
            TreeItemExpansion::Collapsed => icons::CHEVRON_RIGHT,
            TreeItemExpansion::Leaf => unreachable!("leaf has no disclosure geometry"),
        };
        scene.draw_icon(PaintIcon::new(icon, disclosure_bounds, style.text_muted));
    }
    let content = layout.content_bounds();
    let icon_bounds = Rect::from_xywh(
        content.origin.x,
        content.origin.y + (FILE_LIST_ROW_HEIGHT - ICON_SIZE) * 0.5,
        ICON_SIZE,
        ICON_SIZE,
    );
    if layout.item().expansion().is_branch() {
        scene.draw_icon(PaintIcon::new(icons::FILES, icon_bounds, style.text_muted));
    }
    let text_x = icon_bounds.right() + 6.0;
    scene.draw_text(TextBlock::new(
        label,
        zeta_ui::Point::new(text_x, content.origin.y + 4.0),
        zeta_ui::Size::new(
            (layout.bounds().right() - text_x - HORIZONTAL_PADDING).max(1.0),
            18.0,
        ),
        TextStyle::new(12.0, style.text).with_line_height(18.0),
    ));
}

fn draw_search_row(
    scene: &mut UiScene,
    bounds: Rect,
    label: &str,
    style: &FilesPaneStyle,
    state: FileRowState,
) {
    draw_row_background(scene, bounds, style, state);
    let text_x = bounds.origin.x + HORIZONTAL_PADDING + ICON_SIZE + 6.0;
    scene.draw_text(TextBlock::new(
        label,
        zeta_ui::Point::new(text_x, bounds.origin.y + 4.0),
        zeta_ui::Size::new(
            (bounds.right() - text_x - HORIZONTAL_PADDING).max(1.0),
            18.0,
        ),
        TextStyle::new(12.0, style.text).with_line_height(18.0),
    ));
}

fn file_row_state(selected: bool, hovered: bool) -> FileRowState {
    if selected {
        FileRowState::Selected
    } else if hovered {
        FileRowState::Hovered
    } else {
        FileRowState::Resting
    }
}

fn draw_row_background(
    scene: &mut UiScene,
    bounds: Rect,
    style: &FilesPaneStyle,
    state: FileRowState,
) {
    let background = match state {
        FileRowState::Resting => return,
        FileRowState::Hovered => style.hovered_background,
        FileRowState::Selected => style.selected_background,
    };
    scene.draw_rect(PaintRect::new(bounds, background));
}

fn search_result_element_id(index: usize) -> zui::ElementId {
    zui::ElementId::scoped(
        SEARCH_RESULT_SCOPE,
        u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1),
    )
}

fn draw_empty(scene: &mut UiScene, bounds: Rect, style: &FilesPaneStyle, label: &str) {
    scene.draw_text(TextBlock::new(
        label,
        zeta_ui::Point::new(
            bounds.origin.x + HORIZONTAL_PADDING,
            bounds.origin.y + HORIZONTAL_PADDING,
        ),
        zeta_ui::Size::new(
            (bounds.size.width - HORIZONTAL_PADDING * 2.0).max(1.0),
            18.0,
        ),
        TextStyle::new(12.0, style.text_muted).with_line_height(18.0),
    ));
}

#[cfg(test)]
#[path = "pane_tests.rs"]
mod tests;
