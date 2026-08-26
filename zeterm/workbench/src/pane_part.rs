use std::collections::HashMap;

use zeta_ui::layout::PaneGroupLayout;
use zui::ui::GridNode;
use zui::ui::GridPane;
use zui::ui::Rect;
use zui::ui::SplitViewOrientation;
use zui::ui::SplitViewPane;
use zui::ui::SplitViewResize;

use crate::PaneGroup;
use crate::PaneInput;
use crate::PaneInputId;

/// Stable identity for one rectangular group in a [`PanePart`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PaneGroupId(u64);

impl PaneGroupId {
    pub const ROOT: Self = Self(1);

    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Backwards-compatible name for the identity of one visible pane region.
pub type PaneId = PaneGroupId;

/// Stable identity for one owning split inside a [`PanePart`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PaneSplitId(u64);

impl PaneSplitId {
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Direction used when a [`PanePart`] creates a sibling group.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PaneSplitDirection {
    /// Place the sibling beside the active group.
    Horizontal,
    /// Place the sibling above or below the active group.
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

/// A logical input mounted in one visible workbench group.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Pane {
    group_id: PaneGroupId,
    input_id: PaneInputId,
    input: PaneInput,
}

impl Pane {
    fn new(group_id: PaneGroupId, input_id: PaneInputId, input: PaneInput) -> Self {
        Self {
            group_id,
            input_id,
            input,
        }
    }

    /// Returns the stable layout identity of the group showing this input.
    pub const fn id(&self) -> PaneGroupId {
        self.group_id
    }

    /// Returns the stable logical identity of this input.
    pub const fn input_id(&self) -> PaneInputId {
        self.input_id
    }

    /// Returns the content description mounted in this group.
    pub fn input(&self) -> &PaneInput {
        &self.input
    }
}

#[derive(Clone, Debug, PartialEq)]
enum PaneNode {
    Leaf(PaneGroupId),
    Split {
        id: PaneSplitId,
        direction: PaneSplitDirection,
        ratio: f32,
        first: Box<Self>,
        second: Box<Self>,
    },
}

/// Pane Part containing one [`PaneGroup`] per visible rectangle.
///
/// This is the VS Code `EditorPart`-like layer: it owns the grid topology and group selection,
/// while each leaf group owns its logical inputs. It has no renderer nodes or feature runtimes.
#[derive(Clone, Debug, PartialEq)]
pub struct PanePart {
    root: PaneNode,
    groups: HashMap<PaneGroupId, PaneGroup>,
    active: PaneGroupId,
    next_group_id: u64,
    next_split_id: u64,
    workspace_return: Option<PaneInput>,
}

impl Default for PanePart {
    fn default() -> Self {
        Self::new()
    }
}

impl PanePart {
    /// Creates a Pane Part with one empty root group.
    pub fn new() -> Self {
        let root = PaneGroupId::ROOT;
        let mut groups = HashMap::new();
        groups.insert(root, PaneGroup::new());
        Self {
            root: PaneNode::Leaf(root),
            groups,
            active: root,
            next_group_id: root.0 + 1,
            next_split_id: 1,
            workspace_return: None,
        }
    }

    /// Creates a layout with one active root input.
    pub fn with_input(input: PaneInput) -> Self {
        let mut layout = Self::new();
        layout.mount_input(PaneGroupId::ROOT, input);
        layout
    }

    /// Returns the active group identity.
    pub const fn active_group(&self) -> PaneGroupId {
        self.active
    }

    /// Compatibility name for callers that refer to visible regions as panes.
    pub const fn active_pane(&self) -> PaneId {
        self.active
    }

    /// Returns the root group identity.
    pub const fn root_group(&self) -> PaneGroupId {
        PaneGroupId::ROOT
    }

    /// Compatibility name for callers that refer to visible regions as panes.
    pub const fn root_pane(&self) -> PaneId {
        PaneGroupId::ROOT
    }

    /// Returns one group by identity.
    pub fn group(&self, id: PaneGroupId) -> Option<&PaneGroup> {
        self.groups.get(&id)
    }

    /// Returns one mutable group by identity.
    pub(crate) fn group_mut(&mut self, id: PaneGroupId) -> Option<&mut PaneGroup> {
        self.groups.get_mut(&id)
    }

    /// Returns the IDs of visible groups in visual tree order.
    pub fn group_ids(&self) -> Vec<PaneGroupId> {
        let mut groups = Vec::new();
        self.root.collect_leaves(&mut groups);
        groups
    }

    /// Compatibility name for callers that refer to visible regions as panes.
    pub fn leaf_ids(&self) -> Vec<PaneId> {
        self.group_ids()
    }

    /// Returns the active input in one visible group.
    pub fn active_input(&self, group_id: PaneGroupId) -> Option<&PaneInput> {
        self.group(group_id)?.active_input()
    }

    /// Returns the active input identity in one visible group.
    pub fn active_input_id(&self, group_id: PaneGroupId) -> Option<PaneInputId> {
        self.group(group_id)?.active_input_id()
    }

