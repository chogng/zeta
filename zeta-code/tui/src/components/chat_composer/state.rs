use crate::components::chat_input::ChatInput;
use crate::components::chat_input::ChatInputOutcome;
use crate::components::chat_input::ChatInputQueueOutcome;
use crate::components::chat_input::ChatSubmission;
use crate::components::chat_input::CompletionView;
use crate::components::chat_input::QueuedChatInput;
use crate::components::chat_input::SlashCommandInvocation;
use crate::components::key_capture::KeyCapture;
use crate::components::list_selection::ListSelectionAdjustment;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::list_selection::ListSelectionState;
use crate::components::pane::PaneId;
use crate::components::pane::PaneOutcome;
use crate::components::pane::PaneSpec;
use crate::components::pane::PaneStack;
use crate::components::pane::PaneView;
use crate::components::steer::Steer;
use crate::components::steer::SteerId;
use crate::components::text_prompt::TextPromptSpec;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;

pub(crate) struct ChatComposerView<'a> {
    state: &'a ChatComposer,
    input: &'a ChatInput,
}

impl ChatComposerView<'_> {
    pub(super) fn input(&self) -> &str {
        self.input.text()
    }

    pub(super) fn input_cursor_width(&self) -> usize {
        self.input.cursor_display_width()
    }

    pub(super) fn input_cursor_line(&self) -> usize {
        self.input.cursor_line()
    }

    pub(super) fn input_prompt(&self) -> &'static str {
        self.input.prompt()
    }

    pub(super) fn input_desired_height(&self, available_width: u16) -> u16 {
        self.input.desired_height(available_width)
    }

    pub(super) fn pane_views(&self) -> Vec<ChatComposerPaneView<'_>> {
        self.state.pane_views()
    }

    pub(super) fn input_completion(&self) -> Option<CompletionView<'_>> {
        self.state
            .panes
            .is_empty()
            .then(|| self.input.completion())
            .flatten()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ChatComposerPaneKind {
    Stacked,
}

#[derive(Debug)]
pub(crate) enum ChatComposerPaneView<'a> {
    Stacked(PaneView<'a>),
}

