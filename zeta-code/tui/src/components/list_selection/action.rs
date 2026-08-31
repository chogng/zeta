use super::ListSelectionAdjustment;
use super::ListSelectionInputOutcome;
use super::ListSelectionItemId;
use super::ListSelectionModel;
use super::ListSelectionState;
use crate::components::key_hint::KeyHints;
use crossterm::event::KeyEvent;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ListSelectionOutcome<A> {
    Activate(A),
    Adjust(A, ListSelectionAdjustment),
    Consumed,
    Dismiss,
}

/// Binds an opaque feature action to each selectable list item.
#[derive(Debug)]
pub(crate) struct ListSelection<A> {
    state: ListSelectionState,
    actions: BTreeMap<ListSelectionItemId, A>,
    key_hints: KeyHints,
}

impl<A> ListSelection<A> {
    pub(crate) fn new(
        model: ListSelectionModel,
        actions: BTreeMap<ListSelectionItemId, A>,
    ) -> Self {
        let key_hints = model.key_hints().with("Esc", "to close");
        Self {
            state: ListSelectionState::new(model),
            actions,
            key_hints,
        }
    }

    pub(crate) fn replace(
        &mut self,
        model: ListSelectionModel,
        actions: BTreeMap<ListSelectionItemId, A>,
    ) {
        self.key_hints = model.key_hints().with("Esc", "to close");
        self.state.replace_model(model);
        self.actions = actions;
    }

    pub(crate) fn key_hints(&self) -> &str {
        self.key_hints.text()
    }

    pub(crate) fn state(&self) -> &ListSelectionState {
        &self.state
    }

    pub(crate) fn select_tab(&mut self, index: usize) -> bool {
        self.state.select_tab(index)
    }

    pub(crate) fn focus_search(&mut self) -> bool {
        self.state.focus_search()
    }

    pub(crate) fn selected_action(&self) -> Option<&A> {
        let id = self.state.selected_item()?.id()?;
        self.actions.get(id)
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        self.state.handle_paste(pasted);
    }
}

impl<A: Clone> ListSelection<A> {
    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> ListSelectionOutcome<A> {
        match self.state.handle_key(key) {
            ListSelectionInputOutcome::Activate(item_id) => self
                .actions
                .get(&item_id)
                .cloned()
                .map(ListSelectionOutcome::Activate)
                .unwrap_or(ListSelectionOutcome::Consumed),
            ListSelectionInputOutcome::Adjust(item_id, adjustment) => self
                .actions
                .get(&item_id)
                .cloned()
                .map(|action| ListSelectionOutcome::Adjust(action, adjustment))
                .unwrap_or(ListSelectionOutcome::Consumed),
            ListSelectionInputOutcome::Consumed => ListSelectionOutcome::Consumed,
            ListSelectionInputOutcome::Dismiss => ListSelectionOutcome::Dismiss,
        }
    }

    pub(crate) fn activate_visible_item(
        &mut self,
        index: usize,
    ) -> Option<ListSelectionOutcome<A>> {
        let item_id = self.state.activate_visible_item(index)?;
        self.actions
            .get(&item_id)
            .cloned()
            .map(ListSelectionOutcome::Activate)
            .or(Some(ListSelectionOutcome::Consumed))
    }
}