    /// Returns the active input in one visible group.
    pub fn pane_input(&self, group_id: PaneGroupId) -> Option<&PaneInput> {
        self.active_input(group_id)
    }

    /// Returns one visible group and its active logical input.
    pub fn pane(&self, group_id: PaneGroupId) -> Option<Pane> {
        let group = self.group(group_id)?;
        let input_id = group.active_input_id()?;
        let input = group.input(input_id)?.clone();
        Some(Pane::new(group_id, input_id, input))
    }

    /// Returns all visible groups with an active logical input in visual tree order.
    pub fn panes(&self) -> Vec<Pane> {
        self.group_ids()
            .into_iter()
            .filter_map(|group_id| self.pane(group_id))
            .collect()
    }

    /// Mounts an input into an existing group, preserving the active input identity when present.
    pub fn mount_input(&mut self, group_id: PaneGroupId, input: PaneInput) -> Option<PaneInput> {
        let group = self.group_mut(group_id)?;
        if group.active_input_id().is_some() {
            group.replace_active_input(input)
        } else {
            group.open_input(input);
            None
        }
    }

    /// Opens a second input in an existing group and makes it active.
    pub fn open_input(&mut self, group_id: PaneGroupId, input: PaneInput) -> Option<PaneInputId> {
        self.group_mut(group_id)
            .map(|group| group.open_input(input))
    }

    /// Replaces a particular input in an existing group.
    pub fn replace_input(
        &mut self,
        group_id: PaneGroupId,
        input_id: PaneInputId,
        input: PaneInput,
    ) -> Option<PaneInput> {
        self.group_mut(group_id)?.replace_input(input_id, input)
    }

    /// Activates an input in a visible group by stable identity.
    pub fn activate_input(&mut self, group_id: PaneGroupId, input_id: PaneInputId) -> bool {
        self.group_mut(group_id)
            .is_some_and(|group| group.activate_input(input_id))
    }

    /// Closes an input in a visible group and returns its logical state.
    pub fn close_input(&mut self, group_id: PaneGroupId, input_id: PaneInputId) -> Option<Pane> {
        let input = self.group_mut(group_id)?.close_input(input_id)?;
        Some(Pane::new(group_id, input_id, input))
    }

    /// Activates a visible group.
    pub fn activate(&mut self, group_id: PaneGroupId) -> bool {
        if !self.root.contains(group_id) {
            return false;
        }
        self.active = group_id;
        true
    }

    /// Creates an empty sibling group beside the active group.
    pub fn split_active(&mut self, direction: PaneSplitDirection) -> PaneGroupId {
        self.split_active_with_input(direction, None).0
    }

    /// Creates a sibling group and optionally mounts its first input.
    pub fn split_active_with_input(
        &mut self,
        direction: PaneSplitDirection,
        input: Option<PaneInput>,
    ) -> (PaneGroupId, Option<PaneInputId>) {
        let old_active = self.active;
        let new_group = self.allocate_group_id();
        let split_id = self.allocate_split_id();
        let replaced = replace_leaf(
            &mut self.root,
            old_active,
            PaneNode::Split {
                id: split_id,
                direction,
                ratio: 0.5,
                first: Box::new(PaneNode::Leaf(old_active)),
                second: Box::new(PaneNode::Leaf(new_group)),
            },
        );
        assert!(replaced, "active PaneGroup must be present in its PanePart");
        let mut group = PaneGroup::new();
        let input_id = input.map(|input| group.open_input(input));
        self.groups.insert(new_group, group);
        self.active = new_group;
        (new_group, input_id)
    }

    /// Closes the active group and returns its complete logical state.
    pub fn close_active(&mut self) -> Option<(PaneGroupId, PaneGroup)> {
        if self.group_ids().len() <= 1 {
            return None;
        }
        let leaves_before = self.group_ids();
        let removed = self.active;
        let removed_index = leaves_before
            .iter()
            .position(|id| *id == removed)
            .expect("active PaneGroup must be present in its PanePart");
        let replaced = remove_leaf(&mut self.root, removed);
        assert_eq!(
            replaced,
            Some(removed),
            "active PaneGroup must be removable"
        );
        let leaves_after = self.group_ids();
        self.active = leaves_after[removed_index.min(leaves_after.len() - 1)];
        let group = self
            .groups
            .remove(&removed)
            .expect("every PanePart leaf must have a PaneGroup");
        Some((removed, group))
    }

    /// Activates a visible group and closes it.
    pub fn close_group(&mut self, group_id: PaneGroupId) -> Option<(PaneGroupId, PaneGroup)> {
        if !self.activate(group_id) {
            return None;
        }
        self.close_active()
    }

    /// Saves a logical input to restore when this Pane Part returns from Files or Diff.
    pub fn remember_workspace_return(&mut self, input: PaneInput) {
        self.workspace_return = Some(input);
    }

