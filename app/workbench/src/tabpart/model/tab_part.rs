use zeta_protocol::SessionId;

use crate::{TabGroup, TabGroupId, TabInput, TabInputChange, TabInputKey};

/// Workbench-owned Tab Part shared by every horizontal or vertical tab projection.
///
/// The Part owns browser-style groups and one global active input. It contains no orientation,
/// bounds, UI element identity, renderer node, or host runtime state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TabPart {
    groups: Vec<TabGroup>,
    active: Option<TabInputKey>,
    last_session: Option<SessionId>,
    next_group_id: u64,
}

impl Default for TabPart {
    fn default() -> Self {
        let mut default_group = TabGroup::new(TabGroupId::DEFAULT, None);
        default_group.push_input(TabInput::from_settings());
        Self {
            groups: vec![default_group],
            active: None,
            last_session: None,
            next_group_id: TabGroupId::DEFAULT.value() + 1,
        }
    }
}

impl TabPart {
    /// Creates a Tab Part with one anonymous group containing the Settings singleton.
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns browser-style groups in projection order.
    pub fn groups(&self) -> &[TabGroup] {
        &self.groups
    }

    /// Returns one group by stable identity.
    pub fn group(&self, id: TabGroupId) -> Option<&TabGroup> {
        self.groups.iter().find(|group| group.id() == id)
    }

    /// Returns one mutable group by stable identity.
    pub fn group_mut(&mut self, id: TabGroupId) -> Option<&mut TabGroup> {
        self.groups.iter_mut().find(|group| group.id() == id)
    }

    /// Iterates every logical input in group and tab order.
    pub fn inputs(&self) -> impl DoubleEndedIterator<Item = &TabInput> {
        self.groups.iter().flat_map(TabGroup::inputs)
    }

    /// Returns the number of logical inputs across all groups.
    pub fn input_count(&self) -> usize {
        self.groups.iter().map(|group| group.inputs().len()).sum()
    }

    /// Returns one logical input by stable key.
    pub fn input(&self, key: &TabInputKey) -> Option<&TabInput> {
        self.inputs().find(|input| input.key() == key)
    }

    /// Returns the group that owns one logical input.
    pub fn input_group(&self, key: &TabInputKey) -> Option<TabGroupId> {
        self.groups
            .iter()
            .find(|group| group.contains(key))
            .map(TabGroup::id)
    }

    /// Returns the active logical input key.
    pub fn active_tab_key(&self) -> Option<&TabInputKey> {
        self.active.as_ref()
    }

    /// Returns the active logical input.
    pub fn active_tab(&self) -> Option<&TabInput> {
        self.active_tab_key().and_then(|key| self.input(key))
    }

    /// Returns the number of Session inputs, excluding Settings.
    pub fn session_count(&self) -> usize {
        self.inputs().filter(|input| input.is_session()).count()
    }

    /// Returns a Session input by its flattened projection index.
    pub fn session_input_at(&self, index: usize) -> Option<&TabInput> {
        self.inputs().filter(|input| input.is_session()).nth(index)
    }

    /// Returns the last selected Session identity.
    pub fn selected_session(&self) -> Option<&SessionId> {
        self.last_session.as_ref()
    }

    /// Returns whether Settings is the globally active input.
    pub const fn is_settings(&self) -> bool {
        matches!(self.active, Some(TabInputKey::Settings))
    }

    /// Activates a known logical input without depending on a projection identity.
    pub fn activate_tab(&mut self, key: TabInputKey) -> bool {
        if self.input(&key).is_none() {
            return false;
        }
        if let Some(session_id) = key.session_id() {
            self.last_session = Some(session_id.clone());
        }
        self.active = Some(key);
        true
    }

    /// Activates a known Session input.
    pub fn activate_session(&mut self, session_id: &SessionId) -> bool {
        self.activate_tab(TabInputKey::session(session_id.clone()))
    }

    /// Activates the Settings singleton.
    pub fn activate_settings(&mut self) -> bool {
        self.activate_tab(TabInputKey::Settings)
    }

    /// Returns to the last selected Session input.
    pub fn activate_last_session(&mut self) -> bool {
        let Some(session_id) = self.last_session.clone() else {
            self.active = None;
            return false;
        };
        self.activate_session(&session_id)
    }

    /// Creates an empty browser-style group after the existing groups.
    pub fn create_group(&mut self, label: impl Into<String>) -> TabGroupId {
        let id = TabGroupId::from_value(self.next_group_id);
        self.next_group_id = self
            .next_group_id
            .checked_add(1)
            .expect("TabGroup identity space exhausted");
        self.groups.push(TabGroup::new(id, Some(label.into())));
        id
    }

    /// Moves one logical input into a group at the requested group-local index.
    pub fn move_tab_to_group(
        &mut self,
        key: &TabInputKey,
        target_group: TabGroupId,
        index: usize,
    ) -> bool {
        if self.group(target_group).is_none() {
            return false;
        }
        let Some(source_group) = self.input_group(key) else {
            return false;
        };
        if source_group == target_group {
            return self
                .group_mut(target_group)
                .is_some_and(|group| group.move_input(key, index));
        }

        let input = self
            .group_mut(source_group)
            .and_then(|group| group.remove_input(key))
            .expect("resolved TabGroup must own the moved TabInput");
        self.group_mut(target_group)
            .expect("validated target TabGroup must remain present")
            .insert_input(index, input);
        self.remove_empty_non_default_group(source_group);
        true
    }

