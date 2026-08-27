use std::time::Instant;

use crate::workspace_panes::DirectoryEntry;
use crate::workspace_panes::EditorPaneState;
use crate::workspace_panes::FilesState;
use crate::workspace_panes::ScmDiff;
use crate::workspace_panes::WorkspacePaneAction;
use crate::workspace_panes::WorkspacePaneState;
use zeta_app_server_protocol::protocol::fs::FsReadDirectoryEntry;
use zeta_editor::MultiDiffEditorStyle;
use zeta_ui::Point;
use zeta_ui::Rect;
use zeta_ui::Size;
use zeta_ui::TextInput;
use zeta_ui::TextInputCommand;
use zeta_ui::TextInputCompositionEvent;
use zui::ui::ElementId;

use crate::workspace_context::WorkspaceContext;
use crate::workspace_panes::ScrollbarPointerOutcome;

#[cfg(test)]
use crate::workspace_panes::FilesTreeRow;
#[cfg(test)]
use zeta_ui::ScrollState;

pub(crate) use crate::workspace_panes::WorkspacePaneView;

pub(crate) struct WorkspacePaneHost {
    state: WorkspacePaneState,
}

impl Default for WorkspacePaneHost {
    fn default() -> Self {
        Self {
            state: WorkspacePaneState::default(),
        }
    }
}

impl WorkspacePaneHost {
    pub(crate) fn new(context: &WorkspaceContext) -> Self {
        let mut workspace = Self {
            state: WorkspacePaneState::new(context.working_directory().to_path_buf()),
            ..Self::default()
        };
        let _ = workspace.sync_repository(context);
        workspace
    }

    pub(crate) const fn editor(&self) -> &EditorPaneState {
        self.state.scm().editor()
    }

    pub(crate) const fn files(&self) -> &FilesState {
        self.state.files()
    }

    pub(crate) fn set_editor_style(&mut self, style: MultiDiffEditorStyle) {
        self.state.scm_mut().editor_mut().set_style(style);
    }

