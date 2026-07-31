use std::path::PathBuf;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Instant;

use zeta_editor::MultiDiffEditorStyle;
use zeta_file_search::{PathSearchHandle, PathSearchOptions, PathSearchSnapshot};
use zeta_ui::{
    Point, Rect, ScrollAxis, ScrollCommand, ScrollDelta, ScrollMetrics, ScrollState, Size,
    TextInput, TextInputCommand, TextInputCompositionEvent, TreeItem, VirtualListLayout,
};
use zeta_ui_dispatch::ElementId;

use crate::editor_pane::{EditorPaneState, ScrollbarPointerOutcome};
use crate::explorer_pane::FILE_LIST_ROW_HEIGHT;
#[cfg(test)]
use crate::explorer_tree::ExplorerEntry;
use crate::explorer_tree::{ExplorerTree, ExplorerTreeNavigation, ExplorerTreeRow};
use crate::workspace_context::WorkspaceContext;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum AgentSidebarView {
    Changes,
    #[default]
    Files,
}

pub(crate) struct AgentSidebarWorkspace {
    active_view: AgentSidebarView,
    editor: EditorPaneState,
    root: Option<PathBuf>,
    explorer_tree: ExplorerTree,
    file_search_input: TextInput,
    search_visible: bool,
    search_handle: Option<PathSearchHandle>,
    search_receiver: Option<Receiver<PathSearchSnapshot>>,
    search_revision: u64,
    search_matches: Vec<PathBuf>,
    search_pending: bool,
    file_list_scroll_state: ScrollState,
}

impl Default for AgentSidebarWorkspace {
    fn default() -> Self {
        Self {
            active_view: AgentSidebarView::Files,
            editor: EditorPaneState::default(),
            root: None,
            explorer_tree: ExplorerTree::default(),
            file_search_input: TextInput::new(),
            search_visible: false,
            search_handle: None,
            search_receiver: None,
            search_revision: 0,
            search_matches: Vec::new(),
            search_pending: false,
            file_list_scroll_state: ScrollState::default(),
        }
    }
}

impl AgentSidebarWorkspace {
    pub(crate) fn new(context: &WorkspaceContext) -> Self {
        let mut workspace = Self {
            root: Some(context.working_directory().to_path_buf()),
            ..Self::default()
        };
        workspace.sync_repository(context);
        workspace.refresh_files();
        workspace
    }

    pub(crate) const fn active_view(&self) -> AgentSidebarView {
        self.active_view
    }

    pub(crate) fn select_view(&mut self, view: AgentSidebarView) {
        self.active_view = view;
    }

    pub(crate) const fn editor(&self) -> &EditorPaneState {
        &self.editor
    }

    pub(crate) fn set_editor_style(&mut self, style: MultiDiffEditorStyle) {
        self.editor.set_style(style);
    }

    #[cfg(test)]
    pub(crate) fn root_entries(&self) -> Vec<&ExplorerEntry> {
        self.explorer_tree.root_entries()
    }

    pub(crate) fn file_tree_items(&self) -> &[TreeItem] {
        self.explorer_tree.visible_items()
    }

