use std::collections::HashMap;

use zeta_protocol::SessionId;

use crate::Pane;
use crate::PaneContainer;
use crate::PaneGroupId;
use crate::PaneInput;
use crate::PaneInputId;
use crate::PanePart;
use crate::PaneSplitDirection;
use crate::PaneSplitId;
use crate::TabInput;
use crate::TabInputChange;
use crate::TabInputKey;
use crate::TabPart;
use crate::TabStatus;

/// Logical state removed together with one Workbench tab.
#[derive(Clone, Debug, PartialEq)]
pub struct ClosedTab {
    key: TabInputKey,
    panes: Vec<Pane>,
    active_tab: Option<TabInputKey>,
}

impl ClosedTab {
    /// Returns the stable identity of the closed tab.
    pub fn key(&self) -> &TabInputKey {
        &self.key
    }

    /// Returns the logical panes owned by the closed tab.
    pub fn panes(&self) -> &[Pane] {
        &self.panes
    }

    /// Consumes the result and returns the logical panes owned by the closed tab.
    pub fn into_panes(self) -> Vec<Pane> {
        self.panes
    }

    /// Returns the tab selected after the close, if any.
    pub fn active_tab(&self) -> Option<&TabInputKey> {
        self.active_tab.as_ref()
    }
}

/// The Workbench state model.
///
/// `TabPart` owns orientation-neutral groups and tab inputs, and the Workbench owns one
/// [`PaneContainer`] per tab input. Each container owns its complete [`PanePart`] group topology.
/// Product hosts can present the same Tab Part vertically or horizontally without moving renderer,
/// terminal, or App Server state into this model.
#[derive(Clone, Debug, PartialEq)]
pub struct Workbench {
    tab_part: TabPart,
    pane_containers: HashMap<TabInputKey, PaneContainer>,
}

impl Default for Workbench {
    fn default() -> Self {
        let tab_part = TabPart::default();
        let pane_containers = HashMap::from([(
            TabInputKey::Settings,
            PaneContainer::with_input(PaneInput::settings()),
        )]);
        Self {
            tab_part,
            pane_containers,
        }
    }
}

impl Workbench {
    /// Creates a Workbench with the default Tab Part and no Session content.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the orientation-neutral Tab Part.
    pub const fn tab_part(&self) -> &TabPart {
        &self.tab_part
    }

    /// Returns the pane container owned by a tab item.
    pub fn pane_container(&self, tab_key: &TabInputKey) -> Option<&PaneContainer> {
        self.pane_containers.get(tab_key)
    }

    /// Returns the active tab's pane container.
    pub fn active_pane_container(&self) -> Option<&PaneContainer> {
        self.tab_part
            .active_tab_key()
            .and_then(|tab_key| self.pane_container(tab_key))
    }

    /// Returns the Pane Part inside a tab's pane container.
    pub fn pane_part(&self, tab_key: &TabInputKey) -> Option<&PanePart> {
        self.pane_container(tab_key).map(PaneContainer::pane_part)
    }

    /// Returns the mutable pane container owned by a tab item.
    pub(crate) fn pane_container_mut(
        &mut self,
        tab_key: &TabInputKey,
    ) -> Option<&mut PaneContainer> {
        self.pane_containers.get_mut(tab_key)
    }

    /// Returns the mutable Pane Part inside a tab's pane container.
    pub(crate) fn pane_part_mut(&mut self, tab_key: &TabInputKey) -> Option<&mut PanePart> {
        self.pane_container_mut(tab_key)
            .map(PaneContainer::pane_part_mut)
    }

    /// Ensures a tab has an input in its root group without replacing existing content.
    pub fn ensure_root_pane(&mut self, tab_key: TabInputKey, input: PaneInput) -> PaneGroupId {
        let pane_part = self
            .pane_containers
            .get_mut(&tab_key)
            .expect("every TabInput must own a PaneContainer")
            .pane_part_mut();
        let root_group = pane_part.root_group();
        if pane_part.active_input(root_group).is_none() {
            pane_part.mount_input(root_group, input);
        }
        root_group
    }

    /// Returns all tab keys that have a pane container.
    pub fn pane_container_keys(&self) -> impl Iterator<Item = &TabInputKey> {
        self.pane_containers.keys()
    }

    /// Removes a tab's pane container and returns all logical panes it owned.
    fn remove_pane_container(&mut self, tab_key: &TabInputKey) -> Option<Vec<Pane>> {
        self.pane_containers
            .remove(tab_key)
            .map(PaneContainer::take_panes)
    }

