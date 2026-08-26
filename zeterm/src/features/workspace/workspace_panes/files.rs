use std::path::PathBuf;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;

use zeta_file_search::PathSearchHandle;
use zeta_file_search::PathSearchOptions;
use zeta_file_search::PathSearchSnapshot;
use zeta_ui::ScrollAxis;
use zeta_ui::ScrollCommand;
use zeta_ui::ScrollDelta;
use zeta_ui::ScrollMetrics;
use zeta_ui::ScrollState;
use zeta_ui::Size;
use zeta_ui::TextInput;
use zeta_ui::TextInputCommand;
use zeta_ui::TextInputCompositionEvent;
use zeta_ui::TreeItem;
use zeta_ui::VirtualListLayout;
use zui::ui::ElementId;

use crate::workspace_panes::AgentSidebarAction;

#[path = "files/file_icon.rs"]
mod file_icon;
#[path = "files/file_tree.rs"]
mod file_tree;
#[path = "files/layout.rs"]
mod layout;
#[path = "files/pane.rs"]
mod pane;
#[path = "files/toolbar.rs"]
mod toolbar;
#[path = "files/tree_view.rs"]
mod tree_view;

use file_tree::FilesTree;
pub use file_tree::FilesTreeRow;
pub use layout::FilesLayout;
pub use pane::FilesPane;
pub use pane::FilesPaneStyle;
pub use toolbar::FilesToolbar;

pub const FILE_LIST_ROW_HEIGHT: f32 = 24.0;

/// A filesystem entry projected by the host into the Files pane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectoryEntry {
    name: String,
    kind: DirectoryEntryKind,
}

impl DirectoryEntry {
    pub fn file(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: DirectoryEntryKind::File,
        }
    }

    pub fn directory(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: DirectoryEntryKind::Directory,
        }
    }
}

/// The tree-relevant classification of a directory entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DirectoryEntryKind {
    File,
    Directory,
}

/// Retained Files-pane state. The host supplies snapshots and executes actions.
pub struct FilesState {
    root: Option<PathBuf>,
    tree: FilesTree,
    search_input: TextInput,
    search_visible: bool,
    search_handle: Option<PathSearchHandle>,
    search_receiver: Option<Receiver<PathSearchSnapshot>>,
    search_revision: u64,
    search_matches: Vec<PathBuf>,
    search_pending: bool,
    scroll_state: ScrollState,
}

impl Default for FilesState {
    fn default() -> Self {
        Self {
            root: None,
            tree: FilesTree::default(),
            search_input: TextInput::new(),
            search_visible: false,
            search_handle: None,
            search_receiver: None,
            search_revision: 0,
            search_matches: Vec::new(),
            search_pending: false,
            scroll_state: ScrollState::default(),
        }
    }
}

impl FilesState {
    pub fn set_workspace_root(&mut self, root: PathBuf) {
        self.root = Some(root);
        self.tree.clear();
        self.scroll_state = ScrollState::default();
    }
    pub fn refresh(&mut self, entries: Vec<DirectoryEntry>) {
        self.scroll_state = ScrollState::default();
        self.tree.replace_root(entries);
        let Some(root) = self.root.clone() else {
            return;
        };
        match PathSearchHandle::start(root, PathSearchOptions::default()) {
            Ok((handle, receiver)) => {
                self.search_revision = handle.update_query(self.search_input.text());
                self.search_handle = Some(handle);
                self.search_receiver = Some(receiver);
                self.search_matches.clear();
                self.search_pending = true;
            }
            Err(_) => {
                self.search_handle = None;
                self.search_receiver = None;
                self.search_matches.clear();
                self.search_pending = false;
            }
        }
    }
    pub fn complete_directory_load(
        &mut self,
        element: ElementId,
        entries: Vec<DirectoryEntry>,
    ) -> bool {
        self.tree.complete_directory_load(element, entries)
    }
    pub const fn search_visible(&self) -> bool {
        self.search_visible
    }
    pub const fn search_input(&self) -> &TextInput {
        &self.search_input
    }
    pub fn search_matches(&self) -> &[PathBuf] {
        &self.search_matches
    }
    pub const fn scroll_state(&self) -> ScrollState {
        self.scroll_state
    }
    pub fn item_count(&self) -> usize {
        if self.search_visible && !self.search_input.text().trim().is_empty() {
            self.search_matches.len()
        } else {
            self.tree.visible_len()
        }
    }
    pub fn tree_items(&self) -> &[TreeItem] {
        self.tree.visible_items()
    }
    pub fn tree_row(&self, index: usize) -> Option<FilesTreeRow<'_>> {
        self.tree.row(index)
    }
    pub fn selected_element(&self) -> Option<ElementId> {
        self.tree.selected_element()
    }
    pub fn activate(&mut self, element: ElementId) -> Option<AgentSidebarAction> {
        self.tree.activate(element)
    }
    pub fn navigate_right(&mut self, element: ElementId) -> Option<AgentSidebarAction> {
        self.tree.navigate_right(element)
    }
    pub fn navigate_left(&mut self, element: ElementId) -> Option<AgentSidebarAction> {
        self.tree.navigate_left(element)
    }
    pub fn set_search_visible(&mut self, visible: bool) {
        self.search_visible = visible;
        self.scroll_state = ScrollState::default();
        if !visible {
            self.search_input.take_text();
            self.update_search();
        }
    }
    pub fn apply_search(&mut self, command: TextInputCommand) {
        self.search_input.apply(command);
        self.scroll_state = ScrollState::default();
        self.update_search();
    }
    pub fn apply_search_composition(&mut self, event: TextInputCompositionEvent) {
        self.search_input.apply_composition(event);
        self.scroll_state = ScrollState::default();
        self.update_search();
    }
    pub fn cancel_search_composition(&mut self) {
        self.search_input.cancel_composition();
    }
    pub fn clear_search(&mut self) {
        self.search_input.take_text();
        self.scroll_state = ScrollState::default();
        self.update_search();
    }
    pub fn selected_search_text(&self) -> Option<&str> {
        self.search_input.selected_text()
    }
    pub fn poll_search(&mut self) -> bool {
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
            || snapshot.query != self.search_input.text()
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
    pub const fn search_pending(&self) -> bool {
        self.search_pending
    }
    pub fn scroll(&mut self, delta: f32, viewport: Size) -> bool {
        let content = VirtualListLayout::new(self.item_count(), FILE_LIST_ROW_HEIGHT)
            .content_size(viewport.width);
        self.scroll_state.apply(
            ScrollCommand::ByPixels(ScrollDelta::vertical(delta)),
            ScrollMetrics::new(viewport, content),
            ScrollAxis::Vertical,
        )
    }
    fn update_search(&mut self) {
        if let Some(handle) = self.search_handle.as_ref() {
            self.search_revision = handle.update_query(self.search_input.text());
            self.search_pending = true;
        }
    }
}
