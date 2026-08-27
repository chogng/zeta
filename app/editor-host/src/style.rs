//! Host-resolved colors shared by editor overlays.

use zui::ui::Color;

/// Colors required by editor search-adjacent popovers and diagnostic details.
#[derive(Clone, Copy)]
pub struct EditorOverlayStyle {
    pub surface_raised: Color,
    pub border: Color,
    pub text: Color,
    pub text_muted: Color,
    pub surface_hovered: Color,
}

impl EditorOverlayStyle {
    /// Creates the resolved colors used by editor overlays.
    pub const fn new(
        surface_raised: Color,
        border: Color,
        text: Color,
        text_muted: Color,
        surface_hovered: Color,
    ) -> Self {
        Self {
            surface_raised,
            border,
            text,
            text_muted,
            surface_hovered,
        }
    }
}
