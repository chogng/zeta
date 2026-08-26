use zeta_ui::layout::PaneGroupLayout;
use zui::ui::GridNode;
use zui::ui::GridPane;
use zui::ui::Rect;
use zui::ui::SplitViewOrientation;
use zui::ui::SplitViewPane;
use zui::ui::SplitViewResize;

/// Stable identity for one content leaf inside a TabInput-owned PaneGroup.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct PaneId(u64);

impl PaneId {
    const ROOT: Self = Self(1);

    pub(crate) const fn value(self) -> u64 {
        self.0
    }
}

/// Stable identity for one owning split inside a PaneTree.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct PaneSplitId(u64);

impl PaneSplitId {
    pub(crate) const fn value(self) -> u64 {
        self.0
    }
}

/// Direction used when a PaneGroup creates a sibling Pane.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) enum PaneSplitDirection {
    /// Place the sibling beside the active Pane.
    Horizontal,
    /// Place the sibling above or below the active Pane.
    Vertical,
}

impl PaneSplitDirection {
    const fn orientation(self) -> SplitViewOrientation {
        match self {
            Self::Horizontal => SplitViewOrientation::Horizontal,
            Self::Vertical => SplitViewOrientation::Vertical,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum PaneNode {
    Leaf(PaneId),
    Split {
        id: PaneSplitId,
        direction: PaneSplitDirection,
        ratio: f32,
        first: Box<Self>,
        second: Box<Self>,
    },
}

/// Product-owned topology and focus state for one logical TabInput.
///
/// The group intentionally contains no UI element identity, renderer state, or terminal runtime.
/// A host binds each leaf to a content/runtime owner and projects this tree into geometry per frame.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PaneGroup {
    root: PaneNode,
    active: PaneId,
    next_pane_id: u64,
    next_split_id: u64,
}

impl Default for PaneGroup {
    fn default() -> Self {
        Self::new()
    }
}

impl PaneGroup {
    pub(crate) fn new() -> Self {
        Self {
            root: PaneNode::Leaf(PaneId::ROOT),
            active: PaneId::ROOT,
            next_pane_id: PaneId::ROOT.0 + 1,
            next_split_id: 1,
        }
    }

    pub(crate) const fn active_pane(&self) -> PaneId {
        self.active
    }

    pub(crate) const fn root_pane(&self) -> PaneId {
        PaneId::ROOT
    }

    pub(crate) fn activate(&mut self, pane: PaneId) -> bool {
        if !self.root.contains(pane) {
            return false;
        }
        self.active = pane;
        true
    }

    pub(crate) fn leaf_ids(&self) -> Vec<PaneId> {
        let mut leaves = Vec::new();
        self.root.collect_leaves(&mut leaves);
        leaves
    }

    pub(crate) fn split_active(&mut self, direction: PaneSplitDirection) -> PaneId {
        let old_active = self.active;
        let new_pane = self.allocate_pane_id();
        let split_id = self.allocate_split_id();
        let replaced = replace_leaf(
            &mut self.root,
            old_active,
            PaneNode::Split {
                id: split_id,
                direction,
                ratio: 0.5,
                first: Box::new(PaneNode::Leaf(old_active)),
                second: Box::new(PaneNode::Leaf(new_pane)),
            },
        );
        assert!(replaced, "active Pane must be present in its PaneGroup");
        self.active = new_pane;
        new_pane
    }

    pub(crate) fn close_active(&mut self) -> Option<PaneId> {
        let leaves_before = self.leaf_ids();
        if leaves_before.len() <= 1 {
            return None;
        }
        let removed = self.active;
        let removed_index = leaves_before
            .iter()
            .position(|id| *id == removed)
            .expect("active Pane must be present in its PaneGroup");
        let replaced = remove_leaf(&mut self.root, removed);
        assert_eq!(replaced, Some(removed), "active Pane must be removable");
        let leaves_after = self.leaf_ids();
        self.active = leaves_after[removed_index.min(leaves_after.len() - 1)];
        Some(removed)
    }

    pub(crate) fn focus_next(&mut self) -> PaneId {
        self.active = adjacent_leaf(&self.leaf_ids(), self.active, 1);
        self.active
    }

    pub(crate) fn focus_previous(&mut self) -> PaneId {
        self.active = adjacent_leaf(&self.leaf_ids(), self.active, -1);
        self.active
    }

    /// Applies the constrained result of one visible split Sash drag.
    pub(crate) fn resize_split(&mut self, split_id: PaneSplitId, resize: SplitViewResize) -> bool {
        let total = resize.previous_size() + resize.next_size();
        if !total.is_finite() || total <= 0.0 {
            return false;
        }
        let ratio = (resize.previous_size() / total).clamp(0.0, 1.0);
        set_split_ratio(&mut self.root, split_id, ratio)
    }

    /// Builds a backend-neutral recursive geometry input for the current tree.
    pub(crate) fn grid_node(&self, bounds: Rect) -> GridNode<PaneId, PaneSplitId> {
        self.root.grid_node(bounds)
    }

    /// Resolves the current PaneTree into leaf and sash geometry for one frame.
    pub(crate) fn layout(&self, bounds: Rect) -> PaneGroupLayout<PaneId, PaneSplitId> {
        PaneGroupLayout::new(bounds, &self.grid_node(bounds))
    }

    fn allocate_pane_id(&mut self) -> PaneId {
        let id = PaneId(self.next_pane_id);
        self.next_pane_id = self
            .next_pane_id
            .checked_add(1)
            .expect("Pane identity space exhausted");
        id
    }

    fn allocate_split_id(&mut self) -> PaneSplitId {
        let id = PaneSplitId(self.next_split_id);
        self.next_split_id = self
            .next_split_id
            .checked_add(1)
            .expect("Pane split identity space exhausted");
        id
    }
}

impl PaneNode {
    fn collect_leaves(&self, leaves: &mut Vec<PaneId>) {
        match self {
            Self::Leaf(id) => leaves.push(*id),
            Self::Split { first, second, .. } => {
                first.collect_leaves(leaves);
                second.collect_leaves(leaves);
            }
        }
    }

    fn contains(&self, target: PaneId) -> bool {
        match self {
            Self::Leaf(id) => *id == target,
            Self::Split { first, second, .. } => first.contains(target) || second.contains(target),
        }
    }

    fn grid_node(&self, bounds: Rect) -> GridNode<PaneId, PaneSplitId> {
        match self {
            Self::Leaf(id) => GridNode::leaf(*id),
            Self::Split {
                id,
                direction,
                ratio,
                first,
                second,
            } => {
                let orientation = direction.orientation();
                let primary = match orientation {
                    SplitViewOrientation::Horizontal => bounds.size.width,
                    SplitViewOrientation::Vertical => bounds.size.height,
                };
                let first_size = (primary * *ratio).clamp(0.0, primary.max(0.0));
                let second_size = (primary - first_size).max(0.0);
                let first_bounds = child_bounds(bounds, orientation, 0.0, first_size);
                let second_bounds = child_bounds(bounds, orientation, first_size, second_size);
                let minimum = 48.0;
                GridNode::split(
                    *id,
                    orientation,
                    vec![
                        GridPane::new(
                            first.grid_node(first_bounds),
                            SplitViewPane::new(first_size, minimum, f32::INFINITY),
                        ),
                        GridPane::new(
                            second.grid_node(second_bounds),
                            SplitViewPane::new(second_size, minimum, f32::INFINITY),
                        ),
                    ],
                )
            }
        }
    }
}

fn replace_leaf(node: &mut PaneNode, target: PaneId, replacement: PaneNode) -> bool {
    match node {
        PaneNode::Leaf(id) => {
            if *id != target {
                return false;
            }
            *node = replacement;
            true
        }
        PaneNode::Split { first, second, .. } => {
            replace_leaf(first, target, replacement.clone())
                || replace_leaf(second, target, replacement)
        }
    }
}

fn remove_leaf(node: &mut PaneNode, target: PaneId) -> Option<PaneId> {
    let sibling = match node {
        PaneNode::Split { first, second, .. } if matches!(first.as_ref(), PaneNode::Leaf(id) if *id == target) => {
            Some(std::mem::replace(
                second,
                Box::new(PaneNode::Leaf(PaneId::ROOT)),
            ))
        }
        PaneNode::Split { first, second, .. } if matches!(second.as_ref(), PaneNode::Leaf(id) if *id == target) => {
            Some(std::mem::replace(
                first,
                Box::new(PaneNode::Leaf(PaneId::ROOT)),
            ))
        }
        _ => None,
    };
    if let Some(sibling) = sibling {
        *node = *sibling;
        return Some(target);
    }

    match node {
        PaneNode::Leaf(_) => None,
        PaneNode::Split { first, second, .. } => {
            if first.contains(target) {
                remove_leaf(first, target)
            } else if second.contains(target) {
                remove_leaf(second, target)
            } else {
                None
            }
        }
    }
}

fn adjacent_leaf(leaves: &[PaneId], active: PaneId, delta: isize) -> PaneId {
    let Some(index) = leaves.iter().position(|id| *id == active) else {
        return leaves[0];
    };
    let len = leaves.len();
    let next = (index as isize + delta).rem_euclid(len as isize) as usize;
    leaves[next]
}

fn set_split_ratio(node: &mut PaneNode, split_id: PaneSplitId, ratio: f32) -> bool {
    match node {
        PaneNode::Leaf(_) => false,
        PaneNode::Split {
            id,
            ratio: current,
            first,
            second,
            ..
        } => {
            if *id == split_id {
                *current = ratio;
                true
            } else {
                set_split_ratio(first, split_id, ratio) || set_split_ratio(second, split_id, ratio)
            }
        }
    }
}

fn child_bounds(
    bounds: Rect,
    orientation: SplitViewOrientation,
    offset: f32,
    primary_size: f32,
) -> Rect {
    match orientation {
        SplitViewOrientation::Horizontal => Rect::from_xywh(
            bounds.origin.x + offset,
            bounds.origin.y,
            primary_size,
            bounds.size.height,
        ),
        SplitViewOrientation::Vertical => Rect::from_xywh(
            bounds.origin.x,
            bounds.origin.y + offset,
            bounds.size.width,
            primary_size,
        ),
    }
}

#[cfg(test)]
#[path = "pane_group_tests.rs"]
mod tests;
