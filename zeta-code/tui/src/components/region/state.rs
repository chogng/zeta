use crate::components::key_capture::KeyCapture;
use crate::components::key_hint::KeyHints;
use crate::components::list_selection::ListSelectionAdjustment;
use crate::components::list_selection::ListSelectionInputOutcome;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::list_selection::ListSelectionState;
use crate::components::text_prompt::TextPrompt;
use crossterm::event::KeyEvent;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RegionSpec<T> {
    body: T,
    key_hints: KeyHints,
}

impl<T> RegionSpec<T> {
    pub(crate) fn new(body: T) -> Self {
        Self {
            body,
            key_hints: KeyHints::new(),
        }
    }

    pub(crate) fn with_key_hint(
        mut self,
        keys: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        self.key_hints = self.key_hints.with(keys, label);
        self
    }

    pub(crate) fn with_key_hint_note(mut self, note: impl Into<String>) -> Self {
        self.key_hints = self.key_hints.with_note(note);
        self
    }

    pub(crate) fn into_parts(self) -> (T, KeyHints) {
        (self.body, self.key_hints)
    }

    #[cfg(test)]
    pub(crate) fn into_body(self) -> T {
        self.body
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum RegionView<'a> {
    KeyCapture(&'a KeyCapture),
    ListSelection(&'a ListSelectionState),
    TextPrompt(&'a TextPrompt),
}

impl RegionView<'_> {
    pub(crate) fn title(&self) -> &str {
        match self {
            Self::KeyCapture(body) => body.title(),
            Self::ListSelection(body) => body.title(),
            Self::TextPrompt(body) => body.title(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SelectionRegionOutcome<A> {
    Activate(A),
    Adjust(A, ListSelectionAdjustment),
    Consumed,
    Dismiss,
}

#[derive(Debug)]
pub(crate) struct SelectionRegion<A> {
    state: ListSelectionState,
    actions: BTreeMap<ListSelectionItemId, A>,
    key_hints: KeyHints,
}

impl<A> SelectionRegion<A> {
    pub(crate) fn new(
        spec: RegionSpec<ListSelectionModel>,
        actions: BTreeMap<ListSelectionItemId, A>,
    ) -> Self {
        let (model, additional_hints) = spec.into_parts();
        let key_hints = model
            .key_hints()
            .extend(additional_hints)
            .with("Esc", "to close");
        Self {
            state: ListSelectionState::new(model),
            actions,
            key_hints,
        }
    }

    pub(crate) fn replace(
        &mut self,
        spec: RegionSpec<ListSelectionModel>,
        actions: BTreeMap<ListSelectionItemId, A>,
    ) {
        let (model, additional_hints) = spec.into_parts();
        self.key_hints = model
            .key_hints()
            .extend(additional_hints)
            .with("Esc", "to close");
        self.state.replace_model(model);
        self.actions = actions;
    }

    pub(crate) fn view(&self) -> RegionView<'_> {
        RegionView::ListSelection(&self.state)
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

impl<A: Clone> SelectionRegion<A> {
    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> SelectionRegionOutcome<A> {
        match self.state.handle_key(key) {
            ListSelectionInputOutcome::Activate(item_id) => self
                .actions
                .get(&item_id)
                .cloned()
                .map(SelectionRegionOutcome::Activate)
                .unwrap_or(SelectionRegionOutcome::Consumed),
            ListSelectionInputOutcome::Adjust(item_id, adjustment) => self
                .actions
                .get(&item_id)
                .cloned()
                .map(|action| SelectionRegionOutcome::Adjust(action, adjustment))
                .unwrap_or(SelectionRegionOutcome::Consumed),
            ListSelectionInputOutcome::Consumed => SelectionRegionOutcome::Consumed,
            ListSelectionInputOutcome::Dismiss => SelectionRegionOutcome::Dismiss,
        }
    }

    pub(crate) fn activate_visible_item(
        &mut self,
        index: usize,
    ) -> Option<SelectionRegionOutcome<A>> {
        let item_id = self.state.activate_visible_item(index)?;
        self.actions
            .get(&item_id)
            .cloned()
            .map(SelectionRegionOutcome::Activate)
            .or(Some(SelectionRegionOutcome::Consumed))
    }
}
