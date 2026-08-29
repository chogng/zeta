use crate::components::approval::Approval;
use crate::components::approval::ApprovalDecision;
use crate::components::approval::ApprovalOutcome;
use crate::components::approval::ApprovalSpec;
use crate::components::approval::ApprovalView;
use crate::components::chat_input::ChatInput;
use crate::components::chat_input::ChatInputOutcome;
use crate::components::chat_input::ChatSubmission;
use crate::components::chat_input::MentionPluginItem;
use crate::components::chat_input::SkillSelectorItem;
use crate::components::chat_input::SlashCommandCatalog;
use crate::components::chat_input::SlashCommandInvocation;
use crate::components::chat_input::SuggestView;
use crate::components::detail_list::DetailList;
use crate::components::key_capture::KeyCapture;
use crate::components::list_selection::ListSelectionInputOutcome;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::list_selection::ListSelectionState;
use crate::components::pane::Pane;
use crate::components::pane::PaneId;
use crate::components::pane::PaneSpec;
use crate::components::pane::PaneView;
use crate::components::plan_progress::PlanProgress;
use crate::components::plan_progress::PlanProgressView;
use crate::components::query::Query;
use crate::components::query::QueryAnswer;
use crate::components::query::QueryOutcome;
use crate::components::query::QueryQuestion;
use crate::components::query::QueryView;
use crate::components::queue::Queue;
use crate::components::queue::QueueView;
use crate::components::steer::Steer;
use crate::components::steer::SteerId;
use crate::components::steer::SteerView;
use crate::components::text_prompt::TextPrompt;
use crate::components::text_prompt::TextPromptOutcome;
use crate::components::text_prompt::TextPromptSpec;
use crate::mouse::MouseMode;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use zeta_file_search::PathSearchSnapshot;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(crate) struct ChatInputAreaInteractionId(u64);

impl ChatInputAreaInteractionId {
    fn new(value: u64) -> Self {
        Self(value)
    }
}

