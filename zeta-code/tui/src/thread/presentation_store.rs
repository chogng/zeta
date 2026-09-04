use super::plan::PlanState;
use super::transcript::TranscriptCellId;
use crate::thread::composer::ChatInput;
use crate::thread::composer::ChatInputCatalog;
use crate::thread::composer::ChatInputMode;
use crate::thread::composer::SlashCommandCatalog;
use crate::thread::queue::Queue;
use crate::thread::transcript::ChatHistoryRenderCache;
use crate::thread::transcript::ChatHistoryScroll;
use crate::thread::transcript::TranscriptScrollAnchor;
use crate::thread::transcript::TranscriptScrollTarget;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::VecDeque;
use zeta_protocol::ThreadGoal;
use zeta_protocol::ThreadId;

const MAX_THREAD_PRESENTATIONS: usize = 32;

#[derive(Debug)]
pub(crate) struct ThreadPresentationState {
    pub(crate) input: ChatInput,
    pub(crate) goal: Option<ThreadGoal>,
    pub(crate) plan: PlanState,
    pub(crate) queue: Queue,
    pub(crate) scroll: ChatHistoryScroll,
    pub(crate) render_cache: ChatHistoryRenderCache,
    pub(crate) expanded_cells: BTreeSet<TranscriptCellId>,
    pub(crate) selected_cell: Option<TranscriptCellId>,
}

impl Default for ThreadPresentationState {
    fn default() -> Self {
        Self::with_input_catalog(ChatInputCatalog::default())
    }
}

impl ThreadPresentationState {
    fn with_input_catalog(catalog: ChatInputCatalog) -> Self {
        Self {
            input: ChatInput::with_catalog(catalog),
            goal: None,
            plan: PlanState::default(),
            queue: Queue::default(),
            scroll: ChatHistoryScroll::default(),
            render_cache: ChatHistoryRenderCache::default(),
            expanded_cells: BTreeSet::new(),
            selected_cell: None,
        }
    }

    pub(crate) fn toggle_cell(&mut self, cell_id: &TranscriptCellId) -> bool {
        if !self.expanded_cells.remove(cell_id) {
            self.expanded_cells.insert(cell_id.clone());
        }
        self.selected_cell = Some(cell_id.clone());
        self.scroll.apply(TranscriptScrollTarget::Anchor(
            TranscriptScrollAnchor::Cell {
                cell_id: cell_id.as_str().to_owned(),
                line_offset: 0,
            },
        ));
        self.expanded_cells.contains(cell_id)
    }

    pub(crate) fn select_previous_cell(&mut self, cell_ids: &[TranscriptCellId]) -> bool {
        let next = self
            .selected_cell
            .as_ref()
            .and_then(|selected| cell_ids.iter().position(|cell_id| cell_id == selected))
            .and_then(|index| index.checked_sub(1))
            .or_else(|| cell_ids.len().checked_sub(1));
        let Some(index) = next else {
            return false;
        };
        let cell_id = cell_ids[index].clone();
        self.scroll.apply(TranscriptScrollTarget::Anchor(
            TranscriptScrollAnchor::Cell {
                cell_id: cell_id.as_str().to_owned(),
                line_offset: 0,
            },
        ));
        self.selected_cell = Some(cell_id);
        true
    }

    pub(crate) fn select_next_cell(&mut self, cell_ids: &[TranscriptCellId]) -> bool {
        let next = self
            .selected_cell
            .as_ref()
            .and_then(|selected| cell_ids.iter().position(|cell_id| cell_id == selected))
            .map(|index| index.saturating_add(1))
            .unwrap_or_default();
        let Some(cell_id) = cell_ids.get(next).cloned() else {
            self.selected_cell = None;
            return false;
        };
        self.scroll.apply(TranscriptScrollTarget::Anchor(
            TranscriptScrollAnchor::Cell {
                cell_id: cell_id.as_str().to_owned(),
                line_offset: 0,
            },
        ));
        self.selected_cell = Some(cell_id);
        true
    }
}

#[derive(Debug)]
pub(crate) struct ThreadPresentationStore {
    active: ThreadId,
    input_mode: ChatInputMode,
    input_catalog: ChatInputCatalog,
    states: BTreeMap<ThreadId, ThreadPresentationState>,
    recent: VecDeque<ThreadId>,
}

impl ThreadPresentationStore {
    #[cfg(test)]
    pub(crate) fn new(active: ThreadId) -> Self {
        Self::with_input_catalog(active, ChatInputCatalog::default())
    }

    pub(crate) fn with_input_catalog(active: ThreadId, input_catalog: ChatInputCatalog) -> Self {
        let mut states = BTreeMap::new();
        states.insert(
            active.clone(),
            ThreadPresentationState::with_input_catalog(input_catalog.clone()),
        );
        Self {
            active: active.clone(),
            input_mode: ChatInputMode::Standard,
            input_catalog,
            states,
            recent: VecDeque::from([active]),
        }
    }

    pub(crate) fn switch(&mut self, thread_id: ThreadId) {
        if thread_id != self.active {
            self.active_mut().render_cache.clear();
        }
        let input_catalog = self.input_catalog.clone();
        self.states
            .entry(thread_id.clone())
            .or_insert_with(|| ThreadPresentationState::with_input_catalog(input_catalog))
            .input
            .set_input_mode(self.input_mode);
        self.active = thread_id.clone();
        self.touch(thread_id);
        self.evict_inactive();
    }

    pub(crate) fn replace_input_catalog(&mut self, input_catalog: ChatInputCatalog) {
        self.input_catalog = input_catalog.clone();
        for state in self.states.values_mut() {
            state.input.replace_catalog(input_catalog.clone());
        }
    }

    pub(crate) fn slash_commands(&self) -> &SlashCommandCatalog {
        self.input_catalog.slash_commands()
    }

    pub(crate) fn set_input_mode(&mut self, input_mode: ChatInputMode) {
        self.input_mode = input_mode;
        for state in self.states.values_mut() {
            state.input.set_input_mode(input_mode);
        }
    }

    pub(crate) fn active(&self) -> &ThreadPresentationState {
        self.states
            .get(&self.active)
            .expect("the active Thread presentation state exists")
    }

    pub(crate) fn active_mut(&mut self) -> &mut ThreadPresentationState {
        self.states
            .get_mut(&self.active)
            .expect("the active Thread presentation state exists")
    }

    fn touch(&mut self, thread_id: ThreadId) {
        self.recent.retain(|recent| recent != &thread_id);
        self.recent.push_back(thread_id);
    }

    fn evict_inactive(&mut self) {
        while self.states.len() > MAX_THREAD_PRESENTATIONS {
            let thread_id = self
                .recent
                .pop_front()
                .expect("every Thread presentation has a recency entry");
            if thread_id == self.active {
                self.recent.push_back(thread_id);
                continue;
            }
            self.states.remove(&thread_id);
        }
    }

    #[cfg(test)]
    pub(crate) fn contains(&self, thread_id: &ThreadId) -> bool {
        self.states.contains_key(thread_id)
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.states.len()
    }
}

#[cfg(test)]
#[path = "presentation_store_tests.rs"]
mod tests;