    /// Closes a tab and its owned pane container as one logical operation.
    pub fn close_tab(&mut self, tab_key: &TabInputKey) -> Option<ClosedTab> {
        self.tab_part.close_tab(tab_key)?;
        let panes = self
            .remove_pane_container(tab_key)
            .expect("every TabInput must own a PaneContainer");
        Some(ClosedTab {
            key: tab_key.clone(),
            panes,
            active_tab: self.tab_part.active_tab_key().cloned(),
        })
    }

    /// Activates a known tab.
    pub fn activate_tab(&mut self, key: TabInputKey) -> bool {
        self.tab_part.activate_tab(key)
    }

    /// Activates a known Session tab.
    pub fn activate_session(&mut self, session_id: &SessionId) -> bool {
        self.activate_tab(TabInputKey::session(session_id.clone()))
    }

    /// Opens or activates the singleton Settings tab.
    pub fn activate_settings(&mut self) -> bool {
        self.pane_containers
            .entry(TabInputKey::Settings)
            .or_insert_with(|| PaneContainer::with_input(PaneInput::settings()));
        self.tab_part.activate_settings()
    }

    /// Inserts or refreshes a Session tab and atomically creates its initial pane container.
    pub fn upsert_session_input(
        &mut self,
        tab_input: TabInput,
        initial_pane_input: PaneInput,
    ) -> TabInputChange {
        let key = tab_input.key().clone();
        let change = self.tab_part.upsert_session_input(tab_input);
        self.finish_session_upsert(&key, &change, initial_pane_input);
        change
    }

    /// Inserts or refreshes a catalog Session tab without changing the active tab.
    pub fn upsert_catalog_session_input(
        &mut self,
        tab_input: TabInput,
        initial_pane_input: PaneInput,
    ) -> TabInputChange {
        let key = tab_input.key().clone();
        let change = self.tab_part.upsert_catalog_session_input(tab_input);
        self.finish_session_upsert(&key, &change, initial_pane_input);
        change
    }

    /// Updates one Session tab's status metadata without exposing mutable TabPart access.
    pub fn update_session_status(&mut self, session_id: &SessionId, status: TabStatus) {
        self.tab_part.update_status(session_id, status);
    }

    /// Toggles one tab's Workbench-owned pinned state.
    pub fn toggle_tab_pin(&mut self, key: &TabInputKey) -> Option<bool> {
        self.tab_part.toggle_tab_pin(key)
    }

    /// Renames one Workbench tab without changing its Session-owned title.
    pub fn rename_tab(&mut self, key: &TabInputKey, title: impl Into<String>) -> bool {
        self.tab_part.rename_tab(key, title)
    }

    /// Moves one tab to an existing Workbench group.
    pub fn move_tab_to_group(
        &mut self,
        key: &TabInputKey,
        group: crate::TabGroupId,
        index: usize,
    ) -> bool {
        self.tab_part.move_tab_to_group(key, group, index)
    }

    /// Moves one tab into a newly created named group.
    pub fn move_tab_to_new_group(
        &mut self,
        key: &TabInputKey,
        label: impl Into<String>,
    ) -> Option<crate::TabGroupId> {
        self.tab_part.group_tabs([key.clone()], label)
    }

    /// Returns to the last selected Session tab.
    pub fn activate_last_session(&mut self) -> bool {
        self.tab_part.activate_last_session()
    }

    /// Creates a horizontally split pane in the active tab.
    pub fn create_pane(&mut self, input: PaneInput) -> Option<PaneGroupId> {
        self.create_pane_with_direction(input, PaneSplitDirection::Horizontal)
    }

    /// Creates a pane in the requested direction in the active tab.
    pub fn create_pane_with_direction(
        &mut self,
        input: PaneInput,
        direction: PaneSplitDirection,
    ) -> Option<PaneGroupId> {
        let tab_key = self.tab_part.active_tab_key()?.clone();
        let pane_part = self
            .pane_containers
            .get_mut(&tab_key)
            .expect("active TabInput must own a PaneContainer")
            .pane_part_mut();
        Some(pane_part.split_active_with_input(direction, Some(input)).0)
    }

    /// Destroys the active group in the active tab and returns all of its inputs.
    pub fn destroy_pane(&mut self) -> Option<Vec<Pane>> {
        let tab_key = self.tab_part.active_tab_key()?.clone();
        let pane_part = self.pane_part_mut(&tab_key)?;
        pane_part.destroy_active_panes()
    }

    /// Destroys a specific group in the active tab and returns all of its inputs.
    pub fn destroy_pane_by_id(&mut self, pane_id: PaneGroupId) -> Option<Vec<Pane>> {
        let tab_key = self.tab_part.active_tab_key()?.clone();
        let pane_part = self.pane_part_mut(&tab_key)?;
        pane_part.destroy_panes(pane_id)
    }

