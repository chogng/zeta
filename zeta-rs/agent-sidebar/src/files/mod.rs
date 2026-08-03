use std::collections::HashMap;
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
use zeta_ui::TreeItemExpansion;
use zeta_ui::VirtualListLayout;
use zui::ElementId;

use crate::AgentSidebarAction;

mod layout;
mod pane;
mod toolbar;

pub use layout::FILES_TOOLBAR_HEIGHT;
pub use layout::FilesLayout;
pub use pane::EXPLORER_PANE;
pub use pane::FilesPane;
pub use pane::FilesPaneStyle;
pub use toolbar::FilesToolbar;

pub const FILE_LIST_ROW_HEIGHT: f32 = 24.0;
const FILE_TREE_SCOPE: u32 = 5;

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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct NodeId(usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FilesEntry {
    id: NodeId,
    path: PathBuf,
    label: String,
    directory: bool,
    parent: Option<NodeId>,
    expanded: bool,
    children_loaded: bool,
    children: Vec<NodeId>,
}

impl FilesEntry {
    pub fn label(&self) -> &str {
        &self.label
    }
    pub const fn is_directory(&self) -> bool {
        self.directory
    }
    pub const fn is_expanded(&self) -> bool {
        self.expanded
    }
    pub fn element_id(&self) -> ElementId {
        tree_element_id(self.id)
    }
}

#[derive(Clone, Copy)]
pub struct FilesTreeRow<'a> {
    entry: &'a FilesEntry,
    depth: usize,
}

impl<'a> FilesTreeRow<'a> {
    pub const fn entry(self) -> &'a FilesEntry {
        self.entry
    }
    pub const fn depth(self) -> usize {
        self.depth
    }
}

#[derive(Default)]
struct FilesTree {
    nodes: Vec<FilesEntry>,
    roots: Vec<NodeId>,
    elements: HashMap<ElementId, NodeId>,
    visible_nodes: Vec<NodeId>,
    visible_items: Vec<TreeItem>,
    selected: Option<NodeId>,
}

impl FilesTree {
    fn clear(&mut self) {
        self.nodes.clear();
        self.roots.clear();
        self.elements.clear();
        self.visible_nodes.clear();
        self.visible_items.clear();
        self.selected = None;
    }

    fn replace_root(&mut self, entries: Vec<DirectoryEntry>) {
        self.clear();
        self.roots = self.allocate(entries_for(PathBuf::new(), entries), None);
        self.rebuild_visible();
    }

    fn visible_items(&self) -> &[TreeItem] {
        &self.visible_items
    }
    fn visible_len(&self) -> usize {
        self.visible_nodes.len()
    }

