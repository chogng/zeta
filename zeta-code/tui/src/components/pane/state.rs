use crate::components::key_capture::KeyCapture;
use crate::components::list_selection::ListSelectionAdjustment;
use crate::components::list_selection::ListSelectionInputOutcome;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::list_selection::ListSelectionState;
use crate::components::text_prompt::TextPrompt;
use crate::components::text_prompt::TextPromptOutcome;
use crate::components::text_prompt::TextPromptSpec;
use crossterm::event::KeyEvent;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct PaneId(u64);

impl PaneId {
    pub(crate) fn new(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PaneSpec<T> {
    body: T,
    key_hints: String,
}

impl<T> PaneSpec<T> {
    pub(crate) fn new(body: T, key_hints: impl Into<String>) -> Self {
        Self {
            body,
            key_hints: key_hints.into(),
        }
    }

    pub(crate) fn into_parts(self) -> (T, String) {
        (self.body, self.key_hints)
    }

    #[cfg(test)]
    pub(crate) fn into_body(self) -> T {
        self.body
    }
}

#[derive(Debug)]
pub(crate) struct Pane {
    id: PaneId,
    body: PaneBody,
    key_hints: String,
}

#[derive(Debug)]
enum PaneBody {
    KeyCapture(KeyCapture),
    ListSelection(ListSelectionState),
    TextPrompt(TextPrompt),
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct PaneView<'a> {
    body: PaneBodyView<'a>,
    key_hints: &'a str,
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum PaneBodyView<'a> {
    KeyCapture(&'a KeyCapture),
    ListSelection(&'a ListSelectionState),
    TextPrompt(&'a TextPrompt),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PaneOutcome {
    ActivateSelection(ListSelectionItemId),
    AdjustSelection(ListSelectionItemId, ListSelectionAdjustment),
    KeyCaptured(KeyEvent),
    SubmitText(String),
    Consumed,
    Dismiss,
}

#[derive(Debug)]
pub(crate) struct PaneStack {
    entries: Vec<Pane>,
    next_id: u64,
}

impl Default for PaneStack {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            next_id: 1,
        }
    }
}

impl Pane {
    fn new(id: PaneId, body: PaneBody, key_hints: String) -> Self {
        Self {
            id,
            body,
            key_hints,
        }
    }

    pub(crate) fn id(&self) -> PaneId {
        self.id
    }

    pub(crate) fn view(&self) -> PaneView<'_> {
        let body = match &self.body {
            PaneBody::KeyCapture(body) => PaneBodyView::KeyCapture(body),
            PaneBody::ListSelection(body) => PaneBodyView::ListSelection(body),
            PaneBody::TextPrompt(body) => PaneBodyView::TextPrompt(body),
        };
        PaneView {
            body,
            key_hints: &self.key_hints,
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> PaneOutcome {
        match &mut self.body {
            PaneBody::KeyCapture(_) => PaneOutcome::KeyCaptured(key),
            PaneBody::ListSelection(body) => match body.handle_key(key) {
                ListSelectionInputOutcome::Activate(item_id) => {
                    PaneOutcome::ActivateSelection(item_id)
                }
                ListSelectionInputOutcome::Adjust(item_id, adjustment) => {
                    PaneOutcome::AdjustSelection(item_id, adjustment)
                }
                ListSelectionInputOutcome::Consumed => PaneOutcome::Consumed,
                ListSelectionInputOutcome::Dismiss => PaneOutcome::Dismiss,
            },
            PaneBody::TextPrompt(body) => match body.handle_key(key) {
                TextPromptOutcome::Consumed => PaneOutcome::Consumed,
                TextPromptOutcome::Dismiss => PaneOutcome::Dismiss,
                TextPromptOutcome::Submit(value) => PaneOutcome::SubmitText(value),
            },
        }
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        match &mut self.body {
            PaneBody::ListSelection(body) => body.handle_paste(pasted),
            PaneBody::TextPrompt(body) => body.handle_paste(pasted),
            PaneBody::KeyCapture(_) => {}
        }
    }

    fn replace_key_capture(&mut self, body: KeyCapture, key_hints: String) -> bool {
        let PaneBody::KeyCapture(current) = &mut self.body else {
            return false;
        };
        *current = body;
        self.key_hints = key_hints;
        true
    }

    fn replace_list_selection(&mut self, body: ListSelectionModel, key_hints: String) -> bool {
        let PaneBody::ListSelection(current) = &mut self.body else {
            return false;
        };
        current.replace_model(body);
        self.key_hints = key_hints;
        true
    }

    fn list_selection(&self) -> Option<&ListSelectionState> {
        match &self.body {
            PaneBody::ListSelection(body) => Some(body),
            _ => None,
        }
    }

    fn list_selection_mut(&mut self) -> Option<&mut ListSelectionState> {
        match &mut self.body {
            PaneBody::ListSelection(body) => Some(body),
            _ => None,
        }
    }
}

impl PaneView<'_> {
    pub(crate) fn body(&self) -> PaneBodyView<'_> {
        self.body
    }

    pub(crate) fn key_hints(&self) -> &str {
        self.key_hints
    }
}

impl PaneStack {
    pub(crate) fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub(crate) fn top(&self) -> Option<&Pane> {
        self.entries.last()
    }

    pub(crate) fn top_mut(&mut self) -> Option<&mut Pane> {
        self.entries.last_mut()
    }

    pub(crate) fn top_id(&self) -> Option<PaneId> {
        self.top().map(Pane::id)
    }

    pub(crate) fn top_view(&self) -> Option<PaneView<'_>> {
        self.top().map(Pane::view)
    }

    pub(crate) fn push_key_capture(&mut self, spec: PaneSpec<KeyCapture>) -> PaneId {
        let (body, key_hints) = spec.into_parts();
        self.push(PaneBody::KeyCapture(body), key_hints)
    }

    pub(crate) fn push_list_selection(&mut self, spec: PaneSpec<ListSelectionModel>) -> PaneId {
        let (body, key_hints) = spec.into_parts();
        self.push(
            PaneBody::ListSelection(ListSelectionState::new(body)),
            key_hints,
        )
    }

    pub(crate) fn push_text_prompt(&mut self, spec: PaneSpec<TextPromptSpec>) -> PaneId {
        let (body, key_hints) = spec.into_parts();
        self.push(PaneBody::TextPrompt(TextPrompt::new(body)), key_hints)
    }

    pub(crate) fn update_top_key_capture(&mut self, spec: PaneSpec<KeyCapture>) -> Option<PaneId> {
        let (body, key_hints) = spec.into_parts();
        let pane = self.top_mut()?;
        pane.replace_key_capture(body, key_hints)
            .then_some(pane.id())
    }

    pub(crate) fn update_top_list_selection(
        &mut self,
        spec: PaneSpec<ListSelectionModel>,
    ) -> Option<PaneId> {
        let (body, key_hints) = spec.into_parts();
        let pane = self.top_mut()?;
        pane.replace_list_selection(body, key_hints)
            .then_some(pane.id())
    }

    pub(crate) fn pop(&mut self) -> Option<PaneId> {
        self.entries.pop().map(|pane| pane.id())
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Option<(PaneId, PaneOutcome)> {
        let pane = self.top_mut()?;
        Some((pane.id(), pane.handle_key(key)))
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) -> bool {
        let Some(pane) = self.top_mut() else {
            return false;
        };
        pane.handle_paste(pasted);
        true
    }

    pub(crate) fn list_selection(&self) -> Option<&ListSelectionState> {
        self.top()?.list_selection()
    }

    pub(crate) fn select_visible_item(&mut self, index: usize) -> bool {
        self.top_mut()
            .and_then(Pane::list_selection_mut)
            .is_some_and(|body| body.select_visible_item(index))
    }

    pub(crate) fn select_tab(&mut self, index: usize) -> bool {
        self.top_mut()
            .and_then(Pane::list_selection_mut)
            .is_some_and(|body| body.select_tab(index))
    }

    pub(crate) fn activate_visible_item(
        &mut self,
        index: usize,
    ) -> Option<(PaneId, ListSelectionItemId)> {
        let pane = self.top_mut()?;
        let pane_id = pane.id();
        pane.list_selection_mut()?
            .activate_visible_item(index)
            .map(|item_id| (pane_id, item_id))
    }

    fn push(&mut self, body: PaneBody, key_hints: String) -> PaneId {
        let id = PaneId::new(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.entries.push(Pane::new(id, body, key_hints));
        id
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
