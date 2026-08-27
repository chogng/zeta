//! Retained state for one Agent Session Pane.

use std::path::Path;
use std::path::PathBuf;

use serde_json::Value;
use zeta_editor::CodeEditorCommand;
use zeta_editor::CodeEditorSelectionMode;
use zeta_editor::CodeEditorStyle;
use zeta_input_classifier::InputConversation;
use zeta_input_classifier::InputHistoryEntry;
use zeta_protocol::Thread;
use zeta_protocol::ThreadItem;
use zeta_protocol::TurnStatus;
use zeta_slash_commands::SlashCommandCatalogError;
use zeta_slash_commands::SlashCommandDefinition;
use zeta_thread_transcript::ThreadTranscriptSnapshot;
use zeta_thread_transcript::ThreadTranscriptUpdateEnvelope;
use zeta_ui_components::ScrollCommand;
use zui::ui::Point;
use zui::ui::Rect;
use zui::ui::Size;
use zui::ui::TextInputCompositionEvent;

use crate::Composer;
use crate::ComposerInput;
use crate::ComposerInteractionActivation;
use crate::ComposerInteractionModel;
use crate::ComposerInteractionPaneState;
use crate::ComposerInteractionView;
use crate::ComposerModelOption;
use crate::ComposerRoute;
use crate::ComposerSubmission;
use crate::SelectionDirection;
use crate::ThreadTimelineScroll;
use crate::TranscriptState;
use crate::line_capacity;
use crate::line_count;

/// Complete retained state for one Agent Session Pane.
///
/// The product host supplies canonical Thread and transcript snapshots, mechanically assembled
/// transcript changes, and executes submissions. The pane owns their presentation state, Composer
/// state, and timeline scroll position as one unit.
pub struct SessionPaneState {
    thread: Option<Thread>,
    transcript: TranscriptState,
    timeline_scroll: ThreadTimelineScroll,
    composer: Composer,
}

