use zeta_icons::icons;
use zeta_ui::{Component, PaintIcon, PaintRect, Rect, TextBlock, TextStyle, UiScene};
use zeta_ui_dispatch::{AccessibilityRole, InteractionFrame, UiNode};

use crate::agent_sidebar_workspace::AgentSidebarWorkspace;
use crate::shell_interaction::{AGENT_EXPLORER_PANE, AGENT_SIDEBAR};
use crate::shell_style::ShellPalette;

const ROW_HEIGHT: f32 = 24.0;
const HORIZONTAL_PADDING: f32 = 10.0;
const ICON_SIZE: f32 = 14.0;

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
        frame.register(
            UiNode::new(
                AGENT_EXPLORER_PANE,
                self.bounds,
                AccessibilityRole::Group,
                "Files",
            )
            .with_parent(AGENT_SIDEBAR),
        );
    }
}

impl Component for ExplorerPane<'_> {
    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(PaintRect::new(self.bounds, self.palette.surface));
        scene.with_clip(self.bounds, |scene| {
            if self.workspace.search_visible()
                && !self.workspace.file_search_input().text().trim().is_empty()
            {
                if self.workspace.search_matches().is_empty() {
                    draw_empty(scene, self.bounds, self.palette, "No matching files");
                    return;
                }
                for (index, path) in self.workspace.search_matches().iter().enumerate() {
                    draw_file_row(
                        scene,
                        self.bounds,
                        index,
                        &path.to_string_lossy().replace('\\', "/"),
                        false,
                        self.palette,
                    );
                }
                return;
            }
            if self.workspace.root_entries().is_empty() {
                draw_empty(scene, self.bounds, self.palette, "No files loaded");
                return;
            }
            for (index, entry) in self.workspace.root_entries().iter().enumerate() {
                draw_file_row(
                    scene,
                    self.bounds,
                    index,
                    entry.label(),
                    entry.is_directory(),
                    self.palette,
                );
            }
        });
    }
}

fn draw_file_row(
    scene: &mut UiScene,
    bounds: Rect,
    index: usize,
    label: &str,
    directory: bool,
    palette: ShellPalette,
) {
    let y = bounds.origin.y + index as f32 * ROW_HEIGHT;
    if y >= bounds.bottom() {
        return;
    }
    let text_x = if directory {
        let icon_bounds = Rect::from_xywh(
            bounds.origin.x + HORIZONTAL_PADDING,
            y + (ROW_HEIGHT - ICON_SIZE) * 0.5,
            ICON_SIZE,
            ICON_SIZE,
        );
        scene.draw_icon(PaintIcon::new(
            icons::FILES,
            icon_bounds,
            palette.text_muted,
        ));
        icon_bounds.right() + 6.0
    } else {
        bounds.origin.x + HORIZONTAL_PADDING + ICON_SIZE + 6.0
    };
    scene.draw_text(TextBlock::new(
        label,
        zeta_ui::Point::new(text_x, y + 4.0),
        zeta_ui::Size::new(
            (bounds.right() - text_x - HORIZONTAL_PADDING).max(1.0),
            18.0,
        ),
        TextStyle::new(12.0, palette.text).with_line_height(18.0),
    ));
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
