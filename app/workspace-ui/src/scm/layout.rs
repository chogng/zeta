use zeta_ui::Rect;

/// Height reserved by the Changes functional toolbar slot.
pub const SCM_TOOLBAR_HEIGHT: f32 = 36.0;

/// SCM-owned layout for its toolbar slot and multi-diff content.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScmLayout {
    toolbar: Rect,
    content: Rect,
}

impl ScmLayout {
    pub fn for_bounds(bounds: Rect) -> Self {
        let toolbar_height = SCM_TOOLBAR_HEIGHT.min(bounds.size.height.max(0.0));
        Self {
            toolbar: Rect::from_xywh(
                bounds.origin.x,
                bounds.origin.y,
                bounds.size.width.max(0.0),
                toolbar_height,
            ),
            content: Rect::from_xywh(
                bounds.origin.x,
                bounds.origin.y + toolbar_height,
                bounds.size.width.max(0.0),
                (bounds.size.height - toolbar_height).max(0.0),
            ),
        }
    }

    pub const fn toolbar(self) -> Rect {
        self.toolbar
    }

    pub const fn content(self) -> Rect {
        self.content
    }
}

#[cfg(test)]
#[path = "layout_tests.rs"]
mod tests;
