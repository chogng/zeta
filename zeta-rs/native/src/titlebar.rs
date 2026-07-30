use zeta_ui::{
    Border, Component, Edges, FontWeight, PaintRect, Rect, TextBlock, TextStyle, UiScene,
};

use crate::shell_interaction::{ShellHitMap, ShellTarget};
use crate::shell_style::ShellPalette;

pub(crate) const TITLEBAR_HEIGHT: f32 = 35.0;
const TITLE_X: f32 = 78.0;

/// Product-owned draggable titlebar for the single terminal surface.
pub(crate) struct Titlebar<'a> {
    bounds: Rect,
    palette: ShellPalette,
    title: &'a str,
}

impl<'a> Titlebar<'a> {
    pub(crate) fn new(bounds: Rect, title: &'a str, palette: ShellPalette) -> Self {
        Self {
            bounds,
            palette,
            title,
        }
    }

    pub(crate) fn register_hit_regions(&self, hit_map: &mut ShellHitMap) {
        hit_map.register(self.bounds, ShellTarget::WindowDrag);
    }
}

impl Component for Titlebar<'_> {
    fn paint(&self, scene: &mut UiScene) {
        scene.draw_rect(
            PaintRect::new(self.bounds, self.palette.surface_raised).with_border(Border::new(
                Edges::new(0.0, 0.0, 1.0, 0.0),
                self.palette.border,
            )),
        );
        let title_bounds = Rect::from_xywh(
            self.bounds.origin.x + TITLE_X,
            self.bounds.origin.y + 7.0,
            180.0,
            22.0,
        );
        scene.draw_text(TextBlock::new(
            self.title,
            title_bounds.origin,
            title_bounds.size,
            TextStyle::new(17.0, self.palette.text).with_weight(FontWeight::Bold),
        ));
    }
}

#[cfg(test)]
#[path = "titlebar_tests.rs"]
mod tests;
