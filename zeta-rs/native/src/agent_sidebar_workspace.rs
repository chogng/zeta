use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::Instant;

use zeta_file_search::{PathSearchHandle, PathSearchOptions, PathSearchSnapshot};
use zeta_ui::{Point, Rect, Size, TextInput, TextInputCommand, TextInputCompositionEvent};
use zeta_ui_dispatch::ElementId;

use crate::editor_pane::{EditorPaneState, ScrollbarPointerOutcome};
use crate::workspace_context::WorkspaceContext;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum AgentSidebarView {
    Changes,
    #[default]
    Files,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExplorerEntry {
    label: String,
    directory: bool,
}

impl ExplorerEntry {
    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) const fn is_directory(&self) -> bool {
        self.directory
    }
}

pub(crate) struct AgentSidebarWorkspace {
    active_view: AgentSidebarView,
    editor: EditorPaneState,
    root: Option<PathBuf>,
    root_entries: Vec<ExplorerEntry>,
    file_search_input: TextInput,
    search_visible: bool,
    search_handle: Option<PathSearchHandle>,
    search_receiver: Option<Receiver<PathSearchSnapshot>>,
    search_revision: u64,
    search_matches: Vec<PathBuf>,
    search_pending: bool,
}

impl Default for AgentSidebarWorkspace {
    fn default() -> Self {
        Self {
            active_view: AgentSidebarView::Files,
            editor: EditorPaneState::default(),
            root: None,
            root_entries: Vec::new(),
            file_search_input: TextInput::new(),
            search_visible: false,
            search_handle: None,
            search_receiver: None,
            search_revision: 0,
            search_matches: Vec::new(),
            search_pending: false,
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

    pub(crate) fn root_entries(&self) -> &[ExplorerEntry] {
        &self.root_entries
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

    pub(crate) fn set_search_visible(&mut self, visible: bool) {
        self.search_visible = visible;
        if !visible {
            self.file_search_input.take_text();
            self.update_file_search();
        }
    }

    pub(crate) fn apply_file_search(&mut self, command: TextInputCommand) {
        self.file_search_input.apply(command);
        self.update_file_search();
    }

    pub(crate) fn apply_file_search_composition(&mut self, event: TextInputCompositionEvent) {
        self.file_search_input.apply_composition(event);
        self.update_file_search();
    }

    pub(crate) fn cancel_file_search_composition(&mut self) {
        self.file_search_input.cancel_composition();
    }

    pub(crate) fn clear_file_search(&mut self) {
        self.file_search_input.take_text();
        self.update_file_search();
    }

    pub(crate) fn selected_file_search_text(&self) -> Option<&str> {
        self.file_search_input.selected_text()
    }

    pub(crate) fn refresh_files(&mut self) {
        self.root_entries = self
            .root
            .as_deref()
            .map(read_root_entries)
            .unwrap_or_default();
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

fn read_root_entries(root: &Path) -> Vec<ExplorerEntry> {
    let mut entries = std::fs::read_dir(root)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            let label = entry.file_name().to_string_lossy().into_owned();
            (!matches!(label.as_str(), ".git" | ".zeta" | "node_modules" | "target")).then_some(
                ExplorerEntry {
                    label,
                    directory: file_type.is_dir(),
                },
            )
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        right
            .directory
            .cmp(&left.directory)
            .then_with(|| left.label.cmp(&right.label))
    });
    entries
}

#[cfg(test)]
#[path = "agent_sidebar_workspace_tests.rs"]
mod tests;
