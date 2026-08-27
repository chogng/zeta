//! Product-neutral coordination for a Workbench model and its content bindings.
//!
//! This crate owns the boundary between logical Workbench state and product runtime bindings. It
//! does not know which runtime is attached to a pane and does not own rendering, interaction
//! dispatch, or frame scheduling.

use std::collections::HashMap;

pub use zeta_workbench::ClosedTab;
pub use zeta_workbench::InspectorPartState;
pub use zeta_workbench::Pane;
pub use zeta_workbench::PaneGroup;
pub use zeta_workbench::PaneGroupId;
pub use zeta_workbench::PaneId;
pub use zeta_workbench::PaneInput;
pub use zeta_workbench::PaneInputKind;
pub use zeta_workbench::PaneNode;
pub use zeta_workbench::PanePart;
pub use zeta_workbench::PaneSplitDirection;
pub use zeta_workbench::PaneSplitId;
pub use zeta_workbench::TabGroup;
pub use zeta_workbench::TabGroupId;
pub use zeta_workbench::TabInput;
pub use zeta_workbench::TabInputChange;
pub use zeta_workbench::TabInputKey;
pub use zeta_workbench::TabPart;
pub use zeta_workbench::TitlebarPart;
pub use zeta_workbench::Workbench;
use zeta_workbench_layout::LogicalViewport;
use zeta_workbench_layout::WorkbenchLayout;
use zeta_workbench_layout::WorkbenchLayoutSpec;

/// Product scope that owns a mounted pane tree.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum PaneHostScope {
    /// The pane belongs to one logical Workbench tab.
    Tab(TabInputKey),
}

/// Stable lookup key for one product binding attached to a logical pane.
pub type PaneKey = (PaneHostScope, PaneGroupId);

/// Opaque identity for one binding entry in a [`PaneHost`].
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PaneBindingId(u64);

impl PaneBindingId {
    const FIRST: u64 = 1;

    /// Returns the process-local binding number.
    pub const fn value(self) -> u64 {
        self.0
    }
}

struct BindingEntry<B> {
    id: PaneBindingId,
    binding: B,
}

/// Registry connecting visible logical panes to product-owned runtime bindings.
///
/// `B` is supplied by the product host. The registry never inspects or constrains that type, so
/// Terminal, Agent, Editor, and Settings integrations can share the same logical contract.
pub struct PaneHost<B> {
    bindings: HashMap<PaneKey, BindingEntry<B>>,
    next_binding_id: u64,
}

impl<B> Default for PaneHost<B> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B> PaneHost<B> {
    /// Creates an empty binding registry.
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
            next_binding_id: PaneBindingId::FIRST,
        }
    }

    /// Inserts or replaces the binding for one logical pane and returns its opaque identity.
    pub fn insert(&mut self, key: PaneKey, binding: B) -> PaneBindingId {
        let id = self.allocate_binding_id();
        self.bindings.insert(key, BindingEntry { id, binding });
        id
    }

    /// Binds product state to one logical pane and returns its opaque identity.
    pub fn bind(&mut self, key: PaneKey, binding: B) -> PaneBindingId {
        self.insert(key, binding)
    }

    /// Returns the existing binding or inserts the supplied empty binding for one logical pane.
    pub fn ensure(&mut self, key: PaneKey, binding: B) -> &mut B {
        if !self.bindings.contains_key(&key) {
            let id = self.allocate_binding_id();
            self.bindings
                .insert(key.clone(), BindingEntry { id, binding });
        }
        &mut self
            .bindings
            .get_mut(&key)
            .expect("ensured Pane binding must be present")
            .binding
    }

    /// Removes one logical pane binding.
    pub fn remove(&mut self, key: &PaneKey) -> Option<B> {
        self.bindings.remove(key).map(|entry| entry.binding)
    }

    /// Unbinds product state from one logical pane.
    pub fn unbind(&mut self, key: &PaneKey) -> Option<B> {
        self.remove(key)
    }

    /// Removes every binding owned by one logical Workbench tab.
    pub fn remove_tab(&mut self, tab_key: &TabInputKey) -> Vec<B> {
        let scope = PaneHostScope::Tab(tab_key.clone());
        let bindings = std::mem::take(&mut self.bindings);
        let mut removed = Vec::new();
        for (key, entry) in bindings {
            if key.0 == scope {
                removed.push(entry);
            } else {
                self.bindings.insert(key, entry);
            }
        }
        removed.sort_by_key(|entry| entry.id);
        removed.into_iter().map(|entry| entry.binding).collect()
    }

    /// Returns a product binding without exposing registry internals.
    pub fn binding(&self, key: &PaneKey) -> Option<&B> {
        self.bindings.get(key).map(|entry| &entry.binding)
    }

    /// Returns the opaque identity of one binding.
    pub fn binding_id(&self, key: &PaneKey) -> Option<PaneBindingId> {
        self.bindings.get(key).map(|entry| entry.id)
    }

    /// Produces a mount descriptor only for an active input in the supplied Pane Part.
    pub fn mount<'a>(
        &'a self,
        scope: &PaneHostScope,
        pane_part: &'a PanePart,
        pane_id: PaneGroupId,
    ) -> Option<PaneMount<'a, B>> {
        let input = pane_part.active_input(pane_id)?;
        let key = (scope.clone(), pane_id);
        let entry = self.bindings.get(&key)?;
        Some(PaneMount {
            pane_id,
            input,
            binding_id: entry.id,
            binding: &entry.binding,
        })
    }

    fn allocate_binding_id(&mut self) -> PaneBindingId {
        let id = PaneBindingId(self.next_binding_id);
        self.next_binding_id = self
            .next_binding_id
            .checked_add(1)
            .expect("Pane binding identity space exhausted");
        id
    }
}

