use zeta_icons::icons;
use zeta_ui::{
    Component, ComponentElement, Element, ListView, PaintIcon, PaintRect, Rect, TextBlock,
    TextStyle, TreeItemExpansion, TreeItemLayout, TreeView, TreeViewStyle, UiScene,
};
use zui::{
    AccessibilityExpansion, AccessibilityRole, CursorFeedback, FocusBehavior, InteractionFrame,
    NavigationAxis, NavigationGroupId, NodeAction, UiNode,
};

use crate::agent_sidebar_workspace::AgentSidebarWorkspace;
use crate::shell_interaction::{AGENT_EXPLORER_PANE, AGENT_SIDEBAR};
use crate::shell_style::ShellPalette;

pub(crate) const FILE_LIST_ROW_HEIGHT: f32 = 24.0;
const HORIZONTAL_PADDING: f32 = 10.0;
const ICON_SIZE: f32 = 14.0;
const SEARCH_RESULT_SCOPE: u32 = 6;
const OVERSCAN_ITEMS: usize = 2;

/// Product file tree and fuzzy path results hosted by the Files pane.
pub(crate) struct ExplorerPane<'a> {
    bounds: Rect,
    workspace: &'a AgentSidebarWorkspace,
    palette: ShellPalette,
}

impl<'a> ExplorerPane<'a> {
    pub(crate) const fn new(
        bounds: Rect,
        workspace: &'a AgentSidebarWorkspace,
        palette: ShellPalette,
    ) -> Self {
        Self {
            bounds,
            workspace,
            palette,
        }
    }

    pub(crate) fn register_interactions(&self, frame: &mut InteractionFrame) {
        if self.showing_search_results() {
            self.register_search_interactions(frame);
        } else {
            self.register_tree_interactions(frame);
        }
    }

    fn register_tree_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                AGENT_EXPLORER_PANE,
                self.bounds,
                AccessibilityRole::Tree,
                "Files",
            )
            .with_parent(AGENT_SIDEBAR),
        );
        let tree = self.tree_view();
        let navigation = NavigationGroupId::new(AGENT_EXPLORER_PANE);
        for index in tree.visible_range() {
            let Some(row) = self.workspace.file_tree_row(index) else {
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
                .with_parent(AGENT_EXPLORER_PANE)
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
                AGENT_EXPLORER_PANE,
                self.bounds,
                AccessibilityRole::List,
                "File search results",
            )
            .with_parent(AGENT_SIDEBAR),
        );
        let list = self.search_list_view();
        for index in list.visible_range() {
            let Some(path) = self.workspace.search_matches().get(index) else {
                continue;
            };
            frame.register(
                UiNode::new(
                    search_result_element_id(index),
                    list.item_bounds(index).expect("visible file search item"),
                    AccessibilityRole::ListItem,
                    path.to_string_lossy().replace('\\', "/"),
                )
                .with_parent(AGENT_EXPLORER_PANE),
            );
        }
    }

    fn tree_view(&self) -> TreeView<'_> {
        TreeView::new(
            self.bounds,
            self.workspace.file_tree_items(),
            self.workspace.file_list_scroll_state(),
            TreeViewStyle::new(
                self.palette.file_list_scroll_view_style(),
                FILE_LIST_ROW_HEIGHT,
            ),
        )
        .with_overscan_items(OVERSCAN_ITEMS)
    }

    fn search_list_view(&self) -> ListView {
        ListView::new(
            self.bounds,
            self.workspace.search_matches().len(),
            FILE_LIST_ROW_HEIGHT,
            self.workspace.file_list_scroll_state(),
            self.palette.file_list_scroll_view_style(),
        )
        .with_overscan_items(OVERSCAN_ITEMS)
    }

    fn showing_search_results(&self) -> bool {
        self.workspace.search_visible()
            && !self.workspace.file_search_input().text().trim().is_empty()
    }
}

impl Component for ExplorerPane<'_> {
    fn element(&self) -> ComponentElement {
        Element::leaf("ExplorerPane").in_bounds(self.bounds)
    }

    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(PaintRect::new(self.bounds, self.palette.surface));
        if self.workspace.file_list_item_count() == 0 {
            draw_empty(
                scene,
                self.bounds,
                self.palette,
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
                let Some(path) = self.workspace.search_matches().get(item.index()) else {
                    return;
                };
                draw_search_row(
                    scene,
                    item.bounds(),
                    &path.to_string_lossy().replace('\\', "/"),
                    self.palette,
                );
            });
        } else {
            self.tree_view().draw(scene, |scene, layout| {
                let Some(row) = self.workspace.file_tree_row(layout.index()) else {
                    return;
                };
                draw_tree_row(scene, layout, row.entry().label(), self.palette);
            });
        }
    }
}

fn draw_tree_row(scene: &mut UiScene, layout: TreeItemLayout, label: &str, palette: ShellPalette) {
    if let Some(disclosure_bounds) = layout.disclosure_bounds() {
        let icon = match layout.item().expansion() {
            TreeItemExpansion::Expanded => icons::CHEVRON_DOWN,
            TreeItemExpansion::Collapsed => icons::CHEVRON_RIGHT,
            TreeItemExpansion::Leaf => unreachable!("leaf has no disclosure geometry"),
        };
        scene.draw_icon(PaintIcon::new(icon, disclosure_bounds, palette.text_muted));
    }
    let content = layout.content_bounds();
    let icon_bounds = Rect::from_xywh(
        content.origin.x,
        content.origin.y + (FILE_LIST_ROW_HEIGHT - ICON_SIZE) * 0.5,
        ICON_SIZE,
        ICON_SIZE,
    );
    if layout.item().expansion().is_branch() {
        scene.draw_icon(PaintIcon::new(
            icons::FILES,
            icon_bounds,
            palette.text_muted,
        ));
    }
    let text_x = icon_bounds.right() + 6.0;
    scene.draw_text(TextBlock::new(
        label,
        zeta_ui::Point::new(text_x, content.origin.y + 4.0),
        zeta_ui::Size::new(
            (layout.bounds().right() - text_x - HORIZONTAL_PADDING).max(1.0),
            18.0,
        ),
        TextStyle::new(12.0, palette.text).with_line_height(18.0),
    ));
}

fn draw_search_row(scene: &mut UiScene, bounds: Rect, label: &str, palette: ShellPalette) {
    let text_x = bounds.origin.x + HORIZONTAL_PADDING + ICON_SIZE + 6.0;
    scene.draw_text(TextBlock::new(
        label,
        zeta_ui::Point::new(text_x, bounds.origin.y + 4.0),
        zeta_ui::Size::new(
            (bounds.right() - text_x - HORIZONTAL_PADDING).max(1.0),
            18.0,
        ),
        TextStyle::new(12.0, palette.text).with_line_height(18.0),
    ));
}

fn search_result_element_id(index: usize) -> zui::ElementId {
    zui::ElementId::scoped(
        SEARCH_RESULT_SCOPE,
        u32::try_from(index).unwrap_or(u32::MAX).saturating_add(1),
    )
}

fn draw_empty(scene: &mut UiScene, bounds: Rect, palette: ShellPalette, label: &str) {
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
        TextStyle::new(12.0, palette.text_muted).with_line_height(18.0),
    ));
}

#[cfg(test)]
#[path = "explorer_pane_tests.rs"]
mod tests;
