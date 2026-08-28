use std::time::Instant;

use zeta_app_server_protocol::protocol::fs::FsReadDirectoryEntry;
use zeta_editor::MultiDiffEditorStyle;
use zeta_files::DirectoryEntry;
use zeta_files::FilesAction;
use zeta_files::FilesState;
use zeta_scm::EditorPaneState;
use zeta_scm::ScmDiff;
use zeta_scm::ScmState;
use zui::ui::ElementId;
use zui::ui::Point;
use zui::ui::Rect;
use zui::ui::Size;
use zui::ui::TextInput;
use zui::ui::TextInputCommand;
use zui::ui::TextInputCompositionEvent;

use crate::workspace_context::WorkspaceContext;
use zeta_scm::ScrollbarPointerOutcome;

#[cfg(test)]
use zeta_files::FilesTreeRow;
#[cfg(test)]
use zeta_ui_components::ScrollState;

pub(crate) use zeta_workbench::WorkspacePaneSelection;

pub(crate) struct WorkspacePaneHost {
    files: FilesState,
    scm: ScmState,
}

impl Default for WorkspacePaneHost {
    fn default() -> Self {
        Self {
            files: FilesState::default(),
            scm: ScmState::default(),
        }
    }
}

impl WorkspacePaneHost {
    pub(crate) fn new(context: &WorkspaceContext) -> Self {
        let mut workspace = Self::default();
        workspace
            .files
            .set_workspace_root(context.working_directory().to_path_buf());
        let _ = workspace.sync_repository(context);
        workspace
    }

    pub(crate) const fn editor(&self) -> &EditorPaneState {
        self.scm.editor()
    }

    pub(crate) const fn files(&self) -> &FilesState {
        &self.files
    }

    pub(crate) fn set_editor_style(&mut self, style: MultiDiffEditorStyle) {
        self.scm.editor_mut().set_style(style);
    }

    #[cfg(test)]
    pub(crate) fn file_tree_row(&self, index: usize) -> Option<FilesTreeRow<'_>> {
        self.files.tree_row(index)
    }

    pub(crate) fn activate_file_tree_element(&mut self, element: ElementId) -> Option<FilesAction> {
        self.files.activate(element)
    }

    pub(crate) fn navigate_file_tree_right(&mut self, element: ElementId) -> Option<FilesAction> {
        self.files.navigate_right(element)
    }

    pub(crate) fn navigate_file_tree_left(&mut self, element: ElementId) -> Option<FilesAction> {
        self.files.navigate_left(element)
    }

    pub(crate) fn complete_file_tree_directory_load(
        &mut self,
        element: ElementId,
        entries: Vec<FsReadDirectoryEntry>,
    ) -> bool {
        self.files
            .complete_directory_load(element, directory_entries(entries))
    }

    pub(crate) const fn search_visible(&self) -> bool {
        self.files.search_visible()
    }

    pub(crate) const fn file_search_input(&self) -> &TextInput {
        self.files.search_input()
    }

    #[cfg(test)]
    pub(crate) const fn file_list_scroll_state(&self) -> ScrollState {
        self.files.scroll_state()
    }

    pub(crate) fn set_search_visible(&mut self, visible: bool) {
        self.files.set_search_visible(visible);
    }

    pub(crate) fn apply_file_search(&mut self, command: TextInputCommand) {
        self.files.apply_search(command);
    }

    pub(crate) fn apply_file_search_composition(&mut self, event: TextInputCompositionEvent) {
        self.files.apply_search_composition(event);
    }

    pub(crate) fn cancel_file_search_composition(&mut self) {
        self.files.cancel_search_composition();
    }

    pub(crate) fn clear_file_search(&mut self) {
        self.files.clear_search();
    }

    pub(crate) fn selected_file_search_text(&self) -> Option<&str> {
        self.files.selected_search_text()
    }

    pub(crate) fn refresh_files(&mut self, entries: Vec<FsReadDirectoryEntry>) {
        self.files.refresh(directory_entries(entries));
    }

    pub(crate) fn replace_workspace(
        &mut self,
        context: &WorkspaceContext,
    ) -> Vec<zeta_editor::MultiDiffEditorItemIdentity> {
        self.files
            .set_workspace_root(context.working_directory().to_path_buf());
        self.scm.replace_diffs([]);
        self.sync_repository(context)
    }

    pub(crate) fn poll_file_search(&mut self) -> bool {
        self.files.poll_search()
    }

    pub(crate) const fn file_search_pending(&self) -> bool {
        self.files.search_pending()
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
        self.scm.replace_diffs(diffs)
    }

    pub(crate) fn scroll_multi_diff(&mut self, delta: f32, viewport: Size, now: Instant) -> bool {
        self.scm.editor_mut().scroll(delta, viewport, now)
    }

    pub(crate) fn scroll_file_list(&mut self, delta: f32, viewport: Size) -> bool {
        self.files.scroll(delta, viewport)
    }

    pub(crate) fn toggle_multi_diff_fold(&mut self, id: ElementId) -> bool {
        self.scm.editor_mut().toggle_fold_for_element(id)
    }

    pub(crate) fn move_multi_diff_scrollbar(
        &mut self,
        point: Point,
        bounds: Rect,
        now: Instant,
    ) -> ScrollbarPointerOutcome {
        self.scm
            .editor_mut()
            .scrollbar_pointer_moved(point, bounds, now)
    }

    pub(crate) fn press_multi_diff_scrollbar(
        &mut self,
        point: Point,
        bounds: Rect,
        now: Instant,
    ) -> ScrollbarPointerOutcome {
        self.scm.editor_mut().press_scrollbar(point, bounds, now)
    }

    pub(crate) fn release_multi_diff_scrollbar(
        &mut self,
        point: Point,
        bounds: Rect,
        now: Instant,
    ) -> ScrollbarPointerOutcome {
        self.scm.editor_mut().release_scrollbar(point, bounds, now)
    }

    pub(crate) fn leave_multi_diff_scrollbar(&mut self, now: Instant) -> bool {
        self.scm.editor_mut().scrollbar_pointer_left(now)
    }

    pub(crate) fn cancel_multi_diff_scrollbar(&mut self) {
        self.scm.editor_mut().cancel_scrollbar_interaction();
    }

    pub(crate) fn advance_multi_diff_scrollbar(&mut self, now: Instant) -> bool {
        self.scm.editor_mut().advance_scrollbar(now)
    }

    pub(crate) const fn multi_diff_scrollbar_deadline(&self) -> Option<Instant> {
        self.scm.editor().scrollbar_deadline()
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
