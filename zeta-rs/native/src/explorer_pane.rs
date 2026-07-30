use zeta_ui::{Component, FontWeight, PaintRect, Rect, TextBlock, TextStyle, UiScene};
use zeta_ui_dispatch::{AccessibilityRole, InteractionFrame, UiNode};

use crate::shell_interaction::{AGENT_EXPLORER_PANE, AGENT_SIDEBAR};
use crate::shell_style::ShellPalette;

const HEADER_HEIGHT: f32 = 36.0;
const HORIZONTAL_PADDING: f32 = 12.0;

/// Product Explorer Pane hosted as a sibling of EditorPane.
pub(crate) struct ExplorerPane {
    bounds: Rect,
    palette: ShellPalette,
}

impl ExplorerPane {
    pub(crate) const fn new(bounds: Rect, palette: ShellPalette) -> Self {
        Self { bounds, palette }
    }

    pub(crate) fn register_interactions(&self, frame: &mut InteractionFrame) {
        frame.register(
            UiNode::new(
                AGENT_EXPLORER_PANE,
                self.bounds,
                AccessibilityRole::Group,
                "Explorer",
            )
            .with_parent(AGENT_SIDEBAR),
        );
    }
}

impl Component for ExplorerPane {
    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(PaintRect::new(self.bounds, self.palette.surface_raised));
        scene.draw_text(TextBlock::new(
            "Explorer",
            zeta_ui::Point::new(
                self.bounds.origin.x + HORIZONTAL_PADDING,
                self.bounds.origin.y + 10.0,
            ),
            zeta_ui::Size::new(
                (self.bounds.size.width - HORIZONTAL_PADDING * 2.0).max(1.0),
                HEADER_HEIGHT - 10.0,
            ),
            TextStyle::new(12.0, self.palette.text).with_weight(FontWeight::Bold),
        ));
        scene.draw_text(TextBlock::new(
            "No files loaded",
            zeta_ui::Point::new(
                self.bounds.origin.x + HORIZONTAL_PADDING,
                self.bounds.origin.y + HEADER_HEIGHT + 8.0,
            ),
            zeta_ui::Size::new(
                (self.bounds.size.width - HORIZONTAL_PADDING * 2.0).max(1.0),
                18.0,
            ),
            TextStyle::new(12.0, self.palette.text_muted).with_line_height(18.0),
        ));
    }
}