    #[cfg(test)]
    pub(crate) fn file_tree_row(&self, index: usize) -> Option<FilesTreeRow<'_>> {
        self.state.files().tree_row(index)
    }

    pub(crate) fn activate_file_tree_element(
        &mut self,
        element: ElementId,
    ) -> Option<WorkspacePaneAction> {
        self.state.files_mut().activate(element)
    }

    pub(crate) fn navigate_file_tree_right(
        &mut self,
        element: ElementId,
    ) -> Option<WorkspacePaneAction> {
        self.state.files_mut().navigate_right(element)
    }

    pub(crate) fn navigate_file_tree_left(
        &mut self,
        element: ElementId,
    ) -> Option<WorkspacePaneAction> {
        self.state.files_mut().navigate_left(element)
    }

    pub(crate) fn complete_file_tree_directory_load(
        &mut self,
        element: ElementId,
        entries: Vec<FsReadDirectoryEntry>,
    ) -> bool {
        self.state
            .files_mut()
            .complete_directory_load(element, directory_entries(entries))
    }

    pub(crate) const fn search_visible(&self) -> bool {
        self.state.files().search_visible()
    }

    pub(crate) const fn file_search_input(&self) -> &TextInput {
        self.state.files().search_input()
    }

    #[cfg(test)]
    pub(crate) const fn file_list_scroll_state(&self) -> ScrollState {
        self.state.files().scroll_state()
    }

    pub(crate) fn set_search_visible(&mut self, visible: bool) {
        self.state.files_mut().set_search_visible(visible);
    }

    pub(crate) fn apply_file_search(&mut self, command: TextInputCommand) {
        self.state.files_mut().apply_search(command);
    }

    pub(crate) fn apply_file_search_composition(&mut self, event: TextInputCompositionEvent) {
        self.state.files_mut().apply_search_composition(event);
    }

    pub(crate) fn cancel_file_search_composition(&mut self) {
        self.state.files_mut().cancel_search_composition();
    }

    pub(crate) fn clear_file_search(&mut self) {
        self.state.files_mut().clear_search();
    }

    pub(crate) fn selected_file_search_text(&self) -> Option<&str> {
        self.state.files().selected_search_text()
    }

    pub(crate) fn refresh_files(&mut self, entries: Vec<FsReadDirectoryEntry>) {
        self.state.files_mut().refresh(directory_entries(entries));
    }

    pub(crate) fn replace_workspace(
        &mut self,
        context: &WorkspaceContext,
    ) -> Vec<zeta_editor::MultiDiffEditorItemIdentity> {
        self.state
            .replace_workspace(context.working_directory().to_path_buf());
        self.sync_repository(context)
    }

    pub(crate) fn poll_file_search(&mut self) -> bool {
        self.state.files_mut().poll_search()
    }

    pub(crate) const fn file_search_pending(&self) -> bool {
        self.state.files().search_pending()
    }

    pub(crate) fn sync_repository(
        &mut self,
        context: &WorkspaceContext,
    ) -> Vec<zeta_editor::MultiDiffEditorItemIdentity> {
        let diffs = context
            .diffs()
            .iter()
            .map(|diff| ScmDiff::new(diff.path(), diff.document().clone()))
            .collect::<Vec<_>>();
        self.state.scm_mut().replace_diffs(diffs)
    }

    pub(crate) fn scroll_multi_diff(&mut self, delta: f32, viewport: Size, now: Instant) -> bool {
        self.state
            .scm_mut()
            .editor_mut()
            .scroll(delta, viewport, now)
    }

    pub(crate) fn scroll_file_list(&mut self, delta: f32, viewport: Size) -> bool {
        self.state.files_mut().scroll(delta, viewport)
    }

    pub(crate) fn toggle_multi_diff_fold(&mut self, id: ElementId) -> bool {
        self.state
            .scm_mut()
            .editor_mut()
            .toggle_fold_for_element(id)
    }

    pub(crate) fn move_multi_diff_scrollbar(
        &mut self,
        point: Point,
        bounds: Rect,
        now: Instant,
    ) -> ScrollbarPointerOutcome {
        self.state
            .scm_mut()
            .editor_mut()
            .scrollbar_pointer_moved(point, bounds, now)
    }

    pub(crate) fn press_multi_diff_scrollbar(
        &mut self,
        point: Point,
        bounds: Rect,
        now: Instant,
    ) -> ScrollbarPointerOutcome {
        self.state
            .scm_mut()
            .editor_mut()
            .press_scrollbar(point, bounds, now)
    }

    pub(crate) fn release_multi_diff_scrollbar(
        &mut self,
        point: Point,
        bounds: Rect,
        now: Instant,
    ) -> ScrollbarPointerOutcome {
        self.state
            .scm_mut()
            .editor_mut()
            .release_scrollbar(point, bounds, now)
    }

    pub(crate) fn leave_multi_diff_scrollbar(&mut self, now: Instant) -> bool {
        self.state
            .scm_mut()
            .editor_mut()
            .scrollbar_pointer_left(now)
    }

    pub(crate) fn cancel_multi_diff_scrollbar(&mut self) {
        self.state
            .scm_mut()
            .editor_mut()
            .cancel_scrollbar_interaction();
    }

    pub(crate) fn advance_multi_diff_scrollbar(&mut self, now: Instant) -> bool {
        self.state.scm_mut().editor_mut().advance_scrollbar(now)
    }

    pub(crate) const fn multi_diff_scrollbar_deadline(&self) -> Option<Instant> {
        self.state.scm().editor().scrollbar_deadline()
    }
}

fn directory_entries(entries: Vec<FsReadDirectoryEntry>) -> Vec<DirectoryEntry> {
    entries
        .into_iter()
        .map(|entry| {
            if entry.file_type == zeta_app_server_protocol::protocol::fs::FsFileType::Directory {
                DirectoryEntry::directory(entry.name)
            } else {
                DirectoryEntry::file(entry.name)
            }
        })
        .collect()
}

#[cfg(test)]
#[path = "workspace_pane_host_tests.rs"]
mod tests;
