use zeta_ui::{ScrollAxis, ScrollCommand, ScrollMetrics, ScrollState, Size};

/// Presentation-only state for the scrollable View mounted above the Composer.
///
/// The pane does not know which product View is mounted. Callers provide only viewport and content
/// geometry; the pane retains the resulting scroll position and delegates clipping, translation,
/// scrollbar geometry, and paint to `zeta_ui::ScrollView`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ComposerInteractionPaneState {
    scroll_state: ScrollState,
}

impl ComposerInteractionPaneState {
    pub const fn scroll_state(self) -> ScrollState {
        self.scroll_state
    }

    pub fn apply_scroll(&mut self, command: ScrollCommand, viewport: Size, content: Size) -> bool {
        self.scroll_state.apply(
            command,
            ScrollMetrics::new(viewport, content),
            ScrollAxis::Vertical,
        )
    }

    pub fn reset(&mut self) {
        self.scroll_state = ScrollState::default();
    }
}

#[cfg(test)]
#[path = "interaction_pane_tests.rs"]
mod tests;