    /// Combines existing tabs into one new browser-style group.
    pub fn group_tabs(
        &mut self,
        keys: impl IntoIterator<Item = TabInputKey>,
        label: impl Into<String>,
    ) -> Option<TabGroupId> {
        let group = self.create_group(label);
        let mut moved = 0;
        for key in keys {
            if self.move_tab_to_group(&key, group, moved) {
                moved += 1;
            }
        }
        if moved == 0 {
            self.remove_empty_non_default_group(group);
            None
        } else {
            Some(group)
        }
    }

    /// Merges every input from one group into another while preserving source order.
    pub fn merge_groups(&mut self, source: TabGroupId, target: TabGroupId) -> bool {
        if source == target || self.group(target).is_none() {
            return false;
        }
        let Some(keys) = self.group(source).map(|group| {
            group
                .inputs()
                .iter()
                .map(|input| input.key().clone())
                .collect::<Vec<_>>()
        }) else {
            return false;
        };
        let mut target_index = self
            .group(target)
            .expect("validated target TabGroup")
            .inputs()
            .len();
        for key in keys {
            if self.move_tab_to_group(&key, target, target_index) {
                target_index += 1;
            }
        }
        self.remove_empty_non_default_group(source);
        true
    }

    /// Removes a Session input and selects the nearest remaining tab in flattened group order.
    pub fn close_tab(&mut self, key: &TabInputKey) -> Option<TabInput> {
        if !key.is_session() {
            return None;
        }
        let ordered_keys = self
            .inputs()
            .map(|input| input.key().clone())
            .collect::<Vec<_>>();
        let index = ordered_keys.iter().position(|candidate| candidate == key)?;
        let source_group = self.input_group(key)?;
        let was_active = self.active.as_ref() == Some(key);
        let removed = self.group_mut(source_group)?.remove_input(key)?;
        self.remove_empty_non_default_group(source_group);

        if was_active {
            let remaining = self
                .inputs()
                .map(|input| input.key().clone())
                .collect::<Vec<_>>();
            let replacement_index = index.min(remaining.len().saturating_sub(1));
            self.active = remaining.get(replacement_index).cloned();
        }

        if self.last_session.as_ref() == removed.session_id() {
            let last_session = self
                .inputs()
                .rev()
                .find_map(|input| input.session_id().cloned());
            self.last_session = last_session;
        }
        if let Some(session_id) = self.active.as_ref().and_then(TabInputKey::session_id) {
            self.last_session = Some(session_id.clone());
        }
        Some(removed)
    }

    /// Updates a Session status without changing its group or identity.
    pub fn update_status(&mut self, session_id: &SessionId, status_label: &str) {
        let key = self
            .inputs()
            .find(|input| input.session_id() == Some(session_id))
            .map(|input| input.key().clone());
        if let Some(input) = key.and_then(|key| self.input_mut(&key)) {
            input.update_status(status_label);
        }
    }

    /// Inserts or refreshes a Session input using normal selection semantics.
    pub fn upsert_session_input(&mut self, input: TabInput) -> TabInputChange {
        assert!(input.is_session(), "only Session inputs can be upserted");
        let key = input.key().clone();
        let was_settings = self.is_settings();
        if let Some(existing) = self.input_mut(&key) {
            existing.update_from(input);
            self.last_session = key.session_id().cloned();
            if !was_settings {
                self.active = Some(key.clone());
            }
            return TabInputChange::Updated(key);
        }

        self.insert_session_into_default_group(input);
        self.last_session = key.session_id().cloned();
        if !was_settings {
            self.active = Some(key.clone());
        }
        TabInputChange::Added(key)
    }

    /// Inserts or refreshes a catalog Session without changing the active input.
    pub fn upsert_catalog_session_input(&mut self, input: TabInput) -> TabInputChange {
        assert!(input.is_session(), "only Session inputs can be upserted");
        let key = input.key().clone();
        if let Some(existing) = self.input_mut(&key) {
            existing.update_from(input);
            return TabInputChange::Updated(key);
        }

        self.insert_session_into_default_group(input);
        TabInputChange::Added(key)
    }

    fn input_mut(&mut self, key: &TabInputKey) -> Option<&mut TabInput> {
        self.groups
            .iter_mut()
            .find_map(|group| group.input_mut(key))
    }

    fn insert_session_into_default_group(&mut self, input: TabInput) {
        let group = self
            .group_mut(TabGroupId::DEFAULT)
            .expect("default TabGroup must remain present");
        let index = group
            .inputs()
            .iter()
            .position(TabInput::is_settings)
            .unwrap_or(group.inputs().len());
        group.insert_input(index, input);
    }

    fn remove_empty_non_default_group(&mut self, id: TabGroupId) {
        if id == TabGroupId::DEFAULT {
            return;
        }
        if self
            .group(id)
            .is_some_and(|group| group.inputs().is_empty())
        {
            self.groups.retain(|group| group.id() != id);
        }
    }
}

#[cfg(test)]
#[path = "tab_part_tests.rs"]
mod tests;