impl ChatComposerPaneView<'_> {
    pub(crate) fn kind(&self) -> ChatComposerPaneKind {
        match self {
            Self::Stacked(_) => ChatComposerPaneKind::Stacked,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum ChatComposerOutcome {
    ActivateSelectionItem {
        pane_id: PaneId,
        item_id: ListSelectionItemId,
    },
    AdjustSelectionItem {
        pane_id: PaneId,
        item_id: ListSelectionItemId,
        adjustment: ListSelectionAdjustment,
    },
    PaneKeyCaptured {
        pane_id: PaneId,
        key: KeyEvent,
    },
    TextPromptSubmitted {
        pane_id: PaneId,
        value: String,
    },
    Command(SlashCommandInvocation),
    Consumed,
    SubmissionRejected(String),
    Queued(QueuedChatInput),
    Submit(ChatSubmission),
    Unhandled,
    PaneDismissed(PaneId),
}

/// Routes input between the caller-owned persistent chat input and pages above it.
///
/// The chat input remains alive while pages are stacked above it, preserving draft state when a
/// page is dismissed. Product feature state remains outside this component.
#[derive(Debug)]
pub(crate) struct ChatComposer {
    panes: PaneStack,
    pane_order: Vec<ChatComposerPaneKind>,
    steer: Steer,
}

impl ChatComposer {
    pub(crate) fn new() -> Self {
        Self {
            panes: PaneStack::default(),
            pane_order: Vec::new(),
            steer: Steer::default(),
        }
    }

    pub(crate) fn handle_key(
        &mut self,
        input: &mut ChatInput,
        key: KeyEvent,
    ) -> ChatComposerOutcome {
        self.handle_key_with_submission_target(input, key, SubmissionTarget::Start)
    }

    pub(crate) fn handle_active_turn_key(
        &mut self,
        input: &mut ChatInput,
        key: KeyEvent,
    ) -> ChatComposerOutcome {
        self.handle_key_with_submission_target(input, key, SubmissionTarget::Steer)
    }

    pub(crate) fn handle_queued_turn_key(
        &mut self,
        input: &mut ChatInput,
        key: KeyEvent,
    ) -> ChatComposerOutcome {
        self.handle_key_with_submission_target(input, key, SubmissionTarget::Queue)
    }

    fn handle_key_with_submission_target(
        &mut self,
        input: &mut ChatInput,
        key: KeyEvent,
        submission_target: SubmissionTarget,
    ) -> ChatComposerOutcome {
        if let Some((pane_id, outcome)) = self.panes.handle_key(key) {
            return match outcome {
                PaneOutcome::ActivateSelection(item_id) => {
                    ChatComposerOutcome::ActivateSelectionItem { pane_id, item_id }
                }
                PaneOutcome::AdjustSelection(item_id, adjustment) => {
                    ChatComposerOutcome::AdjustSelectionItem {
                        pane_id,
                        item_id,
                        adjustment,
                    }
                }
                PaneOutcome::KeyCaptured(key) => {
                    ChatComposerOutcome::PaneKeyCaptured { pane_id, key }
                }
                PaneOutcome::SubmitText(value) => {
                    ChatComposerOutcome::TextPromptSubmitted { pane_id, value }
                }
                PaneOutcome::Consumed => ChatComposerOutcome::Consumed,
                PaneOutcome::Dismiss => {
                    self.pop_pane();
                    ChatComposerOutcome::PaneDismissed(pane_id)
                }
            };
        }
        if submission_target == SubmissionTarget::Queue
            && key.code == KeyCode::Enter
            && key.modifiers.is_empty()
            && input.completion().is_none()
            && input.accepts_submission_key()
        {
            return self.queue_current_input(input);
        }
        if submission_target == SubmissionTarget::Steer
            && key.code == KeyCode::Enter
            && key.modifiers.is_empty()
            && input.completion().is_none()
            && input.accepts_submission_key()
            && input.submission_contains_skill()
        {
            return ChatComposerOutcome::SubmissionRejected(
                "A running Turn cannot change its Skill; switch follow-up messages to Queue or wait for the next Turn"
                    .into(),
            );
        }
        map_chat_input_outcome(input.handle_key(key))
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, input: &mut ChatInput, text: &str) {
        input.insert_text(text);
    }

    pub(crate) fn handle_paste(
        &mut self,
        input: &mut ChatInput,
        pasted: String,
    ) -> Result<(), String> {
        if !self.panes.is_empty() {
            self.panes.handle_paste(pasted);
            return Ok(());
        }
        input.handle_paste(pasted)?;
        Ok(())
    }

    pub(crate) fn attach_image_bytes(
        &mut self,
        input: &mut ChatInput,
        bytes: Vec<u8>,
    ) -> Result<(), String> {
        input.attach_image_bytes(bytes)?;
        Ok(())
    }

    pub(crate) fn view<'a>(&'a self, input: &'a ChatInput) -> ChatComposerView<'a> {
        ChatComposerView { state: self, input }
    }

    pub(crate) fn pane_active(&self) -> bool {
        !self.panes.is_empty()
    }

    pub(crate) fn pane_key_hints(&self) -> Option<&str> {
        self.panes.top_key_hints()
    }

    pub(crate) fn activate_completion(
        &mut self,
        input: &mut ChatInput,
        index: usize,
    ) -> Option<ChatComposerOutcome> {
        if !self.panes.is_empty() {
            return None;
        }
        input.activate_completion(index).map(map_chat_input_outcome)
    }

    pub(crate) fn select_tab(&mut self, index: usize) -> bool {
        self.panes.select_tab(index)
    }

    pub(crate) fn focus_search(&mut self) -> bool {
        self.panes.focus_search()
    }

    pub(crate) fn activate_visible_item(&mut self, index: usize) -> Option<ChatComposerOutcome> {
        self.panes
            .activate_visible_item(index)
            .map(
                |(pane_id, item_id)| ChatComposerOutcome::ActivateSelectionItem {
                    pane_id,
                    item_id,
                },
            )
    }

    pub(crate) fn push_list_selection(&mut self, model: PaneSpec<ListSelectionModel>) -> PaneId {
        let pane_id = self.panes.push_list_selection(model);
        self.ensure_pane(ChatComposerPaneKind::Stacked);
        pane_id
    }

    pub(crate) fn push_text_prompt(&mut self, spec: PaneSpec<TextPromptSpec>) -> PaneId {
        let pane_id = self.panes.push_text_prompt(spec);
        self.ensure_pane(ChatComposerPaneKind::Stacked);
        pane_id
    }

    pub(crate) fn push_key_capture(&mut self, spec: PaneSpec<KeyCapture>) -> PaneId {
        let pane_id = self.panes.push_key_capture(spec);
        self.ensure_pane(ChatComposerPaneKind::Stacked);
        pane_id
    }

    pub(crate) fn update_top_key_capture(&mut self, spec: PaneSpec<KeyCapture>) -> Option<PaneId> {
        self.panes.update_top_key_capture(spec)
    }

    pub(crate) fn update_top_list_selection(
        &mut self,
        model: PaneSpec<ListSelectionModel>,
    ) -> Option<PaneId> {
        self.panes.update_top_list_selection(model)
    }

    pub(crate) fn pop_pane(&mut self) -> Option<PaneId> {
        let pane_id = self.panes.pop();
        if self.panes.is_empty() {
            self.remove_pane(ChatComposerPaneKind::Stacked);
        }
        pane_id
    }

    pub(crate) fn list_selection(&self) -> Option<&ListSelectionState> {
        self.panes.list_selection()
    }

    pub(crate) fn top_pane_id(&self) -> Option<PaneId> {
        self.panes.top_id()
    }

    pub(crate) fn begin_steer(&mut self, text: String) -> SteerId {
        self.steer.push(text)
    }

    pub(crate) fn finish_steer(&mut self, id: SteerId) -> bool {
        self.steer.remove(id)
    }

    pub(crate) fn clear_steers(&mut self) {
        self.steer.clear();
    }

    pub(crate) fn pane_views(&self) -> Vec<ChatComposerPaneView<'_>> {
        self.pane_order
            .iter()
            .filter_map(|kind| match kind {
                ChatComposerPaneKind::Stacked => {
                    self.panes.top_view().map(ChatComposerPaneView::Stacked)
                }
            })
            .collect()
    }

    fn ensure_pane(&mut self, kind: ChatComposerPaneKind) {
        if !self.pane_order.contains(&kind) {
            self.pane_order.push(kind);
        }
    }

    fn remove_pane(&mut self, kind: ChatComposerPaneKind) {
        self.pane_order.retain(|entry| *entry != kind);
    }

    fn queue_current_input(&mut self, input: &mut ChatInput) -> ChatComposerOutcome {
        let outcome = input.queue_current();
        match outcome {
            ChatInputQueueOutcome::Command(command) => ChatComposerOutcome::Command(command),
            ChatInputQueueOutcome::Consumed => ChatComposerOutcome::Consumed,
            ChatInputQueueOutcome::Queued(input) => ChatComposerOutcome::Queued(input),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SubmissionTarget {
    Start,
    Queue,
    Steer,
}

fn map_chat_input_outcome(outcome: ChatInputOutcome) -> ChatComposerOutcome {
    match outcome {
        ChatInputOutcome::Command(command) => ChatComposerOutcome::Command(command),
        ChatInputOutcome::Consumed => ChatComposerOutcome::Consumed,
        ChatInputOutcome::Submit(prompt) => ChatComposerOutcome::Submit(prompt),
        ChatInputOutcome::Unhandled => ChatComposerOutcome::Unhandled,
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