impl Default for SessionPaneState {
    fn default() -> Self {
        Self::for_working_directory(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

impl SessionPaneState {
    pub fn for_working_directory(working_directory: impl Into<PathBuf>) -> Self {
        Self {
            thread: None,
            transcript: TranscriptState::default(),
            timeline_scroll: ThreadTimelineScroll::default(),
            composer: Composer::for_working_directory(working_directory),
        }
    }

    pub const fn thread(&self) -> Option<&Thread> {
        self.thread.as_ref()
    }

    pub(crate) const fn transcript(&self) -> &TranscriptState {
        &self.transcript
    }

    pub const fn timeline_scroll(&self) -> &ThreadTimelineScroll {
        &self.timeline_scroll
    }

    pub fn timeline_scroll_mut(&mut self) -> &mut ThreadTimelineScroll {
        &mut self.timeline_scroll
    }

    pub(crate) const fn input(&self) -> &ComposerInput {
        self.composer.input()
    }

    pub(crate) const fn interaction(&self) -> &ComposerInteractionModel {
        self.composer.interaction()
    }

    pub(crate) const fn interaction_pane(&self) -> &ComposerInteractionPaneState {
        self.composer.interaction_pane()
    }

    pub fn composer_preferred_height(&self) -> f32 {
        self.composer.input().preferred_height()
    }

    pub fn selected_composer_text(&self) -> Option<&str> {
        self.composer.input().selected_text()
    }

    pub const fn composer_route(&self) -> ComposerRoute {
        self.composer.route()
    }

    pub fn composer_submission(&self) -> Option<ComposerSubmission> {
        self.composer.submission()
    }

    pub fn set_composer_text(&mut self, text: impl Into<String>) {
        self.composer.set_text(text);
    }

    pub fn set_composer_style(&mut self, style: CodeEditorStyle) {
        self.composer.set_input_style(style);
    }

    pub fn set_composer_catalog(
        &mut self,
        slash_commands: Vec<SlashCommandDefinition>,
        models: Vec<ComposerModelOption>,
    ) -> Result<(), SlashCommandCatalogError> {
        self.composer
            .interaction_mut()
            .set_catalog(slash_commands, models)
    }

    pub fn refresh_shell_workspace(&mut self) {
        self.composer.refresh_shell_workspace();
    }

    pub fn apply_composer_command(&mut self, command: CodeEditorCommand) {
        self.composer.apply(command);
    }

    pub fn apply_composer_composition(&mut self, event: TextInputCompositionEvent) {
        self.composer.apply_composition(event);
    }

    pub fn cancel_composer_composition(&mut self) {
        self.composer.cancel_composition();
    }

    pub fn accept_shell_suggestion(&mut self) -> bool {
        self.composer.accept_shell_suggestion()
    }

    pub fn dismiss_shell_suggestion(&mut self) -> bool {
        self.composer.dismiss_shell_suggestion()
    }

    pub fn has_shell_suggestion(&self) -> bool {
        self.composer.has_shell_suggestion()
    }

    pub fn mark_agent_message_submitted(&mut self, text: &str) {
        self.composer.mark_agent_message_submitted(text);
    }

    pub fn mark_shell_command_submitted(&mut self, command: &str) {
        self.composer.mark_shell_command_submitted(command);
    }

    pub fn clear_composer_after_submit(&mut self) {
        self.composer.clear_after_submit();
    }

    pub fn move_composer_caret_to_point(
        &mut self,
        bounds: Rect,
        point: Point,
        mode: CodeEditorSelectionMode,
    ) -> bool {
        self.composer.move_caret_to_point(bounds, point, mode)
    }

    pub fn move_composer_interaction_selection(&mut self, direction: SelectionDirection) {
        self.composer.interaction_mut().move_selection(direction);
    }

    pub fn activate_composer_interaction(&mut self) -> Option<ComposerInteractionActivation> {
        self.composer.interaction_mut().activate_selected()
    }

    pub fn complete_selected_slash(&mut self) -> Option<String> {
        self.composer.interaction_mut().complete_selected_slash()
    }

    pub fn dismiss_composer_interaction(&mut self) -> bool {
        let text = self.composer.input().text().to_owned();
        let dismissed = self.composer.interaction_mut().dismiss(&text);
        if dismissed {
            self.composer.interaction_pane_mut().reset();
        }
        dismissed
    }

    pub fn composer_interaction_view(&self) -> Option<ComposerInteractionView<'_>> {
        self.composer.interaction().view()
    }

    pub fn composer_model_picker_visible(&self) -> bool {
        self.composer.interaction().is_model_picker_visible()
    }

    pub fn composer_interaction_visible(&self) -> bool {
        self.composer.interaction().is_visible()
    }

    pub fn select_composer_interaction_item(&mut self, index: usize) -> bool {
        self.composer.interaction_mut().select_item(index)
    }

    pub fn reset_composer_interaction_scroll(&mut self) {
        self.composer.interaction_pane_mut().reset();
    }

    pub fn scroll_composer_interaction(
        &mut self,
        command: ScrollCommand,
        viewport: Size,
        content: Size,
    ) -> bool {
        self.composer
            .interaction_pane_mut()
            .apply_scroll(command, viewport, content)
    }

    pub fn replace_thread(
        &mut self,
        thread: Thread,
        transcript: ThreadTranscriptSnapshot,
        scroll_limit: usize,
    ) {
        let previous_line_count = line_count(&self.transcript);
        synchronize_composer(&mut self.composer, &thread);
        self.thread = Some(thread);
        self.transcript.replace_snapshot(transcript);
        self.preserve_timeline_after_growth(previous_line_count, scroll_limit);
    }

    pub fn apply_transcript_update(
        &mut self,
        update: ThreadTranscriptUpdateEnvelope,
        scroll_limit: usize,
    ) {
        let previous_line_count = line_count(&self.transcript);
        self.transcript.apply_update(update);
        self.preserve_timeline_after_growth(previous_line_count, scroll_limit);
    }

    pub fn timeline_scroll_limit(&self, bounds: Rect) -> usize {
        line_count(&self.transcript).saturating_sub(line_capacity(bounds))
    }

    pub fn set_working_directory(&mut self, working_directory: &Path) {
        self.composer.set_working_directory(working_directory);
    }

    fn preserve_timeline_after_growth(&mut self, previous_line_count: usize, scroll_limit: usize) {
        let added_lines = line_count(&self.transcript).saturating_sub(previous_line_count);
        self.timeline_scroll
            .preserve_view_after_growth(added_lines, scroll_limit);
        self.timeline_scroll.clamp(scroll_limit);
    }
}

fn synchronize_composer(composer: &mut Composer, thread: &Thread) {
    composer.replace_classification_history(classification_history_for_thread(thread));
    let Some(turn) = thread.turns.last() else {
        composer.synchronize_conversation(InputConversation::Standalone);
        return;
    };
    let has_agent_message = turn
        .items
        .iter()
        .any(|item| matches!(item, ThreadItem::AgentMessage { .. }));
    match (turn.status, has_agent_message) {
        (TurnStatus::Completed, true) => {
            composer.synchronize_conversation(InputConversation::AgentFollowUp);
        }
        (
            TurnStatus::Created
            | TurnStatus::Running
            | TurnStatus::WaitingForApproval
            | TurnStatus::WaitingForUserInput
            | TurnStatus::WaitingForCapability
            | TurnStatus::Cancelling,
            true,
        ) => composer.mark_agent_response_started(),
        _ => composer.synchronize_conversation(InputConversation::Standalone),
    }
}

fn classification_history_for_thread(thread: &Thread) -> Vec<InputHistoryEntry> {
    thread
        .turns
        .iter()
        .flat_map(|turn| {
            let agent_prompts = turn.items.iter().filter_map(|item| match item {
                ThreadItem::UserMessage { text, .. } => {
                    Some(InputHistoryEntry::agent(text.clone()))
                }
                _ => None,
            });
            let has_agent_prompt = turn
                .items
                .iter()
                .any(|item| matches!(item, ThreadItem::UserMessage { .. }));
            let shell_command = (!has_agent_prompt && !turn_command_was_not_found(&turn.items))
                .then(|| turn.items.iter().find_map(shell_history_entry))
                .flatten();
            agent_prompts.chain(shell_command)
        })
        .collect()
}

fn shell_history_entry(item: &ThreadItem) -> Option<InputHistoryEntry> {
    let ThreadItem::ToolCall {
        name,
        arguments_json,
        ..
    } = item
    else {
        return None;
    };
    if name.as_str() != "shell-command" {
        return None;
    }
    let Value::Object(arguments) = serde_json::from_str(arguments_json).ok()? else {
        return None;
    };
    if let Some(command) = arguments.get("command").and_then(Value::as_str) {
        return Some(InputHistoryEntry::shell(command));
    }
    let arguments = arguments.get("arguments").and_then(Value::as_array)?;
    (arguments.len() == 2 && arguments[0].as_str() == Some("-lc"))
        .then(|| arguments[1].as_str().map(InputHistoryEntry::shell))
        .flatten()
}

fn turn_command_was_not_found(items: &[ThreadItem]) -> bool {
    items.iter().any(|item| {
        let ThreadItem::ToolResult { text, .. } = item else {
            return false;
        };
        serde_json::from_str::<Value>(text)
            .ok()
            .and_then(|value| value.get("exit_code").and_then(Value::as_i64))
            == Some(127)
    })
}

#[cfg(test)]
#[path = "pane_state_tests.rs"]
mod tests;
