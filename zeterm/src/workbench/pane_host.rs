use std::collections::HashMap;

use zeta_protocol::SessionId;

use crate::pane_group::{PaneGroup, PaneId};
use crate::pane_input::{PaneBinding, PaneInput, PaneInputKind};
use crate::tab_input::TabInputKey;
use crate::terminal_session::TerminalSessionKey;

/// Product scope that owns a PaneGroup mounted by the Native host.
///
/// Every visible product pane belongs to the active Session workbench group. Terminal startup and
/// feature-specific view state remain Native-owned; this enum only keeps the host binding keyed to
/// the logical tab that owns the pane tree.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum PaneHostScope {
    Tab(TabInputKey),
}

type PaneBindingKey = (PaneHostScope, PaneId);

/// A mounted Pane content descriptor for one frame.
///
/// The mount borrows the host-owned binding. It carries no retained UI node or feature state;
/// those are created by the feature-specific view when the host dispatches this mount.
#[derive(Clone, Copy)]
pub(crate) struct PaneViewMount<'a> {
    pane_id: PaneId,
    binding: &'a PaneBinding,
}

impl<'a> PaneViewMount<'a> {
    pub(crate) const fn pane_id(self) -> PaneId {
        self.pane_id
    }

    pub(crate) fn kind(self) -> PaneInputKind {
        self.binding.input().kind()
    }

    pub(crate) fn terminal_key(self) -> Option<TerminalSessionKey> {
        self.binding.terminal_key()
    }
}

/// Native host boundary between PaneGroup topology and content-specific PaneViews.
///
/// `PaneGroup` owns the tree and focus state. `PaneHost` owns the mapping from a leaf to its
/// logical input and optional runtime. It deliberately does not own feature state or renderer
/// nodes, so a group can mount heterogeneous content without making the layout model generic.
#[derive(Default)]
pub(crate) struct PaneHost {
    bindings: HashMap<PaneBindingKey, PaneBinding>,
}

impl PaneHost {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn insert(&mut self, key: PaneBindingKey, binding: PaneBinding) {
        self.bindings.insert(key, binding);
    }

    pub(crate) fn remove(&mut self, key: &PaneBindingKey) -> Option<PaneBinding> {
        self.bindings.remove(key)
    }

    pub(crate) fn binding(&self, key: &PaneBindingKey) -> Option<&PaneBinding> {
        self.bindings.get(key)
    }

    pub(crate) fn terminal_key(&self, key: &PaneBindingKey) -> Option<TerminalSessionKey> {
        self.binding(key).and_then(PaneBinding::terminal_key)
    }

    pub(crate) fn kind(&self, key: &PaneBindingKey) -> Option<PaneInputKind> {
        self.binding(key).map(|binding| binding.input().kind())
    }

    /// Ensures a leaf has a matching TerminalPaneInput before attaching its runtime key.
    pub(crate) fn ensure_terminal(
        &mut self,
        key: PaneBindingKey,
        session_id: &SessionId,
        terminal_key: TerminalSessionKey,
    ) -> bool {
        let binding = self
            .bindings
            .entry(key)
            .or_insert_with(|| PaneBinding::new(PaneInput::terminal(session_id.clone())));
        binding.bind_terminal(session_id, terminal_key)
    }

    /// Produces one content mount only for a leaf that belongs to the supplied group.
    pub(crate) fn mount<'a>(
        &'a self,
        scope: &PaneHostScope,
        group: &PaneGroup,
        pane_id: PaneId,
    ) -> Option<PaneViewMount<'a>> {
        if !group.leaf_ids().contains(&pane_id) {
            return None;
        }
        let binding = self.binding(&(scope.clone(), pane_id))?;
        Some(PaneViewMount { pane_id, binding })
    }
}

#[cfg(test)]
#[path = "pane_host_tests.rs"]
mod tests;