pub(crate) enum ChatInputAreaOverlayView<'a> {
    Suggest(SuggestView<'a>),
    Approval(ApprovalView<'a>),
    Query(QueryView<'a>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChatInputAreaHeightEntryKind {
    Pane,
    PlanProgress,
    Queue,
    Steer,
}

#[derive(Debug)]
pub(crate) enum ChatInputAreaHeightEntryView<'a> {
    Pane(PaneEntryView<'a>),
    PlanProgress(PlanProgressView<'a>),
    Queue(QueueView<'a>),
    Steer(SteerView<'a>),
}

#[derive(Debug)]
pub(crate) enum PaneEntryView<'a> {
    DetailList(PaneView<'a, DetailList>),
    KeyCapture(PaneView<'a, KeyCapture>),
    ListSelection(PaneView<'a, ListSelectionState>),
    TextPrompt(PaneView<'a, TextPrompt>),
}

impl ChatInputAreaHeightEntryView<'_> {
    pub(crate) fn kind(&self) -> ChatInputAreaHeightEntryKind {
        match self {
            Self::Pane(_) => ChatInputAreaHeightEntryKind::Pane,
            Self::PlanProgress(_) => ChatInputAreaHeightEntryKind::PlanProgress,
            Self::Queue(_) => ChatInputAreaHeightEntryKind::Queue,
            Self::Steer(_) => ChatInputAreaHeightEntryKind::Steer,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ChatInputAreaOutcome {
    ActivateSelectionItem {
        pane_id: PaneId,
        item_id: ListSelectionItemId,
    },
    TextPromptSubmitted {
        pane_id: PaneId,
        value: String,
    },
    Command(SlashCommandInvocation),
    Consumed,
    ApprovalResponse {
        interaction_id: ChatInputAreaInteractionId,
        decision: ApprovalDecision,
    },
    QueryResponse {
        interaction_id: ChatInputAreaInteractionId,
        answers: Vec<QueryAnswer>,
    },
    Queue(ChatSubmission),
    SubmissionRejected(String),
    Submit(ChatSubmission),
    Unhandled,
    PaneDismissed(PaneId),
}

/// Owns focus and routing for the persistent chat input and pages above it.
///
/// The chat input remains alive while pages are stacked above it, preserving draft state when a
/// page is dismissed. Product feature state remains outside this component.
#[derive(Debug)]
pub(crate) struct ChatInputArea {
    chat_input: ChatInput,
    panes: Vec<PaneEntry>,
    next_pane_id: u64,
    height_order: Vec<ChatInputAreaHeightEntryKind>,
    plan_progress: Option<PlanProgress>,
    queue: Queue,
    steer: Steer,
    interaction: Option<AgentInteraction>,
    next_interaction_id: u64,
}

#[derive(Debug)]
enum PaneEntry {
    DetailList {
        id: PaneId,
        pane: Pane<DetailList>,
    },
    KeyCapture {
        id: PaneId,
        pane: Pane<KeyCapture>,
    },
    ListSelection {
        id: PaneId,
        pane: Pane<ListSelectionState>,
    },
    TextPrompt {
        id: PaneId,
        pane: Pane<TextPrompt>,
    },
}

#[derive(Debug)]
enum AgentInteraction {
    Approval {
        id: ChatInputAreaInteractionId,
        state: Approval,
    },
    Query {
        id: ChatInputAreaInteractionId,
        state: Query,
    },
}

impl ChatInputArea {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            chat_input: ChatInput::new(),
            panes: Vec::new(),
            next_pane_id: 1,
            height_order: Vec::new(),
            plan_progress: None,
            queue: Queue::default(),
            steer: Steer::default(),
            interaction: None,
            next_interaction_id: 1,
        }
    }

    pub(crate) fn with_slash_commands(slash_commands: SlashCommandCatalog) -> Self {
        Self {
            chat_input: ChatInput::with_slash_commands(slash_commands),
            panes: Vec::new(),
            next_pane_id: 1,
            height_order: Vec::new(),
            plan_progress: None,
            queue: Queue::default(),
            steer: Steer::default(),
            interaction: None,
            next_interaction_id: 1,
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> ChatInputAreaOutcome {
        self.handle_key_with_submission_target(key, SubmissionTarget::StartTurn)
    }

    pub(crate) fn handle_active_turn_key(&mut self, key: KeyEvent) -> ChatInputAreaOutcome {
        self.handle_key_with_submission_target(key, SubmissionTarget::SteerTurn)
    }

    fn handle_key_with_submission_target(
        &mut self,
        key: KeyEvent,
        submission_target: SubmissionTarget,
    ) -> ChatInputAreaOutcome {
        if self.chat_input.query_answer_active() {
            return match self.chat_input.handle_key(key) {
                ChatInputOutcome::QueryAnswerCancelled => ChatInputAreaOutcome::Consumed,
                ChatInputOutcome::QueryAnswerSubmitted(value) => {
                    let Some(AgentInteraction::Query { id, state }) = self.interaction.as_mut()
                    else {
                        return ChatInputAreaOutcome::Unhandled;
                    };
                    let interaction_id = *id;
                    let outcome = state.submit_custom_answer(value);
                    map_query_outcome(interaction_id, outcome, &mut self.chat_input)
                }
                outcome => map_chat_input_outcome(outcome),
            };
        }
        if let Some(interaction) = self.interaction.as_mut() {
            return match interaction {
                AgentInteraction::Approval { id, state } => match state.handle_key(key) {
                    ApprovalOutcome::Consumed => ChatInputAreaOutcome::Consumed,
                    ApprovalOutcome::Respond(decision) => ChatInputAreaOutcome::ApprovalResponse {
                        interaction_id: *id,
                        decision,
                    },
                    ApprovalOutcome::Unhandled => ChatInputAreaOutcome::Unhandled,
                },
                AgentInteraction::Query { id, state } => {
                    let interaction_id = *id;
                    let outcome = state.handle_key(key);
                    map_query_outcome(interaction_id, outcome, &mut self.chat_input)
                }
            };
        }
        if let Some(entry) = self.panes.last_mut() {
            let (pane_id, outcome) = match entry {
                PaneEntry::DetailList { id, .. } => {
                    let dismiss = key.code == KeyCode::Esc
                        || (key.modifiers.contains(KeyModifiers::CONTROL)
                            && key.code == KeyCode::Char('c'));
                    (*id, dismiss.then_some(PaneInputOutcome::Dismiss))
                }
                PaneEntry::KeyCapture { id, .. } => (*id, Some(PaneInputOutcome::Unhandled)),
                PaneEntry::ListSelection { id, pane } => {
                    let outcome = match pane.body_mut().handle_key(key) {
                        ListSelectionInputOutcome::Activate(item_id) => {
                            PaneInputOutcome::ActivateSelection(item_id)
                        }
                        ListSelectionInputOutcome::Consumed => PaneInputOutcome::Consumed,
                        ListSelectionInputOutcome::Dismiss => PaneInputOutcome::Dismiss,
                    };
                    (*id, Some(outcome))
                }
                PaneEntry::TextPrompt { id, pane } => {
                    let outcome = match pane.body_mut().handle_key(key) {
                        TextPromptOutcome::Consumed => PaneInputOutcome::Consumed,
                        TextPromptOutcome::Dismiss => PaneInputOutcome::Dismiss,
                        TextPromptOutcome::Submit(value) => PaneInputOutcome::SubmitText(value),
                    };
                    (*id, Some(outcome))
                }
            };
            return match outcome.unwrap_or(PaneInputOutcome::Consumed) {
                PaneInputOutcome::ActivateSelection(item_id) => {
                    ChatInputAreaOutcome::ActivateSelectionItem { pane_id, item_id }
                }
                PaneInputOutcome::SubmitText(value) => {
                    ChatInputAreaOutcome::TextPromptSubmitted { pane_id, value }
                }
                PaneInputOutcome::Consumed => ChatInputAreaOutcome::Consumed,
                PaneInputOutcome::Unhandled => ChatInputAreaOutcome::Unhandled,
                PaneInputOutcome::Dismiss => {
                    self.pop_pane();
                    ChatInputAreaOutcome::PaneDismissed(pane_id)
                }
            };
        }
        if submission_target == SubmissionTarget::SteerTurn
            && key.code == KeyCode::Tab
            && key.modifiers.is_empty()
            && self.suggest().is_none()
        {
            return map_queued_chat_input_outcome(self.chat_input.submit_current());
        }
        if submission_target == SubmissionTarget::SteerTurn
            && key.code == KeyCode::Enter
            && key.modifiers.is_empty()
            && self.suggest().is_none()
            && self.chat_input.submission_contains_skill()
        {
            return ChatInputAreaOutcome::SubmissionRejected(
                "A running Turn cannot change its Skill; press Tab to queue this message for the next Turn"
                    .into(),
            );
        }
        map_chat_input_outcome(self.chat_input.handle_key(key))
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, text: &str) {
        self.chat_input.insert_text(text);
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) -> Result<(), String> {
        if self.chat_input.query_answer_active() {
            return self.chat_input.handle_paste(pasted);
        }
        if self.interaction.is_some() {
            return Ok(());
        }
        if let Some(entry) = self.panes.last_mut() {
            match entry {
                PaneEntry::ListSelection { pane, .. } => pane.body_mut().handle_paste(pasted),
                PaneEntry::TextPrompt { pane, .. } => pane.body_mut().handle_paste(pasted),
                PaneEntry::DetailList { .. } | PaneEntry::KeyCapture { .. } => {}
            }
            return Ok(());
        }
        self.chat_input.handle_paste(pasted)
    }

    pub(crate) fn attach_image_bytes(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        if self.interaction.is_some() {
            return Err("images are unavailable while an Agent interaction is active".into());
        }
        self.chat_input.attach_image_bytes(bytes)
    }

    pub(crate) fn replace_chat_input_catalog(
        &mut self,
        slash_commands: SlashCommandCatalog,
        skills: Vec<SkillSelectorItem>,
        plugins: Vec<MentionPluginItem>,
    ) {
        self.chat_input
            .replace_chat_input_catalog(slash_commands, skills, plugins);
    }

    pub(crate) fn text(&self) -> &str {
        self.chat_input.text()
    }

    pub(crate) fn cursor_display_width(&self) -> usize {
        self.chat_input.cursor_display_width()
    }

    pub(crate) fn cursor_line(&self) -> usize {
        self.chat_input.cursor_line()
    }

    pub(crate) fn chat_input_desired_height(&self, available_width: u16) -> u16 {
        self.chat_input.desired_height(available_width)
    }

    pub(crate) fn suggest(&self) -> Option<SuggestView<'_>> {
        if self.interaction.is_some()
            || self.chat_input.query_answer_active()
            || !self.panes.is_empty()
        {
            return None;
        }
        self.chat_input
            .slash_popup()
            .map(SuggestView::Slash)
            .or_else(|| self.chat_input.mention_popup().map(SuggestView::Mention))
            .or_else(|| self.chat_input.skill_popup().map(SuggestView::Skill))
    }

    pub(crate) fn mouse_mode(&self) -> MouseMode {
        if self.interaction.is_some() {
            return MouseMode::UiClick;
        }
        if self.panes.last().is_some_and(|entry| match entry {
            PaneEntry::ListSelection { pane, .. } => pane.body().mouse_mode() == MouseMode::UiClick,
            PaneEntry::DetailList { .. }
            | PaneEntry::KeyCapture { .. }
            | PaneEntry::TextPrompt { .. } => false,
        }) || self.suggest().is_some()
            || self.plan_progress.is_some()
        {
            MouseMode::UiClick
        } else {
            MouseMode::TerminalSelection
        }
    }

    pub(crate) fn mention_query(&self) -> Option<&str> {
        if !self.panes.is_empty() {
            return None;
        }
        self.chat_input.mention_query()
    }

    pub(crate) fn apply_file_search_snapshot(&mut self, snapshot: PathSearchSnapshot) {
        self.chat_input.apply_file_search_snapshot(snapshot);
    }

    pub(crate) fn overlay(&self) -> Option<ChatInputAreaOverlayView<'_>> {
        match self.interaction.as_ref() {
            Some(AgentInteraction::Approval { state, .. }) => {
                Some(ChatInputAreaOverlayView::Approval(state.view()))
            }
            Some(AgentInteraction::Query { state, .. }) => {
                Some(ChatInputAreaOverlayView::Query(state.view()))
            }
            None => self.suggest().map(ChatInputAreaOverlayView::Suggest),
        }
    }

    pub(crate) fn show_approval(
        &mut self,
        spec: ApprovalSpec,
    ) -> Result<ChatInputAreaInteractionId, String> {
        self.ensure_interaction_slot_available()?;
        let id = self.allocate_interaction_id();
        self.interaction = Some(AgentInteraction::Approval {
            id,
            state: Approval::new(spec),
        });
        Ok(id)
    }

    pub(crate) fn show_query(
        &mut self,
        questions: Vec<QueryQuestion>,
    ) -> Result<ChatInputAreaInteractionId, String> {
        self.ensure_interaction_slot_available()?;
        let state = Query::new(questions)?;
        let id = self.allocate_interaction_id();
        self.interaction = Some(AgentInteraction::Query { id, state });
        Ok(id)
    }

    pub(crate) fn resolve_interaction(&mut self, expected: ChatInputAreaInteractionId) -> bool {
        let matches = self
            .interaction
            .as_ref()
            .is_some_and(|interaction| interaction.id() == expected);
        if matches {
            self.interaction = None;
        }
        matches
    }

    pub(crate) fn interaction_submission_failed(
        &mut self,
        expected: ChatInputAreaInteractionId,
        error: String,
    ) -> bool {
        let Some(interaction) = self.interaction.as_mut() else {
            return false;
        };
        if interaction.id() != expected {
            return false;
        }
        match interaction {
            AgentInteraction::Approval { state, .. } => state.submission_failed(error),
            AgentInteraction::Query { state, .. } => state.submission_failed(error),
        }
        true
    }

    pub(crate) fn query_answer_active(&self) -> bool {
        self.chat_input.query_answer_active()
    }

    pub(crate) fn interaction_active(&self) -> bool {
        self.interaction.is_some()
    }

    pub(crate) fn pane_active(&self) -> bool {
        !self.panes.is_empty()
    }

    pub(crate) fn activate_suggest(&mut self, index: usize) -> Option<ChatInputAreaOutcome> {
        if !self.panes.is_empty() {
            return None;
        }
        match self.suggest()? {
            SuggestView::Slash(_) => self
                .chat_input
                .activate_slash_command(index)
                .map(map_chat_input_outcome),
            SuggestView::Mention(_) => self
                .chat_input
                .activate_mention(index)
                .then_some(ChatInputAreaOutcome::Consumed),
            SuggestView::Skill(_) => self
                .chat_input
                .activate_skill(index)
                .then_some(ChatInputAreaOutcome::Consumed),
        }
    }

    pub(crate) fn select_suggest(&mut self, index: usize) -> bool {
        if !self.panes.is_empty() {
            return false;
        }
        match self.suggest() {
            Some(SuggestView::Slash(_)) => self.chat_input.select_slash_command(index),
            Some(SuggestView::Mention(_)) => self.chat_input.select_mention(index),
            Some(SuggestView::Skill(_)) => self.chat_input.select_skill(index),
            None => false,
        }
    }

    pub(crate) fn select_overlay_choice(&mut self, index: usize) -> bool {
        match self.interaction.as_mut() {
            Some(AgentInteraction::Approval { state, .. }) => state.select(index),
            Some(AgentInteraction::Query { state, .. }) => state.select(index),
            None => self.select_suggest(index),
        }
    }

    pub(crate) fn activate_overlay_choice(&mut self, index: usize) -> Option<ChatInputAreaOutcome> {
        match self.interaction.as_mut() {
            Some(AgentInteraction::Approval { id, state }) => {
                let decision = match state.activate(index)? {
                    ApprovalOutcome::Respond(decision) => decision,
                    ApprovalOutcome::Consumed | ApprovalOutcome::Unhandled => return None,
                };
                Some(ChatInputAreaOutcome::ApprovalResponse {
                    interaction_id: *id,
                    decision,
                })
            }
            Some(AgentInteraction::Query { id, state }) => {
                let interaction_id = *id;
                let outcome = state.activate(index)?;
                Some(map_query_outcome(
                    interaction_id,
                    outcome,
                    &mut self.chat_input,
                ))
            }
            None => self.activate_suggest(index),
        }
    }

    pub(crate) fn select_visible_item(&mut self, index: usize) -> bool {
        match self.panes.last_mut() {
            Some(PaneEntry::ListSelection { pane, .. }) => {
                pane.body_mut().select_visible_item(index)
            }
            Some(_) => false,
            None => false,
        }
    }

    pub(crate) fn select_tab(&mut self, index: usize) -> bool {
        match self.panes.last_mut() {
            Some(PaneEntry::ListSelection { pane, .. }) => pane.body_mut().select_tab(index),
            Some(_) => false,
            None => false,
        }
    }

    pub(crate) fn activate_visible_item(&mut self, index: usize) -> Option<ChatInputAreaOutcome> {
        let Some(PaneEntry::ListSelection { id, pane }) = self.panes.last_mut() else {
            return None;
        };
        let pane_id = *id;
        pane.body_mut()
            .activate_visible_item(index)
            .map(|item_id| ChatInputAreaOutcome::ActivateSelectionItem { pane_id, item_id })
    }

    pub(crate) fn push_list_selection(&mut self, model: PaneSpec<ListSelectionModel>) -> PaneId {
        let (body, key_hints) = model.into_parts();
        let pane_id = self.allocate_pane_id();
        self.panes.push(PaneEntry::ListSelection {
            id: pane_id,
            pane: Pane::new(ListSelectionState::new(body), key_hints),
        });
        self.ensure_height_entry(ChatInputAreaHeightEntryKind::Pane);
        pane_id
    }

    pub(crate) fn push_detail_list(&mut self, spec: PaneSpec<DetailList>) -> PaneId {
        let (body, key_hints) = spec.into_parts();
        let pane_id = self.allocate_pane_id();
        self.panes.push(PaneEntry::DetailList {
            id: pane_id,
            pane: Pane::new(body, key_hints),
        });
        self.ensure_height_entry(ChatInputAreaHeightEntryKind::Pane);
        pane_id
    }

    pub(crate) fn push_text_prompt(&mut self, spec: PaneSpec<TextPromptSpec>) -> PaneId {
        let (body, key_hints) = spec.into_parts();
        let pane_id = self.allocate_pane_id();
        self.panes.push(PaneEntry::TextPrompt {
            id: pane_id,
            pane: Pane::new(TextPrompt::new(body), key_hints),
        });
        self.ensure_height_entry(ChatInputAreaHeightEntryKind::Pane);
        pane_id
    }

    pub(crate) fn push_key_capture(&mut self, spec: PaneSpec<KeyCapture>) -> PaneId {
        let (body, key_hints) = spec.into_parts();
        let pane_id = self.allocate_pane_id();
        self.panes.push(PaneEntry::KeyCapture {
            id: pane_id,
            pane: Pane::new(body, key_hints),
        });
        self.ensure_height_entry(ChatInputAreaHeightEntryKind::Pane);
        pane_id
    }

    pub(crate) fn update_top_key_capture(&mut self, spec: PaneSpec<KeyCapture>) -> Option<PaneId> {
        let (body, key_hints) = spec.into_parts();
        let Some(PaneEntry::KeyCapture { id, pane }) = self.panes.last_mut() else {
            return None;
        };
        *pane.body_mut() = body;
        pane.replace_key_hints(key_hints);
        Some(*id)
    }

    pub(crate) fn update_top_list_selection(
        &mut self,
        model: PaneSpec<ListSelectionModel>,
    ) -> Option<PaneId> {
        let (body, key_hints) = model.into_parts();
        let Some(PaneEntry::ListSelection { id, pane }) = self.panes.last_mut() else {
            return None;
        };
        pane.body_mut().replace_model(body);
        pane.replace_key_hints(key_hints);
        Some(*id)
    }

    pub(crate) fn pop_pane(&mut self) -> Option<PaneId> {
        let pane_id = self.panes.pop().map(|entry| entry.id());
        if self.panes.is_empty() {
            self.remove_height_entry(ChatInputAreaHeightEntryKind::Pane);
        }
        pane_id
    }

    pub(crate) fn list_selection(&self) -> Option<&ListSelectionState> {
        match self.panes.last() {
            Some(PaneEntry::ListSelection { pane, .. }) => Some(pane.body()),
            Some(_) => None,
            None => None,
        }
    }

    pub(crate) fn list_selection_pane(&self) -> Option<PaneView<'_, ListSelectionState>> {
        match self.panes.last() {
            Some(PaneEntry::ListSelection { pane, .. }) => Some(pane.view()),
            Some(_) => None,
            None => None,
        }
    }

    pub(crate) fn top_pane_id(&self) -> Option<PaneId> {
        self.panes.last().map(PaneEntry::id)
    }

    pub(crate) fn replace_turn_status(
        &mut self,
        plan: Option<zeta_protocol::PlanUpdate>,
        queued_turns: Vec<String>,
    ) {
        match plan {
            Some(plan) => {
                if let Some(progress) = self.plan_progress.as_mut() {
                    progress.replace(plan);
                } else {
                    self.plan_progress = Some(PlanProgress::new(plan));
                    self.ensure_height_entry(ChatInputAreaHeightEntryKind::PlanProgress);
                }
                if self
                    .plan_progress
                    .as_ref()
                    .is_some_and(PlanProgress::is_complete)
                {
                    self.plan_progress = None;
                    self.remove_height_entry(ChatInputAreaHeightEntryKind::PlanProgress);
                }
            }
            None => {
                self.plan_progress = None;
                self.remove_height_entry(ChatInputAreaHeightEntryKind::PlanProgress);
            }
        }

        self.queue.replace(queued_turns);
        if self.queue.is_empty() {
            self.remove_height_entry(ChatInputAreaHeightEntryKind::Queue);
        } else {
            self.ensure_height_entry(ChatInputAreaHeightEntryKind::Queue);
        }
    }

    pub(crate) fn begin_steer(&mut self, text: String) -> SteerId {
        let id = self.steer.push(text);
        self.ensure_height_entry(ChatInputAreaHeightEntryKind::Steer);
        id
    }

    pub(crate) fn finish_steer(&mut self, id: SteerId) -> bool {
        let removed = self.steer.remove(id);
        if self.steer.is_empty() {
            self.remove_height_entry(ChatInputAreaHeightEntryKind::Steer);
        }
        removed
    }

    pub(crate) fn clear_steers(&mut self) {
        self.steer.clear();
        self.remove_height_entry(ChatInputAreaHeightEntryKind::Steer);
    }

    pub(crate) fn height_entries(&self) -> Vec<ChatInputAreaHeightEntryView<'_>> {
        self.height_order
            .iter()
            .filter_map(|kind| match kind {
                ChatInputAreaHeightEntryKind::Pane => self
                    .panes
                    .last()
                    .map(PaneEntry::view)
                    .map(ChatInputAreaHeightEntryView::Pane),
                ChatInputAreaHeightEntryKind::PlanProgress => self
                    .plan_progress
                    .as_ref()
                    .map(|progress| ChatInputAreaHeightEntryView::PlanProgress(progress.view())),
                ChatInputAreaHeightEntryKind::Queue => (!self.queue.is_empty())
                    .then(|| ChatInputAreaHeightEntryView::Queue(self.queue.view())),
                ChatInputAreaHeightEntryKind::Steer => (!self.steer.is_empty())
                    .then(|| ChatInputAreaHeightEntryView::Steer(self.steer.view())),
            })
            .collect()
    }

    pub(crate) fn toggle_plan_progress(&mut self) -> bool {
        let Some(progress) = self.plan_progress.as_mut() else {
            return false;
        };
        progress.toggle_expanded();
        true
    }

    fn ensure_height_entry(&mut self, kind: ChatInputAreaHeightEntryKind) {
        if !self.height_order.contains(&kind) {
            self.height_order.push(kind);
        }
    }

    fn remove_height_entry(&mut self, kind: ChatInputAreaHeightEntryKind) {
        self.height_order.retain(|entry| *entry != kind);
    }

    fn ensure_interaction_slot_available(&self) -> Result<(), String> {
        if self.interaction.is_some() {
            Err("an Agent interaction is already active".into())
        } else {
            Ok(())
        }
    }

    fn allocate_interaction_id(&mut self) -> ChatInputAreaInteractionId {
        let id = ChatInputAreaInteractionId::new(self.next_interaction_id);
        self.next_interaction_id = self.next_interaction_id.saturating_add(1);
        id
    }

    fn allocate_pane_id(&mut self) -> PaneId {
        let id = PaneId::new(self.next_pane_id);
        self.next_pane_id = self.next_pane_id.saturating_add(1);
        id
    }
}

impl PaneEntry {
    fn id(&self) -> PaneId {
        match self {
            Self::DetailList { id, .. }
            | Self::KeyCapture { id, .. }
            | Self::ListSelection { id, .. }
            | Self::TextPrompt { id, .. } => *id,
        }
    }

    fn view(&self) -> PaneEntryView<'_> {
        match self {
            Self::DetailList { pane, .. } => PaneEntryView::DetailList(pane.view()),
            Self::KeyCapture { pane, .. } => PaneEntryView::KeyCapture(pane.view()),
            Self::ListSelection { pane, .. } => PaneEntryView::ListSelection(pane.view()),
            Self::TextPrompt { pane, .. } => PaneEntryView::TextPrompt(pane.view()),
        }
    }
}

enum PaneInputOutcome {
    ActivateSelection(ListSelectionItemId),
    SubmitText(String),
    Consumed,
    Dismiss,
    Unhandled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SubmissionTarget {
    StartTurn,
    SteerTurn,
}

impl AgentInteraction {
    fn id(&self) -> ChatInputAreaInteractionId {
        match self {
            Self::Approval { id, .. } | Self::Query { id, .. } => *id,
        }
    }
}

fn map_query_outcome(
    interaction_id: ChatInputAreaInteractionId,
    outcome: QueryOutcome,
    chat_input: &mut ChatInput,
) -> ChatInputAreaOutcome {
    match outcome {
        QueryOutcome::BeginCustomAnswer => {
            chat_input.begin_query_answer();
            ChatInputAreaOutcome::Consumed
        }
        QueryOutcome::Completed(answers) => ChatInputAreaOutcome::QueryResponse {
            interaction_id,
            answers,
        },
        QueryOutcome::Consumed => ChatInputAreaOutcome::Consumed,
        QueryOutcome::Unhandled => ChatInputAreaOutcome::Unhandled,
    }
}

fn map_chat_input_outcome(outcome: ChatInputOutcome) -> ChatInputAreaOutcome {
    match outcome {
        ChatInputOutcome::Command(command) => ChatInputAreaOutcome::Command(command),
        ChatInputOutcome::Consumed => ChatInputAreaOutcome::Consumed,
        ChatInputOutcome::QueryAnswerCancelled | ChatInputOutcome::QueryAnswerSubmitted(_) => {
            ChatInputAreaOutcome::Unhandled
        }
        ChatInputOutcome::Submit(prompt) => ChatInputAreaOutcome::Submit(prompt),
        ChatInputOutcome::Unhandled => ChatInputAreaOutcome::Unhandled,
    }
}

fn map_queued_chat_input_outcome(outcome: ChatInputOutcome) -> ChatInputAreaOutcome {
    match outcome {
        ChatInputOutcome::Submit(prompt) => ChatInputAreaOutcome::Queue(prompt),
        outcome => map_chat_input_outcome(outcome),
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