    fn row(&self, index: usize) -> Option<FilesTreeRow<'_>> {
        let node_id = *self.visible_nodes.get(index)?;
        Some(FilesTreeRow {
            entry: self.nodes.get(node_id.0)?,
            depth: self.visible_items.get(index)?.depth(),
        })
    }

    fn selected_element(&self) -> Option<ElementId> {
        self.selected.map(|node| tree_element_id(node))
    }

    fn activate(&mut self, element: ElementId) -> Option<AgentSidebarAction> {
        let node = self.elements.get(&element).copied()?;
        self.selected = Some(node);
        if !self.nodes[node.0].directory {
            return Some(AgentSidebarAction::OpenFile {
                path: self.nodes[node.0].path.clone(),
            });
        }
        if self.nodes[node.0].expanded {
            self.nodes[node.0].expanded = false;
        } else if self.nodes[node.0].children_loaded {
            self.nodes[node.0].expanded = true;
        } else {
            return Some(AgentSidebarAction::LoadChildren {
                element,
                path: self.nodes[node.0].path.clone(),
            });
        }
        self.rebuild_visible();
        Some(AgentSidebarAction::StateChanged)
    }

    fn navigate_right(&mut self, element: ElementId) -> Option<AgentSidebarAction> {
        let node = self.elements.get(&element).copied()?;
        if !self.nodes[node.0].directory {
            return Some(AgentSidebarAction::Handled);
        }
        if !self.nodes[node.0].expanded {
            if self.nodes[node.0].children_loaded {
                self.nodes[node.0].expanded = true;
                self.rebuild_visible();
                return Some(AgentSidebarAction::StateChanged);
            }
            return Some(AgentSidebarAction::LoadChildren {
                element,
                path: self.nodes[node.0].path.clone(),
            });
        }
        let Some(child) = self.nodes[node.0].children.first().copied() else {
            return Some(AgentSidebarAction::Handled);
        };
        self.selected = Some(child);
        Some(AgentSidebarAction::Focus(tree_element_id(child)))
    }

    fn navigate_left(&mut self, element: ElementId) -> Option<AgentSidebarAction> {
        let node = self.elements.get(&element).copied()?;
        if self.nodes[node.0].directory && self.nodes[node.0].expanded {
            self.nodes[node.0].expanded = false;
            self.rebuild_visible();
            return Some(AgentSidebarAction::StateChanged);
        }
        let Some(parent) = self.nodes[node.0].parent else {
            return Some(AgentSidebarAction::Handled);
        };
        self.selected = Some(parent);
        Some(AgentSidebarAction::Focus(tree_element_id(parent)))
    }

    fn complete_directory_load(
        &mut self,
        element: ElementId,
        entries: Vec<DirectoryEntry>,
    ) -> bool {
        let Some(node) = self.elements.get(&element).copied() else {
            return false;
        };
        if !self.nodes[node.0].directory || self.nodes[node.0].children_loaded {
            return false;
        }
        let children = self.allocate(
            entries_for(self.nodes[node.0].path.clone(), entries),
            Some(node),
        );
        let entry = &mut self.nodes[node.0];
        entry.children = children;
        entry.children_loaded = true;
        entry.expanded = true;
        self.rebuild_visible();
        true
    }

    fn allocate(&mut self, entries: Vec<EntrySpec>, parent: Option<NodeId>) -> Vec<NodeId> {
        entries
            .into_iter()
            .filter_map(|entry| {
                let index = self.nodes.len();
                let local = u32::try_from(index).ok()?.checked_add(1)?;
                let id = NodeId(index);
                self.nodes.push(FilesEntry {
                    id,
                    path: entry.path,
                    label: entry.label,
                    directory: entry.directory,
                    parent,
                    expanded: false,
                    children_loaded: false,
                    children: Vec::new(),
                });
                self.elements
                    .insert(ElementId::scoped(FILE_TREE_SCOPE, local), id);
                Some(id)
            })
            .collect()
    }

    fn rebuild_visible(&mut self) {
        let mut nodes = Vec::new();
        let mut items = Vec::new();
        append_visible(&self.nodes, &self.roots, 0, &mut nodes, &mut items);
        self.visible_nodes = nodes;
        self.visible_items = items;
    }
}

fn append_visible(
    nodes: &[FilesEntry],
    ids: &[NodeId],
    depth: usize,
    visible_nodes: &mut Vec<NodeId>,
    visible_items: &mut Vec<TreeItem>,
) {
    for id in ids {
        let Some(node) = nodes.get(id.0) else {
            continue;
        };
        visible_nodes.push(*id);
        visible_items.push(TreeItem::new(
            depth,
            if !node.directory {
                TreeItemExpansion::Leaf
            } else if node.expanded {
                TreeItemExpansion::Expanded
            } else {
                TreeItemExpansion::Collapsed
            },
        ));
        if node.expanded {
            append_visible(
                nodes,
                &node.children,
                depth.saturating_add(1),
                visible_nodes,
                visible_items,
            );
        }
    }
}

struct EntrySpec {
    path: PathBuf,
    label: String,
    directory: bool,
}

fn entries_for(parent: PathBuf, entries: Vec<DirectoryEntry>) -> Vec<EntrySpec> {
    let mut result = entries
        .into_iter()
        .map(|entry| EntrySpec {
            path: parent.join(&entry.name),
            label: entry.name,
            directory: entry.kind == DirectoryEntryKind::Directory,
        })
        .collect::<Vec<_>>();
    result.sort_by(|left, right| {
        right
            .directory
            .cmp(&left.directory)
            .then_with(|| left.label.cmp(&right.label))
    });
    result
}

fn tree_element_id(id: NodeId) -> ElementId {
    ElementId::scoped(
        FILE_TREE_SCOPE,
        u32::try_from(id.0).unwrap_or(u32::MAX).saturating_add(1),
    )
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

#[cfg(test)]
#[path = "files_tests.rs"]
mod tests;
