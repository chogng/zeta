use crate::components::chat_input::ChatInput;
use crate::components::chat_input::ChatInputOutcome;
use crate::components::chat_input::ChatInputQueueOutcome;
use crate::components::chat_input::ChatSubmission;
use crate::components::chat_input::QueuedChatInput;
use crate::components::chat_input::SlashCommandCatalog;
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
use crate::components::suggest::MentionPluginItem;
use crate::components::suggest::SkillSelectorItem;
use crate::components::suggest::Suggest;
use crate::components::suggest::SuggestEdit;
use crate::components::suggest::SuggestInputOutcome;
use crate::components::suggest::SuggestView;
use crate::components::text_prompt::TextPromptSpec;
use crate::mouse::MouseMode;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use zeta_file_search::PathSearchSnapshot;

pub(crate) enum ChatComposerOverlayView<'a> {
    Suggest(SuggestView<'a>),
}

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

    pub(super) fn overlay(&self) -> Option<ChatComposerOverlayView<'_>> {
        self.state.overlay(self.input)
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

/// Owns focus and routing for the persistent chat input and pages above it.
///
/// The chat input remains alive while pages are stacked above it, preserving draft state when a
/// page is dismissed. Product feature state remains outside this component.
#[derive(Debug)]
pub(crate) struct ChatComposer {
    suggest: Suggest,
    panes: PaneStack,
    pane_order: Vec<ChatComposerPaneKind>,
    steer: Steer,
}

