use std::time::Instant;

use zeta_diff::DiffDocument;
use zeta_ui::{Point, Rect, Size};

use crate::editor_pane::{EditorPaneState, ScrollbarPointerOutcome};

/// Product content mounted inside the Agent Sidebar's sibling panes.
#[derive(Default)]
pub(crate) struct AgentSidebarWorkspace {
    editor: EditorPaneState,
}

impl AgentSidebarWorkspace {
    pub(crate) const fn editor(&self) -> &EditorPaneState {
        &self.editor
    }

    pub(crate) fn scroll_multi_diff(&mut self, delta: f32, viewport: Size, now: Instant) -> bool {
        self.editor.scroll(delta, viewport, now)
    }

    pub(crate) fn move_multi_diff_scrollbar(
        &mut self,
        point: Point,
        bounds: Rect,
        now: Instant,
    ) -> ScrollbarPointerOutcome {
        self.editor.scrollbar_pointer_moved(point, bounds, now)
    }

    pub(crate) fn press_multi_diff_scrollbar(
        &mut self,
        point: Point,
        bounds: Rect,
        now: Instant,
    ) -> ScrollbarPointerOutcome {
        self.editor.press_scrollbar(point, bounds, now)
    }

    pub(crate) fn release_multi_diff_scrollbar(
        &mut self,
        point: Point,
        bounds: Rect,
        now: Instant,
    ) -> ScrollbarPointerOutcome {
        self.editor.release_scrollbar(point, bounds, now)
    }

    pub(crate) fn leave_multi_diff_scrollbar(&mut self, now: Instant) -> bool {
        self.editor.scrollbar_pointer_left(now)
    }

    pub(crate) fn cancel_multi_diff_scrollbar(&mut self) {
        self.editor.cancel_scrollbar_interaction();
    }

    pub(crate) fn advance_multi_diff_scrollbar(&mut self, now: Instant) -> bool {
        self.editor.advance_scrollbar(now)
    }

    pub(crate) const fn multi_diff_scrollbar_deadline(&self) -> Option<Instant> {
        self.editor.scrollbar_deadline()
    }

    #[allow(
        dead_code,
        reason = "called once the authoritative changed-file projection is connected"
    )]
    pub(crate) fn open_diff(
        &mut self,
        file_name: impl Into<String>,
        original_label: impl Into<String>,
        modified_label: impl Into<String>,
        document: DiffDocument,
    ) {
        self.editor
            .open_diff(file_name, original_label, modified_label, document)
    }
}
