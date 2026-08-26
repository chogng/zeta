use std::collections::HashMap;

use zeta_protocol::SessionId;

use crate::terminal_session::TerminalSessionKey;
use crate::workbench_host::pane_input::PaneBinding;
use crate::workbench_host::{PaneId, PaneInput, PaneInputKind, TabInputKey};
use zeta_workbench::PanePart;

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
    input: &'a PaneInput,
    binding: &'a PaneBinding,
}

impl<'a> PaneViewMount<'a> {
    pub(crate) const fn pane_id(self) -> PaneId {
        self.pane_id
    }

    pub(crate) fn kind(self) -> PaneInputKind {
        self.input.kind()
    }

    pub(crate) fn terminal_key(self) -> Option<TerminalSessionKey> {
        self.binding.terminal_key()
    }
}

/// Native host boundary between PaneGroup topology and content-specific PaneViews.
///
/// `PanePart` owns the tree and focus state, and each `PaneGroup` owns its logical inputs.
/// `PaneHost` only owns the mapping from a visible group to its optional product runtime. It
/// deliberately does not own feature state or renderer nodes, so a layout can mount heterogeneous
/// content without making the model generic.
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

    /// Removes every product runtime binding owned by one logical Workbench tab.
    pub(crate) fn remove_tab(&mut self, tab_key: &TabInputKey) -> Vec<PaneBinding> {
        let scope = PaneHostScope::Tab(tab_key.clone());
        let bindings = std::mem::take(&mut self.bindings);
        let mut removed = Vec::new();
        for (key, binding) in bindings {
            if key.0 == scope {
                removed.push(binding);
            } else {
                self.bindings.insert(key, binding);
            }
        }
        removed
    }

    pub(crate) fn binding(&self, key: &PaneBindingKey) -> Option<&PaneBinding> {
        self.bindings.get(key)
    }

    pub(crate) fn terminal_key(&self, key: &PaneBindingKey) -> Option<TerminalSessionKey> {
        self.binding(key).and_then(PaneBinding::terminal_key)
    }

    /// Ensures a visible group has a matching Terminal input before attaching its runtime key.
    pub(crate) fn ensure_terminal(
        &mut self,
        key: PaneBindingKey,
        input: &PaneInput,
        session_id: &SessionId,
        terminal_key: TerminalSessionKey,
    ) -> bool {
        let binding = self.bindings.entry(key).or_insert_with(PaneBinding::new);
        binding.bind_terminal(input, session_id, terminal_key)
    }

    /// Produces one content mount only for a leaf that belongs to the supplied group.
    pub(crate) fn mount<'a>(
        &'a self,
        scope: &PaneHostScope,
        layout: &'a PanePart,
        pane_id: PaneId,
    ) -> Option<PaneViewMount<'a>> {
        let input = layout.active_input(pane_id)?;
        let binding = self.binding(&(scope.clone(), pane_id))?;
        Some(PaneViewMount {
            pane_id,
            input,
            binding,
        })
    }
}

#[cfg(test)]
#[path = "pane_host_tests.rs"]
mod tests;
