use zeta_ui::Rect;

pub(crate) const TOOLBAR_HEIGHT: f32 = 36.0;

/// Resolved toolbar and active-pane geometry inside the Agent Sidebar.
///
/// The toolbar owns the pane-switching ActionBar. Implementations must place
/// the selected Changes or Files pane in the single content rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct AgentSidebarLayout {
    toolbar: Rect,
    content: Rect,
}

impl AgentSidebarLayout {
    pub(crate) fn for_bounds(bounds: Rect) -> Self {
        let toolbar_height = TOOLBAR_HEIGHT.min(bounds.size.height.max(0.0));
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

    pub(crate) const fn toolbar(self) -> Rect {
        self.toolbar
    }

    pub(crate) const fn content(self) -> Rect {
        self.content
    }
}

#[cfg(test)]
#[path = "agent_sidebar_layout_tests.rs"]
mod tests;