impl ChatComposer {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            suggest: Suggest::new(crate::components::chat_input::default_slash_command_catalog()),
            panes: PaneStack::default(),
            pane_order: Vec::new(),
            steer: Steer::default(),
        }
    }

    pub(crate) fn with_slash_commands(slash_commands: SlashCommandCatalog) -> Self {
        Self {
            suggest: Suggest::new(slash_commands),
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
            && self.suggest(input).is_none()
            && input.accepts_submission_key()
        {
            return self.queue_current_input(input);
        }
        if submission_target == SubmissionTarget::Steer
            && key.code == KeyCode::Enter
            && key.modifiers.is_empty()
            && self.suggest(input).is_none()
            && input.accepts_submission_key()
            && input.submission_contains_skill()
        {
            return ChatComposerOutcome::SubmissionRejected(
                "A running Turn cannot change its Skill; switch follow-up messages to Queue or wait for the next Turn"
                    .into(),
            );
        }
        map_chat_input_outcome(self.handle_chat_input_key(input, key))
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, input: &mut ChatInput, text: &str) {
        input.insert_text(text);
        self.sync_suggest(input);
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
        self.sync_suggest(input);
        Ok(())
    }

    pub(crate) fn attach_image_bytes(
        &mut self,
        input: &mut ChatInput,
        bytes: Vec<u8>,
    ) -> Result<(), String> {
        input.attach_image_bytes(bytes)?;
        self.sync_suggest(input);
        Ok(())
    }

    pub(crate) fn replace_chat_input_catalog(
        &mut self,
        input: &mut ChatInput,
        slash_commands: SlashCommandCatalog,
        skills: Vec<SkillSelectorItem>,
        plugins: Vec<MentionPluginItem>,
    ) {
        self.suggest
            .replace_catalog(slash_commands, skills, plugins);
        self.sync_suggest(input);
    }

    pub(crate) fn view<'a>(&'a self, input: &'a ChatInput) -> ChatComposerView<'a> {
        ChatComposerView { state: self, input }
    }

    pub(crate) fn suggest(&self, _input: &ChatInput) -> Option<SuggestView<'_>> {
        if !self.panes.is_empty() {
            return None;
        }
        self.suggest.view()
    }

    pub(crate) fn mouse_mode(&self, input: &ChatInput) -> MouseMode {
        if self.panes.mouse_mode() == MouseMode::UiClick || self.suggest(input).is_some() {
            MouseMode::UiClick
        } else {
            MouseMode::TerminalSelection
        }
    }

    pub(crate) fn mention_query(&self) -> Option<&str> {
        if !self.panes.is_empty() {
            return None;
        }
        self.suggest.mention_query()
    }

    pub(crate) fn apply_file_search_snapshot(&mut self, snapshot: PathSearchSnapshot) {
        self.suggest.apply_file_search_snapshot(snapshot);
    }

    pub(crate) fn overlay(&self, input: &ChatInput) -> Option<ChatComposerOverlayView<'_>> {
        self.suggest(input).map(ChatComposerOverlayView::Suggest)
    }

    pub(crate) fn pane_active(&self) -> bool {
        !self.panes.is_empty()
    }

    pub(crate) fn activate_suggest(
        &mut self,
        input: &mut ChatInput,
        index: usize,
    ) -> Option<ChatComposerOutcome> {
        if !self.panes.is_empty() {
            return None;
        }
        self.suggest(input)?;
        self.activate_chat_input_suggest(input, index)
            .map(map_chat_input_outcome)
    }

    pub(crate) fn select_suggest(&mut self, input: &ChatInput, index: usize) -> bool {
        if !self.panes.is_empty() {
            return false;
        }
        self.suggest(input).is_some() && self.suggest.select(index)
    }

    pub(crate) fn select_overlay_choice(&mut self, input: &ChatInput, index: usize) -> bool {
        self.select_suggest(input, index)
    }

    pub(crate) fn activate_overlay_choice(
        &mut self,
        input: &mut ChatInput,
        index: usize,
    ) -> Option<ChatComposerOutcome> {
        self.activate_suggest(input, index)
    }

    pub(crate) fn select_visible_item(&mut self, index: usize) -> bool {
        self.panes.select_visible_item(index)
    }

    pub(crate) fn select_tab(&mut self, index: usize) -> bool {
        self.panes.select_tab(index)
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
        let command = self.current_command(input);
        let outcome = input.queue_current(command);
        self.suggest.clear();
        match outcome {
            ChatInputQueueOutcome::Command(command) => ChatComposerOutcome::Command(command),
            ChatInputQueueOutcome::Consumed => ChatComposerOutcome::Consumed,
            ChatInputQueueOutcome::Queued(input) => ChatComposerOutcome::Queued(input),
        }
    }

    fn handle_chat_input_key(&mut self, input: &mut ChatInput, key: KeyEvent) -> ChatInputOutcome {
        match self.suggest.handle_key(key) {
            SuggestInputOutcome::Completed(edit) => {
                self.apply_suggest_edit(input, edit);
                self.sync_suggest(input);
                return ChatInputOutcome::Consumed;
            }
            SuggestInputOutcome::Consumed => return ChatInputOutcome::Consumed,
            SuggestInputOutcome::Submit(edit) => {
                self.apply_suggest_edit(input, edit);
                self.sync_suggest(input);
                return self.submit_current(input);
            }
            SuggestInputOutcome::Unhandled => {}
        }

        if key.code == KeyCode::Enter && key.modifiers.is_empty() && input.accepts_submission_key()
        {
            return self.submit_current(input);
        }

        let outcome = input.handle_key(key);
        self.sync_suggest(input);
        outcome
    }

    fn activate_chat_input_suggest(
        &mut self,
        input: &mut ChatInput,
        index: usize,
    ) -> Option<ChatInputOutcome> {
        let outcome = self.suggest.activate(index)?;
        let (edit, submit) = match outcome {
            SuggestInputOutcome::Completed(edit) => (edit, false),
            SuggestInputOutcome::Submit(edit) => (edit, true),
            SuggestInputOutcome::Consumed | SuggestInputOutcome::Unhandled => return None,
        };
        self.apply_suggest_edit(input, edit);
        self.sync_suggest(input);
        Some(if submit {
            self.submit_current(input)
        } else {
            ChatInputOutcome::Consumed
        })
    }

    fn submit_current(&mut self, input: &mut ChatInput) -> ChatInputOutcome {
        let command = self.current_command(input);
        let outcome = input.submit_current(command);
        self.suggest.clear();
        outcome
    }

    fn current_command(
        &self,
        input: &ChatInput,
    ) -> Option<zeta_slash_commands::SlashCommandInvocation> {
        input
            .submission_display_text()
            .and_then(|text| self.suggest.invocation(&text))
    }

    fn sync_suggest(&mut self, input: &mut ChatInput) {
        let desired_command = self
            .suggest
            .command_element_range(input.draft_text(), input.draft_cursor());
        input.reconcile(desired_command);
        self.suggest.sync_textarea(
            input.draft_text(),
            input.draft_cursor(),
            input.slash_command_active(),
        );
    }

    fn apply_suggest_edit(&mut self, input: &mut ChatInput, edit: SuggestEdit) {
        match edit {
            SuggestEdit::Text { range, replacement } => {
                input.apply_text_completion(range, &replacement);
            }
            SuggestEdit::Element {
                range,
                value,
                skill,
            } => input.apply_element_completion(range, &value, skill),
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
