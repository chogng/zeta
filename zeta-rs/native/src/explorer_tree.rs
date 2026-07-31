use std::collections::HashMap;
use std::path::{Path, PathBuf};

use zeta_ui::{TreeItem, TreeItemExpansion};
use zeta_ui_dispatch::ElementId;

const FILE_TREE_SCOPE: u32 = 5;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
struct ExplorerNodeId(usize);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExplorerEntry {
    id: ExplorerNodeId,
    path: PathBuf,
    label: String,
    directory: bool,
    parent: Option<ExplorerNodeId>,
    expanded: bool,
    children_loaded: bool,
    children: Vec<ExplorerNodeId>,
}

impl ExplorerEntry {
    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    pub(crate) const fn is_directory(&self) -> bool {
        self.directory
    }

    pub(crate) const fn is_expanded(&self) -> bool {
        self.expanded
    }

    pub(crate) fn element_id(&self) -> ElementId {
        file_tree_element_id(self.id)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct ExplorerTreeRow<'a> {
    entry: &'a ExplorerEntry,
    depth: usize,
}

impl<'a> ExplorerTreeRow<'a> {
    pub(crate) const fn entry(self) -> &'a ExplorerEntry {
        self.entry
    }

    pub(crate) const fn depth(self) -> usize {
        self.depth
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExplorerTreeNavigation {
    Handled,
    StateChanged,
    Focus(ElementId),
}

#[derive(Default)]
pub(crate) struct ExplorerTree {
    nodes: Vec<ExplorerEntry>,
    roots: Vec<ExplorerNodeId>,
    elements: HashMap<ElementId, ExplorerNodeId>,
    visible_nodes: Vec<ExplorerNodeId>,
    visible_items: Vec<TreeItem>,
}

impl ExplorerTree {
    pub(crate) fn replace_root(&mut self, root: Option<&Path>) {
        self.nodes.clear();
        self.roots.clear();
        self.elements.clear();
        self.visible_nodes.clear();
        self.visible_items.clear();
        let Some(root) = root else {
            return;
        };
        let entries = read_entries(root);
        self.roots = self.allocate_entries(entries, None);
        self.rebuild_visible();
    }

    #[cfg(test)]
    pub(crate) fn root_entries(&self) -> Vec<&ExplorerEntry> {
        self.roots
            .iter()
            .filter_map(|id| self.nodes.get(id.0))
            .collect()
    }

    pub(crate) fn visible_items(&self) -> &[TreeItem] {
        &self.visible_items
    }

    pub(crate) fn visible_len(&self) -> usize {
        self.visible_nodes.len()
    }

    pub(crate) fn row(&self, index: usize) -> Option<ExplorerTreeRow<'_>> {
        let node_id = *self.visible_nodes.get(index)?;
        let entry = self.nodes.get(node_id.0)?;
        Some(ExplorerTreeRow {
            entry,
            depth: self.visible_items.get(index)?.depth(),
        })
    }

    pub(crate) fn activate_element(&mut self, element: ElementId) -> bool {
        let Some(node_id) = self.elements.get(&element).copied() else {
            return false;
        };
        if !self.nodes[node_id.0].directory {
            return true;
        }
        if self.nodes[node_id.0].expanded {
            self.nodes[node_id.0].expanded = false;
        } else {
            self.load_children(node_id);
            self.nodes[node_id.0].expanded = true;
        }
        self.rebuild_visible();
        true
    }

    pub(crate) fn navigate_right(&mut self, element: ElementId) -> Option<ExplorerTreeNavigation> {
        let node_id = self.elements.get(&element).copied()?;
        if !self.nodes[node_id.0].directory {
            return Some(ExplorerTreeNavigation::Handled);
        }
        if !self.nodes[node_id.0].expanded {
            self.load_children(node_id);
            self.nodes[node_id.0].expanded = true;
            self.rebuild_visible();
            return Some(ExplorerTreeNavigation::StateChanged);
        }
        Some(
            self.nodes[node_id.0]
                .children
                .first()
                .map(|child| ExplorerTreeNavigation::Focus(file_tree_element_id(*child)))
                .unwrap_or(ExplorerTreeNavigation::Handled),
        )
    }

    pub(crate) fn navigate_left(&mut self, element: ElementId) -> Option<ExplorerTreeNavigation> {
        let node_id = self.elements.get(&element).copied()?;
        if self.nodes[node_id.0].directory && self.nodes[node_id.0].expanded {
            self.nodes[node_id.0].expanded = false;
            self.rebuild_visible();
            return Some(ExplorerTreeNavigation::StateChanged);
        }
        Some(
            self.nodes[node_id.0]
                .parent
                .map(|parent| ExplorerTreeNavigation::Focus(file_tree_element_id(parent)))
                .unwrap_or(ExplorerTreeNavigation::Handled),
        )
    }

    fn load_children(&mut self, node_id: ExplorerNodeId) {
        if self.nodes[node_id.0].children_loaded {
            return;
        }
        let path = self.nodes[node_id.0].path.clone();
        let children = self.allocate_entries(read_entries(&path), Some(node_id));
        let node = &mut self.nodes[node_id.0];
        node.children = children;
        node.children_loaded = true;
    }

    fn allocate_entries(
        &mut self,
        entries: Vec<ExplorerEntrySpec>,
        parent: Option<ExplorerNodeId>,
    ) -> Vec<ExplorerNodeId> {
        entries
            .into_iter()
            .filter_map(|entry| {
                let index = self.nodes.len();
                let local = u32::try_from(index).ok()?.checked_add(1)?;
                let id = ExplorerNodeId(index);
                let element = ElementId::scoped(FILE_TREE_SCOPE, local);
                self.nodes.push(ExplorerEntry {
                    id,
                    path: entry.path,
                    label: entry.label,
                    directory: entry.directory,
                    parent,
                    expanded: false,
                    children_loaded: false,
                    children: Vec::new(),
                });
                self.elements.insert(element, id);
                Some(id)
            })
            .collect()
    }

    fn rebuild_visible(&mut self) {
        let mut visible_nodes = Vec::new();
        let mut visible_items = Vec::new();
        append_visible(
            &self.nodes,
            &self.roots,
            0,
            &mut visible_nodes,
            &mut visible_items,
        );
        self.visible_nodes = visible_nodes;
        self.visible_items = visible_items;
    }
}

fn append_visible(
    nodes: &[ExplorerEntry],
    node_ids: &[ExplorerNodeId],
    depth: usize,
    visible_nodes: &mut Vec<ExplorerNodeId>,
    visible_items: &mut Vec<TreeItem>,
) {
    for &node_id in node_ids {
        let Some(node) = nodes.get(node_id.0) else {
            continue;
        };
        visible_nodes.push(node_id);
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExplorerEntrySpec {
    path: PathBuf,
    label: String,
    directory: bool,
}

fn read_entries(directory: &Path) -> Vec<ExplorerEntrySpec> {
    let mut entries = std::fs::read_dir(directory)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let file_type = entry.file_type().ok()?;
            let label = entry.file_name().to_string_lossy().into_owned();
            (!ignored_name(&label)).then_some(ExplorerEntrySpec {
                path: entry.path(),
                label,
                directory: file_type.is_dir(),
            })
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

fn ignored_name(label: &str) -> bool {
    matches!(label, ".git" | ".zeta" | "node_modules" | "target")
}

fn file_tree_element_id(id: ExplorerNodeId) -> ElementId {
    ElementId::scoped(
        FILE_TREE_SCOPE,
        u32::try_from(id.0).unwrap_or(u32::MAX).saturating_add(1),
    )
}

#[cfg(test)]
#[path = "explorer_tree_tests.rs"]
mod tests;
