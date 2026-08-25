use std::sync::Arc;
use std::sync::Mutex;

use crate::ui::foundation::Point;
use crate::ui::presentation::InspectionFrame;
use crate::ui::presentation::InspectionNode;
use crate::ui::presentation::InspectionNodeId;

/// A selected node and its complete parent path within one immutable inspection frame.
///
/// The nodes are copied from the frame so a product inspector can retain a selection while it
/// composes its own panel. The selection remains meaningful only for the frame it came from.
#[derive(Clone, Debug, PartialEq)]
pub struct InspectionSelection {
    path: Vec<InspectionNode>,
    selected_index: usize,
}

impl InspectionSelection {
    /// Selects the deepest inspection node at `point` and copies its ancestor path.
    pub fn at(frame: &InspectionFrame, point: Point) -> Option<Self> {
        let target = frame.target_at(point)?;
        Self::from_node(frame, target.id())
    }

    /// Copies the ancestor path for one node in `frame`.
    pub fn from_node(frame: &InspectionFrame, id: InspectionNodeId) -> Option<Self> {
        Self::from_path(frame.ancestry(id).into_iter().cloned().collect())
    }

    /// Creates a selection from an already resolved parent-to-child path.
    pub fn from_path(path: Vec<InspectionNode>) -> Option<Self> {
        if path.is_empty() {
            return None;
        }
        Some(Self {
            selected_index: path.len() - 1,
            path,
        })
    }

    /// Returns the complete parent-to-selected-node path.
    pub fn path(&self) -> &[InspectionNode] {
        &self.path
    }

    /// Returns the currently selected node, if the path is non-empty.
    pub fn target(&self) -> Option<&InspectionNode> {
        self.path.get(self.selected_index)
    }

    /// Returns the index of the selected node within [`Self::path`].
    pub const fn selected_index(&self) -> usize {
        self.selected_index
    }

    /// Retargets the selection to another node in the same parent path.
    pub fn select_index(&mut self, index: usize) -> bool {
        if index >= self.path.len() {
            return false;
        }
        self.selected_index = index;
        true
    }
}

/// Product-neutral state machine shared by interactive native layout inspectors.
///
/// The state owns enablement, picking, hover, and locked selection. A host remains responsible
/// for window policy, pointer routing, panel layout, and presentation of the selected nodes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct InspectorState {
    enabled: bool,
    picking: bool,
    hovered: Option<InspectionSelection>,
    locked: Option<InspectionSelection>,
}

impl InspectorState {
    /// Returns whether the inspector should participate in host input and presentation.
    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns whether pointer picking is active.
    pub const fn is_picking(&self) -> bool {
        self.enabled && self.picking
    }

    /// Enables the inspector and clears any selection from a previous session.
    pub fn open(&mut self) {
        self.enabled = true;
        self.picking = false;
        self.hovered = None;
        self.locked = None;
    }

    /// Toggles the inspector and returns its new enabled state.
    pub fn toggle(&mut self) -> bool {
        if self.enabled {
            self.close();
        } else {
            self.open();
        }
        self.enabled
    }

    /// Disables the inspector and clears its transient and locked selection.
    pub fn close(&mut self) {
        self.enabled = false;
        self.picking = false;
        self.hovered = None;
        self.locked = None;
    }

    /// Stops picking, or closes the inspector when picking is already stopped.
    ///
    /// Returns `true` when the inspector was closed.
    pub fn stop_picking_or_close(&mut self) -> bool {
        if self.picking {
            self.picking = false;
            self.hovered = None;
            false
        } else {
            self.close();
            true
        }
    }

    /// Toggles pointer picking and discards a previous locked selection when picking starts.
    pub fn toggle_picking(&mut self) {
        if !self.enabled {
            return;
        }
        if self.picking {
            self.picking = false;
            self.hovered = None;
        } else {
            self.picking = true;
            self.hovered = None;
            self.locked = None;
        }
    }

    /// Updates the transient hovered selection while picking is active.
    pub fn set_hovered(&mut self, selection: Option<InspectionSelection>) {
        if self.is_picking() && self.locked.is_none() {
            self.hovered = selection;
        } else {
            self.hovered = None;
        }
    }