    /// Takes the logical input saved for this Pane Part, if any.
    pub fn take_workspace_return(&mut self) -> Option<PaneInput> {
        self.workspace_return.take()
    }

    /// Drops the saved workspace return for this Pane Part.
    pub fn clear_workspace_return(&mut self) {
        self.workspace_return = None;
    }

    /// Destroys the active group and returns every input it owned.
    pub fn destroy_active_panes(&mut self) -> Option<Vec<Pane>> {
        let (removed_group_id, group) = self.close_active()?;
        Some(
            group
                .take_inputs()
                .into_iter()
                .map(|(input_id, input)| Pane::new(removed_group_id, input_id, input))
                .collect(),
        )
    }

    /// Destroys a specific group after making it active and returns every input it owned.
    pub fn destroy_panes(&mut self, group_id: PaneGroupId) -> Option<Vec<Pane>> {
        let (removed_group_id, group) = self.close_group(group_id)?;
        Some(
            group
                .take_inputs()
                .into_iter()
                .map(|(input_id, input)| Pane::new(removed_group_id, input_id, input))
                .collect(),
        )
    }

    /// Cycles focus through visible groups in visual tree order.
    pub fn focus_next(&mut self) -> PaneGroupId {
        self.active = adjacent_leaf(&self.group_ids(), self.active, 1);
        self.active
    }

    /// Cycles focus backwards through visible groups in visual tree order.
    pub fn focus_previous(&mut self) -> PaneGroupId {
        self.active = adjacent_leaf(&self.group_ids(), self.active, -1);
        self.active
    }

    /// Applies the constrained result of one visible split sash drag.
    pub fn resize_split(&mut self, split_id: PaneSplitId, resize: SplitViewResize) -> bool {
        let total = resize.previous_size() + resize.next_size();
        if !total.is_finite() || total <= 0.0 {
            return false;
        }
        let ratio = (resize.previous_size() / total).clamp(0.0, 1.0);
        set_split_ratio(&mut self.root, split_id, ratio)
    }

    /// Builds a backend-neutral recursive geometry input for the current layout.
    pub fn grid_node(&self, bounds: Rect) -> GridNode<PaneGroupId, PaneSplitId> {
        self.root.grid_node(bounds)
    }

    /// Resolves the current layout into leaf and sash geometry for one frame.
    pub fn layout(&self, bounds: Rect) -> PaneGroupLayout<PaneGroupId, PaneSplitId> {
        PaneGroupLayout::new(bounds, &self.grid_node(bounds))
    }

    /// Takes all visible groups in visual tree order for tab teardown.
    pub(crate) fn take_groups(self) -> Vec<(PaneGroupId, PaneGroup)> {
        let ids = self.group_ids();
        let mut groups = self.groups;
        ids.into_iter()
            .filter_map(|id| groups.remove(&id).map(|group| (id, group)))
            .collect()
    }

    pub(crate) fn take_panes(self) -> Vec<Pane> {
        self.take_groups()
            .into_iter()
            .flat_map(|(group_id, group)| {
                group
                    .take_inputs()
                    .into_iter()
                    .map(move |(input_id, input)| Pane::new(group_id, input_id, input))
            })
            .collect()
    }

    fn allocate_group_id(&mut self) -> PaneGroupId {
        let id = PaneGroupId(self.next_group_id);
        self.next_group_id = self
            .next_group_id
            .checked_add(1)
            .expect("PaneGroup identity space exhausted");
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
    fn collect_leaves(&self, leaves: &mut Vec<PaneGroupId>) {
        match self {
            Self::Leaf(id) => leaves.push(*id),
            Self::Split { first, second, .. } => {
                first.collect_leaves(leaves);
                second.collect_leaves(leaves);
            }
        }
    }

    fn contains(&self, target: PaneGroupId) -> bool {
        match self {
            Self::Leaf(id) => *id == target,
            Self::Split { first, second, .. } => first.contains(target) || second.contains(target),
        }
    }

    fn grid_node(&self, bounds: Rect) -> GridNode<PaneGroupId, PaneSplitId> {
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

fn replace_leaf(node: &mut PaneNode, target: PaneGroupId, replacement: PaneNode) -> bool {
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

fn remove_leaf(node: &mut PaneNode, target: PaneGroupId) -> Option<PaneGroupId> {
    let sibling = match node {
        PaneNode::Split { first, second, .. } if matches!(first.as_ref(), PaneNode::Leaf(id) if *id == target) => {
            Some(std::mem::replace(
                second,
                Box::new(PaneNode::Leaf(PaneGroupId::ROOT)),
            ))
        }
        PaneNode::Split { first, second, .. } if matches!(second.as_ref(), PaneNode::Leaf(id) if *id == target) => {
            Some(std::mem::replace(
                first,
                Box::new(PaneNode::Leaf(PaneGroupId::ROOT)),
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

fn adjacent_leaf(leaves: &[PaneGroupId], active: PaneGroupId, delta: isize) -> PaneGroupId {
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
#[path = "pane_part_tests.rs"]
mod tests;
