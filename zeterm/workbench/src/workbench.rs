use std::collections::HashMap;

use zeta_protocol::Session;
use zeta_protocol::SessionId;

use crate::Pane;
use crate::PaneId;
use crate::PaneInput;
use crate::PaneInputId;
use crate::PanePart;
use crate::PaneSplitDirection;
use crate::PaneSplitId;
use crate::TabInputChange;
use crate::TabInputKey;
use crate::TabPart;
use crate::TitlebarPart;
use zui::ui::SplitViewResize;

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
/// `TabPart` owns projection-neutral groups and tab inputs, `TitlebarPart` is the command surface,
/// and the Workbench owns one [`PanePart`] per tab input. Product hosts can project the same Tab
/// Part vertically or horizontally without moving renderer, terminal, or App Server state into
/// this model.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Workbench {
    titlebar_part: TitlebarPart,
    tab_part: TabPart,
    pane_parts: HashMap<TabInputKey, PanePart>,
}

impl Workbench {
    /// Creates a Workbench with the default Tab Part and no Session content.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the titlebar command surface.
    pub const fn titlebar_part(&self) -> &TitlebarPart {
        &self.titlebar_part
    }

    /// Returns the titlebar command surface.
    pub const fn titlebar(&self) -> &TitlebarPart {
        self.titlebar_part()
    }

    /// Returns the projection-neutral Tab Part.
    pub const fn tab_part(&self) -> &TabPart {
        &self.tab_part
    }

    /// Returns the mutable projection-neutral Tab Part.
    pub fn tab_part_mut(&mut self) -> &mut TabPart {
        &mut self.tab_part
    }

    /// Returns the Pane Part owned by a tab item.
    pub fn pane_part(&self, tab_key: &TabInputKey) -> Option<&PanePart> {
        self.pane_parts.get(tab_key)
    }

    /// Returns the mutable Pane Part owned by a tab item.
    pub(crate) fn pane_part_mut(&mut self, tab_key: &TabInputKey) -> Option<&mut PanePart> {
        self.pane_parts.get_mut(tab_key)
    }

    /// Returns the Pane Part for a tab item, creating an empty one when necessary.
    pub(crate) fn ensure_pane_part(&mut self, tab_key: TabInputKey) -> &mut PanePart {
        self.pane_parts.entry(tab_key).or_insert_with(PanePart::new)
    }

    /// Ensures a tab has an input in its root group without replacing existing content.
    pub fn ensure_root_pane(&mut self, tab_key: TabInputKey, input: PaneInput) -> PaneId {
        let pane_part = self.ensure_pane_part(tab_key);
        let root_group = pane_part.root_group();
        if pane_part.active_input(root_group).is_none() {
            pane_part.mount_input(root_group, input);
        }
        root_group
    }

    /// Returns all tab keys that have a Pane Part.
    pub fn pane_part_keys(&self) -> impl Iterator<Item = &TabInputKey> {
        self.pane_parts.keys()
    }

    /// Removes a tab's Pane Part and returns all logical inputs it owned.
    pub fn remove_pane_part(&mut self, tab_key: &TabInputKey) -> Option<Vec<Pane>> {
        self.pane_parts.remove(tab_key).map(PanePart::take_panes)
    }

    /// Closes a Session tab and its owned Pane Part as one logical operation.
    pub fn close_tab(&mut self, tab_key: &TabInputKey) -> Option<ClosedTab> {
        self.tab_part.close_tab(tab_key)?;
        let panes = self.remove_pane_part(tab_key).unwrap_or_default();
        Some(ClosedTab {
            key: tab_key.clone(),
            panes,
            active_tab: self.tab_part.active_tab_key().cloned(),
        })
    }

    /// Activates a known tab and initializes its default content pane when necessary.
    pub fn activate_tab(&mut self, key: TabInputKey) -> bool {
        if !self.tab_part.activate_tab(key.clone()) {
            return false;
        }
        self.ensure_default_pane(&key);
        true
    }

    /// Activates a known Session tab.
    pub fn activate_session(&mut self, session_id: &SessionId) -> bool {
        self.activate_tab(TabInputKey::session(session_id.clone()))
    }

    /// Activates the singleton Settings tab.
    pub fn activate_settings(&mut self) -> bool {
        if !self.tab_part.activate_settings() {
            return false;
        }
        self.ensure_default_pane(&TabInputKey::Settings);
        true
    }

    /// Inserts or refreshes a Session tab and its default Terminal pane.
    pub fn upsert_session(&mut self, session: &Session, workspace: &str) -> TabInputChange {
        let key = TabInputKey::session(session.session_id.clone());
        let change = self.tab_part.upsert_session(session, workspace);
        self.ensure_default_pane(&key);
        change
    }

    /// Inserts or refreshes a catalog Session tab without changing the active tab.
    pub fn upsert_catalog_session(&mut self, session: &Session, workspace: &str) -> TabInputChange {
        let key = TabInputKey::session(session.session_id.clone());
        let change = self.tab_part.upsert_catalog_session(session, workspace);
        self.ensure_default_pane(&key);
        change
    }

    /// Creates a horizontally split pane in the active tab.
    pub fn create_pane(&mut self, input: PaneInput) -> Option<PaneId> {
        self.create_pane_with_direction(input, PaneSplitDirection::Horizontal)
    }

    /// Creates a pane in the requested direction in the active tab.
    pub fn create_pane_with_direction(
        &mut self,
        input: PaneInput,
        direction: PaneSplitDirection,
    ) -> Option<PaneId> {
        let tab_key = self.tab_part.active_tab_key()?.clone();
        let titlebar_part = self.titlebar_part;
        let pane_part = self.ensure_pane_part(tab_key);
        Some(titlebar_part.create_pane_with_direction(pane_part, input, direction))
    }