/// Immutable description of one mounted logical pane.
pub struct PaneMount<'a, B> {
    pane_id: PaneGroupId,
    input: &'a PaneInput,
    binding_id: PaneBindingId,
    binding: &'a B,
}

impl<'a, B> Copy for PaneMount<'a, B> {}

impl<'a, B> Clone for PaneMount<'a, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<'a, B> PaneMount<'a, B> {
    /// Returns the stable logical group identity.
    pub const fn pane_id(self) -> PaneGroupId {
        self.pane_id
    }

    /// Returns the logical input kind.
    pub const fn kind(self) -> PaneInputKind {
        self.input.kind()
    }

    /// Returns the logical input description.
    pub const fn input(self) -> &'a PaneInput {
        self.input
    }

    /// Returns the opaque binding identity.
    pub const fn binding_id(self) -> PaneBindingId {
        self.binding_id
    }

    /// Returns the product binding associated with this mount.
    pub const fn binding(self) -> &'a B {
        self.binding
    }
}

/// Workbench coordinator containing logical state and generic pane bindings.
pub struct WorkbenchHost<B> {
    workbench: Workbench,
    pane_host: PaneHost<B>,
}

impl<B> Default for WorkbenchHost<B> {
    fn default() -> Self {
        Self::new()
    }
}

impl<B> WorkbenchHost<B> {
    /// Creates a Workbench coordinator with an empty binding registry.
    pub fn new() -> Self {
        Self {
            workbench: Workbench::new(),
            pane_host: PaneHost::new(),
        }
    }

    /// Returns the logical Workbench model.
    pub const fn workbench(&self) -> &Workbench {
        &self.workbench
    }

    /// Returns the mutable logical Workbench model.
    pub const fn workbench_mut(&mut self) -> &mut Workbench {
        &mut self.workbench
    }

    /// Returns the product binding registry.
    pub const fn pane_host(&self) -> &PaneHost<B> {
        &self.pane_host
    }

    /// Returns the mutable product binding registry.
    pub const fn pane_host_mut(&mut self) -> &mut PaneHost<B> {
        &mut self.pane_host
    }

    /// Closes one logical Workbench tab and removes all bindings owned by that tab.
    ///
    /// The returned bindings remain product-owned. A product host can use them to release a
    /// Terminal, Agent, Editor, or other runtime after the logical tab has been removed.
    pub fn close_tab(&mut self, tab_key: &TabInputKey) -> Option<(ClosedTab, Vec<B>)> {
        let closed = self.workbench.close_tab(tab_key)?;
        let bindings = self.pane_host.remove_tab(tab_key);
        Some((closed, bindings))
    }

    /// Resolves Workbench geometry without participating in rendering.
    pub fn layout(
        &self,
        spec: WorkbenchLayoutSpec,
        viewport: LogicalViewport,
    ) -> Option<WorkbenchLayout> {
        spec.for_viewport(viewport)
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
