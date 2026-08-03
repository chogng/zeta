use zeta_ui::{Rect, ScrollAxis, ScrollCommand, ScrollMetrics, ScrollState, Size};

/// Presentation-only state for the scrollable View mounted above the Composer.
///
/// The pane does not know which product View is mounted. Callers provide only viewport and content
/// geometry; the pane retains the resulting scroll position and delegates clipping, translation,
/// scrollbar geometry, and paint to `zeta_ui::ScrollView`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct ComposerInteractionPaneState {
    scroll_state: ScrollState,
}

impl ComposerInteractionPaneState {
    pub(crate) const fn scroll_state(self) -> ScrollState {
        self.scroll_state
    }

    pub(crate) fn apply_scroll(
        &mut self,
        command: ScrollCommand,
        viewport: Size,
        content: Size,
    ) -> bool {
        self.scroll_state.apply(
            command,
            ScrollMetrics::new(viewport, content),
            ScrollAxis::Vertical,
        )
    }

    pub(crate) fn ensure_visible(
        &mut self,
        content_bounds: Rect,
        viewport: Size,
        content: Size,
    ) -> bool {
        self.apply_scroll(
            ScrollCommand::EnsureVisible(content_bounds),
            viewport,
            content,
        )
    }

    pub(crate) fn reset(&mut self) {
        self.scroll_state = ScrollState::default();
    }
}

#[cfg(test)]
#[path = "composer_interaction_pane_tests.rs"]
mod tests;