    /// Destroys the active group in the active tab and returns all of its inputs.
    pub fn destroy_pane(&mut self) -> Option<Vec<Pane>> {
        let tab_key = self.tab_part.active_tab_key()?.clone();
        let titlebar_part = self.titlebar_part;
        let pane_part = self.pane_part_mut(&tab_key)?;
        titlebar_part.destroy_active_panes(pane_part)
    }

    /// Destroys a specific group in the active tab and returns all of its inputs.
    pub fn destroy_pane_by_id(&mut self, pane_id: PaneId) -> Option<Vec<Pane>> {
        let tab_key = self.tab_part.active_tab_key()?.clone();
        let titlebar_part = self.titlebar_part;
        let pane_part = self.pane_part_mut(&tab_key)?;
        titlebar_part.destroy_panes(pane_part, pane_id)
    }

    /// Returns one pane by stable tab and group identities.
    pub fn pane(&self, tab_key: &TabInputKey, pane_id: PaneId) -> Option<Pane> {
        self.pane_part(tab_key)?.pane(pane_id)
    }

    /// Returns the active pane for a specific tab identity.
    pub fn active_pane_for(&self, tab_key: &TabInputKey) -> Option<Pane> {
        let pane_part = self.pane_part(tab_key)?;
        pane_part.pane(pane_part.active_group())
    }

    /// Returns the active input for a specific tab and group identity.
    pub fn pane_input(&self, tab_key: &TabInputKey, pane_id: PaneId) -> Option<&PaneInput> {
        self.pane_part(tab_key)?.pane_input(pane_id)
    }

    /// Activates a group through its stable tab and group identities.
    pub fn activate_pane(&mut self, tab_key: &TabInputKey, pane_id: PaneId) -> bool {
        self.pane_part_mut(tab_key)
            .is_some_and(|pane_part| pane_part.activate(pane_id))
    }

    /// Mounts an input in a group through stable identities.
    pub fn mount_input(
        &mut self,
        tab_key: &TabInputKey,
        pane_id: PaneId,
        input: PaneInput,
    ) -> Option<PaneInput> {
        self.pane_part_mut(tab_key)?.mount_input(pane_id, input)
    }

    /// Opens a second input in a group through stable identities.
    pub fn open_input(
        &mut self,
        tab_key: &TabInputKey,
        pane_id: PaneId,
        input: PaneInput,
    ) -> Option<PaneInputId> {
        self.pane_part_mut(tab_key)?.open_input(pane_id, input)
    }

    /// Replaces one input through stable tab, group, and input identities.
    pub fn replace_input(
        &mut self,
        tab_key: &TabInputKey,
        pane_id: PaneId,
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
        pane_id: PaneId,
        input_id: PaneInputId,
    ) -> bool {
        self.pane_part_mut(tab_key)
            .is_some_and(|pane_part| pane_part.activate_input(pane_id, input_id))
    }

    /// Closes one input through stable tab, group, and input identities.
    pub fn close_input(
        &mut self,
        tab_key: &TabInputKey,
        pane_id: PaneId,
        input_id: PaneInputId,
    ) -> Option<Pane> {
        self.pane_part_mut(tab_key)?.close_input(pane_id, input_id)
    }

    /// Cycles to the next visible group in a specific tab.
    pub fn focus_next_pane(&mut self, tab_key: &TabInputKey) -> Option<PaneId> {
        Some(self.pane_part_mut(tab_key)?.focus_next())
    }

    /// Cycles to the previous visible group in a specific tab.
    pub fn focus_previous_pane(&mut self, tab_key: &TabInputKey) -> Option<PaneId> {
        Some(self.pane_part_mut(tab_key)?.focus_previous())
    }

    /// Resizes one split through stable tab and split identities.
    pub fn resize_split(
        &mut self,
        tab_key: &TabInputKey,
        split_id: PaneSplitId,
        resize: SplitViewResize,
    ) -> bool {
        self.pane_part_mut(tab_key)
            .is_some_and(|pane_part| pane_part.resize_split(split_id, resize))
    }

    /// Saves a product input to restore when this tab returns from a workspace surface.
    pub fn remember_workspace_return(&mut self, tab_key: &TabInputKey, input: PaneInput) -> bool {
        let Some(pane_part) = self.pane_part_mut(tab_key) else {
            return false;
        };
        pane_part.remember_workspace_return(input);
        true
    }

    /// Takes a saved workspace input for a specific tab.
    pub fn take_workspace_return(&mut self, tab_key: &TabInputKey) -> Option<PaneInput> {
        self.pane_part_mut(tab_key)?.take_workspace_return()
    }

    /// Drops a saved workspace input for a specific tab.
    pub fn clear_workspace_return(&mut self, tab_key: &TabInputKey) -> bool {
        let Some(pane_part) = self.pane_part_mut(tab_key) else {
            return false;
        };
        pane_part.clear_workspace_return();
        true
    }

    /// Returns the active pane in the active tab.
    pub fn active_pane(&self) -> Option<Pane> {
        let tab_key = self.tab_part.active_tab_key()?;
        let pane_part = self.pane_part(tab_key)?;
        pane_part.pane(pane_part.active_group())
    }

    fn ensure_default_pane(&mut self, key: &TabInputKey) {
        let input = match key {
            TabInputKey::Session(session_id) => PaneInput::terminal(session_id.clone()),
            TabInputKey::Settings => PaneInput::settings(),
        };
        self.ensure_root_pane(key.clone(), input);
    }
}

#[cfg(test)]
#[path = "workbench_tests.rs"]
mod tests;