    /// Locks a selection and stops pointer picking.
    pub fn select(&mut self, selection: Option<InspectionSelection>) {
        if !self.enabled {
            return;
        }
        self.locked = selection;
        self.hovered = None;
        self.picking = false;
    }

    /// Returns the locked selection, if any.
    pub fn locked_selection(&self) -> Option<&InspectionSelection> {
        self.locked.as_ref()
    }

    /// Returns the current locked selection or, while picking, the hovered selection.
    pub fn selection(&self) -> Option<&InspectionSelection> {
        self.locked.as_ref().or(self.hovered.as_ref())
    }

    /// Retargets the current selection to one of its ancestor rows.
    pub fn select_index(&mut self, index: usize) -> bool {
        if !self.enabled {
            return false;
        }
        let Some(mut selection) = self.selection().cloned() else {
            return false;
        };
        if !selection.select_index(index) {
            return false;
        }
        self.locked = Some(selection);
        self.hovered = None;
        self.picking = false;
        true
    }
}

/// Cloneable DevTools session capability associated with one runtime-owned window.
///
/// The runtime owns the session state so any part of an application that has the window
/// capability can open, close, or toggle DevTools without creating a second inspector state. A
/// host still decides how the session is laid out and painted in its scene.
#[derive(Clone, Debug)]
pub struct DevToolsHandle {
    state: Arc<Mutex<InspectorState>>,
}

impl Default for DevToolsHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl DevToolsHandle {
    /// Creates an independent DevTools session, primarily for a custom host or tests.
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(InspectorState::default())),
        }
    }

    /// Returns whether this window's DevTools session is open.
    pub fn is_open(&self) -> bool {
        self.state.lock().expect("devtools state lock").is_enabled()
    }

    /// Returns whether pointer picking is active.
    pub fn is_picking(&self) -> bool {
        self.state.lock().expect("devtools state lock").is_picking()
    }

    /// Opens the DevTools session and clears its previous selection.
    pub fn open(&self) {
        self.state.lock().expect("devtools state lock").open();
    }

    /// Closes the DevTools session and clears its selection.
    pub fn close(&self) {
        self.state.lock().expect("devtools state lock").close();
    }

    /// Toggles the DevTools session and returns whether it is now open.
    pub fn toggle(&self) -> bool {
        self.state.lock().expect("devtools state lock").toggle()
    }

    /// Stops picking, or closes the session when picking is already stopped.
    ///
    /// Returns `true` when the session was closed.
    pub fn stop_picking_or_close(&self) -> bool {
        self.state
            .lock()
            .expect("devtools state lock")
            .stop_picking_or_close()
    }

    /// Toggles pointer picking when the session is open.
    pub fn toggle_picking(&self) {
        self.state
            .lock()
            .expect("devtools state lock")
            .toggle_picking();
    }

    /// Updates the transient hovered selection while picking.
    pub fn set_hovered(&self, selection: Option<InspectionSelection>) {
        self.state
            .lock()
            .expect("devtools state lock")
            .set_hovered(selection);
    }

    /// Locks a selection and stops pointer picking.
    pub fn select(&self, selection: Option<InspectionSelection>) {
        self.state
            .lock()
            .expect("devtools state lock")
            .select(selection);
    }

    /// Retargets the current selection to one of its ancestor rows.
    pub fn select_index(&self, index: usize) -> bool {
        self.state
            .lock()
            .expect("devtools state lock")
            .select_index(index)
    }

    /// Returns the locked selection, if any.
    pub fn locked_selection(&self) -> Option<InspectionSelection> {
        self.state
            .lock()
            .expect("devtools state lock")
            .locked_selection()
            .cloned()
    }

    /// Returns the locked selection or the current hover selection.
    pub fn selection(&self) -> Option<InspectionSelection> {
        self.state
            .lock()
            .expect("devtools state lock")
            .selection()
            .cloned()
    }
}

#[cfg(test)]
#[path = "inspection_tests.rs"]
mod tests;
