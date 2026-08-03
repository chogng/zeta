use std::time::Instant;

use zeta_agent_sidebar::AgentSidebar;
use zeta_agent_sidebar::AgentSidebarAction;
use zeta_agent_sidebar::DirectoryEntry;
use zeta_agent_sidebar::EditorPaneState;
use zeta_agent_sidebar::FilesState;
use zeta_agent_sidebar::ScmDiff;
use zeta_app_server_protocol::protocol::fs::FsReadDirectoryEntry;
use zeta_editor::MultiDiffEditorStyle;
use zeta_ui::Point;
use zeta_ui::Rect;
use zeta_ui::Size;
use zeta_ui::TextInput;
use zeta_ui::TextInputCommand;
use zeta_ui::TextInputCompositionEvent;
use zui::ElementId;

use crate::workspace_context::WorkspaceContext;
use zeta_agent_sidebar::ScrollbarPointerOutcome;

#[cfg(test)]
use zeta_agent_sidebar::FilesTreeRow;
#[cfg(test)]
use zeta_ui::ScrollState;

pub(crate) use zeta_agent_sidebar::AgentSidebarView;

pub(crate) struct AgentSidebarWorkspace {
    sidebar: AgentSidebar,
}

impl Default for AgentSidebarWorkspace {
    fn default() -> Self {
        Self {
            sidebar: AgentSidebar::default(),
        }
    }
}

impl AgentSidebarWorkspace {
    pub(crate) fn new(context: &WorkspaceContext) -> Self {
        let mut workspace = Self {
            sidebar: AgentSidebar::new(context.working_directory().to_path_buf()),
            ..Self::default()
        };
        let _ = workspace.sync_repository(context);
        workspace
    }

    pub(crate) const fn active_view(&self) -> AgentSidebarView {
        self.sidebar.active_view()
    }

    pub(crate) fn select_view(&mut self, view: AgentSidebarView) {
        self.sidebar.select_view(view);
    }

    pub(crate) const fn editor(&self) -> &EditorPaneState {
        self.sidebar.scm().editor()
    }

    pub(crate) const fn files(&self) -> &FilesState {
        self.sidebar.files()
    }

    pub(crate) fn set_editor_style(&mut self, style: MultiDiffEditorStyle) {
        self.sidebar.scm_mut().editor_mut().set_style(style);
    }

    #[cfg(test)]
    pub(crate) fn file_tree_row(&self, index: usize) -> Option<FilesTreeRow<'_>> {
        self.sidebar.files().tree_row(index)
    }

    pub(crate) fn activate_file_tree_element(
        &mut self,
        element: ElementId,
    ) -> Option<AgentSidebarAction> {
        self.sidebar.files_mut().activate(element)
    }

    pub(crate) fn navigate_file_tree_right(
        &mut self,
        element: ElementId,
    ) -> Option<AgentSidebarAction> {
        self.sidebar.files_mut().navigate_right(element)
    }

    pub(crate) fn navigate_file_tree_left(
        &mut self,
        element: ElementId,
    ) -> Option<AgentSidebarAction> {
        self.sidebar.files_mut().navigate_left(element)
    }

    pub(crate) fn complete_file_tree_directory_load(
        &mut self,
        element: ElementId,
        entries: Vec<FsReadDirectoryEntry>,
    ) -> bool {
        self.sidebar
            .files_mut()
            .complete_directory_load(element, directory_entries(entries))
    }

    pub(crate) const fn search_visible(&self) -> bool {
        self.sidebar.files().search_visible()
    }

    pub(crate) const fn file_search_input(&self) -> &TextInput {
        self.sidebar.files().search_input()
    }

    #[cfg(test)]
    pub(crate) const fn file_list_scroll_state(&self) -> ScrollState {
        self.sidebar.files().scroll_state()
    }

    pub(crate) fn set_search_visible(&mut self, visible: bool) {
        self.sidebar.files_mut().set_search_visible(visible);
    }

    pub(crate) fn apply_file_search(&mut self, command: TextInputCommand) {
        self.sidebar.files_mut().apply_search(command);
    }

    pub(crate) fn apply_file_search_composition(&mut self, event: TextInputCompositionEvent) {
        self.sidebar.files_mut().apply_search_composition(event);
    }

    pub(crate) fn cancel_file_search_composition(&mut self) {
        self.sidebar.files_mut().cancel_search_composition();
    }

    pub(crate) fn clear_file_search(&mut self) {
        self.sidebar.files_mut().clear_search();
    }

    pub(crate) fn selected_file_search_text(&self) -> Option<&str> {
        self.sidebar.files().selected_search_text()
    }

    pub(crate) fn refresh_files(&mut self, entries: Vec<FsReadDirectoryEntry>) {
        self.sidebar.files_mut().refresh(directory_entries(entries));
    }

    pub(crate) fn replace_workspace(
        &mut self,
        context: &WorkspaceContext,
    ) -> Vec<zeta_editor::MultiDiffEditorItemIdentity> {
        self.sidebar
            .replace_workspace(context.working_directory().to_path_buf());
        self.sync_repository(context)
    }

    pub(crate) fn poll_file_search(&mut self) -> bool {
        self.sidebar.files_mut().poll_search()
    }

    pub(crate) const fn file_search_pending(&self) -> bool {
        self.sidebar.files().search_pending()
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
        self.sidebar.scm_mut().replace_diffs(diffs)
    }

    pub(crate) fn scroll_multi_diff(&mut self, delta: f32, viewport: Size, now: Instant) -> bool {
        self.sidebar
            .scm_mut()
            .editor_mut()
            .scroll(delta, viewport, now)
    }

    pub(crate) fn scroll_file_list(&mut self, delta: f32, viewport: Size) -> bool {
        self.sidebar.files_mut().scroll(delta, viewport)
    }

    pub(crate) fn toggle_multi_diff_fold(&mut self, id: ElementId) -> bool {
        self.sidebar
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
        self.sidebar
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
        self.sidebar
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
        self.sidebar
            .scm_mut()
            .editor_mut()
            .release_scrollbar(point, bounds, now)
    }

    pub(crate) fn leave_multi_diff_scrollbar(&mut self, now: Instant) -> bool {
        self.sidebar
            .scm_mut()
            .editor_mut()
            .scrollbar_pointer_left(now)
    }

    pub(crate) fn cancel_multi_diff_scrollbar(&mut self) {
        self.sidebar
            .scm_mut()
            .editor_mut()
            .cancel_scrollbar_interaction();
    }

    pub(crate) fn advance_multi_diff_scrollbar(&mut self, now: Instant) -> bool {
        self.sidebar.scm_mut().editor_mut().advance_scrollbar(now)
    }

    pub(crate) const fn multi_diff_scrollbar_deadline(&self) -> Option<Instant> {
        self.sidebar.scm().editor().scrollbar_deadline()
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
#[path = "agent_sidebar_workspace_tests.rs"]
mod tests;