    pub(crate) fn file_tree_row(&self, index: usize) -> Option<ExplorerTreeRow<'_>> {
        self.explorer_tree.row(index)
    }

    pub(crate) fn activate_file_tree_element(&mut self, element: ElementId) -> bool {
        self.explorer_tree.activate_element(element)
    }

    pub(crate) fn navigate_file_tree_right(
        &mut self,
        element: ElementId,
    ) -> Option<ExplorerTreeNavigation> {
        self.explorer_tree.navigate_right(element)
    }

    pub(crate) fn navigate_file_tree_left(
        &mut self,
        element: ElementId,
    ) -> Option<ExplorerTreeNavigation> {
        self.explorer_tree.navigate_left(element)
    }

    pub(crate) const fn search_visible(&self) -> bool {
        self.search_visible
    }

    pub(crate) const fn file_search_input(&self) -> &TextInput {
        &self.file_search_input
    }

    pub(crate) fn search_matches(&self) -> &[PathBuf] {
        &self.search_matches
    }

    pub(crate) const fn file_list_scroll_state(&self) -> ScrollState {
        self.file_list_scroll_state
    }

    pub(crate) fn file_list_item_count(&self) -> usize {
        if self.search_visible && !self.file_search_input.text().trim().is_empty() {
            self.search_matches.len()
        } else {
            self.explorer_tree.visible_len()
        }
    }

    pub(crate) fn set_search_visible(&mut self, visible: bool) {
        self.search_visible = visible;
        self.file_list_scroll_state = ScrollState::default();
        if !visible {
            self.file_search_input.take_text();
            self.update_file_search();
        }
    }

    pub(crate) fn apply_file_search(&mut self, command: TextInputCommand) {
        self.file_search_input.apply(command);
        self.file_list_scroll_state = ScrollState::default();
        self.update_file_search();
    }

    pub(crate) fn apply_file_search_composition(&mut self, event: TextInputCompositionEvent) {
        self.file_search_input.apply_composition(event);
        self.file_list_scroll_state = ScrollState::default();
        self.update_file_search();
    }

    pub(crate) fn cancel_file_search_composition(&mut self) {
        self.file_search_input.cancel_composition();
    }

    pub(crate) fn clear_file_search(&mut self) {
        self.file_search_input.take_text();
        self.file_list_scroll_state = ScrollState::default();
        self.update_file_search();
    }

    pub(crate) fn selected_file_search_text(&self) -> Option<&str> {
        self.file_search_input.selected_text()
    }

    pub(crate) fn refresh_files(&mut self) {
        self.file_list_scroll_state = ScrollState::default();
        self.explorer_tree.replace_root(self.root.as_deref());
        let Some(root) = self.root.clone() else {
            return;
        };
        match PathSearchHandle::start(root, PathSearchOptions::default()) {
            Ok((handle, receiver)) => {
                self.search_revision = handle.update_query(self.file_search_input.text());
                self.search_handle = Some(handle);
                self.search_receiver = Some(receiver);
                self.search_matches.clear();
                self.search_pending = true;
            }
            Err(error) => {
                eprintln!("could not index workspace files: {error}");
                self.search_handle = None;
                self.search_receiver = None;
                self.search_matches.clear();
                self.search_pending = false;
            }
        }
    }

    pub(crate) fn replace_workspace(&mut self, context: &WorkspaceContext) {
        self.root = Some(context.working_directory().to_path_buf());
        self.sync_repository(context);
        self.refresh_files();
    }

    pub(crate) fn poll_file_search(&mut self) -> bool {
        let Some(receiver) = self.search_receiver.as_ref() else {
            return false;
        };
        let mut latest = None;
        loop {
            match receiver.try_recv() {
                Ok(snapshot) => latest = Some(snapshot),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    self.search_pending = false;
                    break;
                }
            }
        }
        let Some(snapshot) = latest else {
            return false;
        };
        if snapshot.query_revision != self.search_revision
            || snapshot.query != self.file_search_input.text()
        {
            return false;
        }
        let matches = snapshot
            .matches
            .into_iter()
            .map(|matched| matched.path)
            .collect::<Vec<_>>();
        let changed = matches != self.search_matches;
        self.search_matches = matches;
        self.search_pending = !snapshot.search_complete;
        changed
    }

    pub(crate) const fn file_search_pending(&self) -> bool {
        self.search_pending
    }

    pub(crate) fn sync_repository(&mut self, context: &WorkspaceContext) {
        self.editor.replace_diffs(context.diffs());
    }

    pub(crate) fn scroll_multi_diff(&mut self, delta: f32, viewport: Size, now: Instant) -> bool {
        self.editor.scroll(delta, viewport, now)
    }

    pub(crate) fn scroll_file_list(&mut self, delta: f32, viewport: Size) -> bool {
        let content = VirtualListLayout::new(self.file_list_item_count(), FILE_LIST_ROW_HEIGHT)
            .content_size(viewport.width);
        self.file_list_scroll_state.apply(
            ScrollCommand::ByPixels(ScrollDelta::vertical(delta)),
            ScrollMetrics::new(viewport, content),
            ScrollAxis::Vertical,
        )
    }

    pub(crate) fn toggle_multi_diff_fold(&mut self, id: ElementId) -> bool {
        self.editor.toggle_fold_for_element(id)
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

    fn update_file_search(&mut self) {
        if let Some(handle) = self.search_handle.as_ref() {
            self.search_revision = handle.update_query(self.file_search_input.text());
            self.search_pending = true;
        }
    }
}

#[cfg(test)]
#[path = "agent_sidebar_workspace_tests.rs"]
mod tests;