    /// Returns one pane by stable tab and group identities.
    pub fn pane(&self, tab_key: &TabInputKey, pane_id: PaneGroupId) -> Option<Pane> {
        self.pane_part(tab_key)?.pane(pane_id)
    }

    /// Returns the active pane for a specific tab identity.
    pub fn active_pane_for(&self, tab_key: &TabInputKey) -> Option<Pane> {
        let pane_part = self.pane_part(tab_key)?;
        pane_part.pane(pane_part.active_group())
    }

    /// Returns the active input for a specific tab and group identity.
    pub fn pane_input(&self, tab_key: &TabInputKey, pane_id: PaneGroupId) -> Option<&PaneInput> {
        self.pane_part(tab_key)?.pane_input(pane_id)
    }

    /// Activates a group through its stable tab and group identities.
    pub fn activate_pane(&mut self, tab_key: &TabInputKey, pane_id: PaneGroupId) -> bool {
        self.pane_part_mut(tab_key)
            .is_some_and(|pane_part| pane_part.activate(pane_id))
    }

    /// Mounts an input in a group through stable identities.
    pub fn mount_input(
        &mut self,
        tab_key: &TabInputKey,
        pane_id: PaneGroupId,
        input: PaneInput,
    ) -> Option<PaneInput> {
        self.pane_part_mut(tab_key)?.mount_input(pane_id, input)
    }

    /// Opens a second input in a group through stable identities.
    pub fn open_input(
        &mut self,
        tab_key: &TabInputKey,
        pane_id: PaneGroupId,
        input: PaneInput,
    ) -> Option<PaneInputId> {
        self.pane_part_mut(tab_key)?.open_input(pane_id, input)
    }

    /// Replaces one input through stable tab, group, and input identities.
    pub fn replace_input(
        &mut self,
        tab_key: &TabInputKey,
        pane_id: PaneGroupId,
        input_id: PaneInputId,
        input: PaneInput,
    ) -> Option<PaneInput> {
        self.pane_part_mut(tab_key)?
            .replace_input(pane_id, input_id, input)
    }

    /// Activates one input through stable tab, group, and input identities.
    pub fn activate_input(
        &mut self,
        tab_key: &TabInputKey,
        pane_id: PaneGroupId,
        input_id: PaneInputId,
    ) -> bool {
        self.pane_part_mut(tab_key)
            .is_some_and(|pane_part| pane_part.activate_input(pane_id, input_id))
    }

    /// Closes one input through stable tab, group, and input identities.
    pub fn close_input(
        &mut self,
        tab_key: &TabInputKey,
        pane_id: PaneGroupId,
        input_id: PaneInputId,
    ) -> Option<Pane> {
        self.pane_part_mut(tab_key)?.close_input(pane_id, input_id)
    }

    /// Cycles to the next visible group in a specific tab.
    pub fn focus_next_pane(&mut self, tab_key: &TabInputKey) -> Option<PaneGroupId> {
        Some(self.pane_part_mut(tab_key)?.focus_next())
    }

    /// Cycles to the previous visible group in a specific tab.
    pub fn focus_previous_pane(&mut self, tab_key: &TabInputKey) -> Option<PaneGroupId> {
        Some(self.pane_part_mut(tab_key)?.focus_previous())
    }

    /// Applies a normalized first-child ratio to one split through stable tab and split identities.
    pub fn resize_split(
        &mut self,
        tab_key: &TabInputKey,
        split_id: PaneSplitId,
        ratio: f32,
    ) -> bool {
        self.pane_part_mut(tab_key)
            .is_some_and(|pane_part| pane_part.set_split_ratio(split_id, ratio))
    }

    /// Returns the active pane in the active tab.
    pub fn active_pane(&self) -> Option<Pane> {
        let pane_part = self.active_pane_container()?.pane_part();
        pane_part.pane(pane_part.active_group())
    }

    fn finish_session_upsert(
        &mut self,
        key: &TabInputKey,
        change: &TabInputChange,
        initial_pane_input: PaneInput,
    ) {
        match change {
            TabInputChange::Added(_) => {
                let previous = self
                    .pane_containers
                    .insert(key.clone(), PaneContainer::with_input(initial_pane_input));
                assert!(
                    previous.is_none(),
                    "new TabInput must not replace a PaneContainer"
                );
            }
            TabInputChange::Updated(_) => {
                assert!(
                    self.pane_containers.contains_key(key),
                    "every TabInput must own a PaneContainer"
                );
            }
        }
    }
}

#[cfg(test)]
#[path = "workbench_tests.rs"]
mod tests;
