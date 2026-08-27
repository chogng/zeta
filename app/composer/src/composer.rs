use std::path::Path;
use std::path::PathBuf;

use zeta_editor::CodeEditorCommand;
use zeta_editor::CodeEditorLanguage;
use zeta_editor::CodeEditorSelectionMode;
use zeta_editor::CodeEditorStyle;
use zeta_editor::CodeEditorTextEdit;
use zeta_input_classifier::InputClassificationContext;
use zeta_input_classifier::InputClassifier;
use zeta_input_classifier::InputConversation;
use zeta_input_classifier::InputHistoryEntry;
use zeta_input_classifier::InputRoute;
use zeta_input_classifier::ShellCompletion;
use zeta_input_classifier::ShellCompletionSnapshot;
use zui::ui::Point;
use zui::ui::Rect;
use zui::ui::TextInputCompositionEvent;

use crate::ComposerInput;
use crate::ComposerInteractionModel;
use crate::ComposerInteractionPaneState;

/// Current classifier-selected submission route for the shared Agent Console composer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ComposerRoute {
    #[default]
    Agent,
    Shell,
}

pub enum ComposerSubmission {
    AgentMessage(String),
    ShellCommand(String),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum AgentResponseState {
    #[default]
    None,
    Pending,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ShellGhostSuggestion {
    edit: CodeEditorTextEdit,
}

/// Composer-owned input, routing, history, and completion state shared by Agent and Shell input.
pub struct Composer {
    input: ComposerInput,
    interaction: ComposerInteractionModel,
    interaction_pane: ComposerInteractionPaneState,
    route: ComposerRoute,
    classifier: InputClassifier,
    conversation: InputConversation,
    agent_response: AgentResponseState,
    shell_history: Vec<String>,
    shell_history_index: Option<usize>,
    shell_history_draft: Option<String>,
    shell_suggestion: Option<ShellGhostSuggestion>,
    dismissed_shell_suggestion_input: Option<String>,
}

impl Default for Composer {
    fn default() -> Self {
        Self::for_working_directory(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

impl Composer {
    pub fn for_working_directory(working_directory: impl Into<PathBuf>) -> Self {
        zeta_input_classifier::start_background_warmup();
        Self {
            input: ComposerInput::default(),
            interaction: ComposerInteractionModel::new(),
            interaction_pane: ComposerInteractionPaneState::default(),
            route: ComposerRoute::Agent,
            classifier: InputClassifier::for_working_directory(working_directory),
            conversation: InputConversation::Standalone,
            agent_response: AgentResponseState::None,
            shell_history: Vec::new(),
            shell_history_index: None,
            shell_history_draft: None,
            shell_suggestion: None,
            dismissed_shell_suggestion_input: None,
        }
    }

    pub const fn input(&self) -> &ComposerInput {
        &self.input
    }

    pub const fn interaction(&self) -> &ComposerInteractionModel {
        &self.interaction
    }

    pub fn interaction_mut(&mut self) -> &mut ComposerInteractionModel {
        &mut self.interaction
    }

    pub const fn interaction_pane(&self) -> &ComposerInteractionPaneState {
        &self.interaction_pane
    }

    pub fn interaction_pane_mut(&mut self) -> &mut ComposerInteractionPaneState {
        &mut self.interaction_pane
    }

    pub const fn route(&self) -> ComposerRoute {
        self.route
    }

    pub fn submission(&self) -> Option<ComposerSubmission> {
        (!self.input.text().trim().is_empty()).then(|| match self.route {
            ComposerRoute::Agent => ComposerSubmission::AgentMessage(self.input.text().to_owned()),
            ComposerRoute::Shell => ComposerSubmission::ShellCommand(self.input.text().to_owned()),
        })
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.leave_shell_history();
        self.input.set_text(text);
        self.refresh_classification();
    }

    pub fn set_working_directory(&mut self, working_directory: &Path) {
        self.classifier.set_working_directory(working_directory);
        self.refresh_classification();
    }

    pub fn refresh_shell_workspace(&mut self) {
        self.classifier.refresh_shell_workspace();
        self.refresh_classification();
    }

    pub fn has_shell_suggestion(&self) -> bool {
        self.shell_suggestion.is_some()
    }

    pub fn accept_shell_suggestion(&mut self) -> bool {
        let Some(suggestion) = self.shell_suggestion.take() else {
            return false;
        };
        self.leave_shell_history();
        self.dismissed_shell_suggestion_input = None;
        let applied = self.input.apply_text_edit(suggestion.edit);
        self.refresh_classification();
        applied
    }

    pub fn dismiss_shell_suggestion(&mut self) -> bool {
        if self.shell_suggestion.take().is_none() {
            return false;
        }
        self.dismissed_shell_suggestion_input = Some(self.input.text().to_owned());
        self.input.hide_ghost_text();
        true
    }

    fn shell_completion_snapshot(&self) -> Option<ShellCompletionSnapshot> {
        let text = self.input.text();
        let cursor = self.input.cursor();
        if cursor != text.len()
            || text.trim().is_empty()
            || text.trim_start().starts_with('/')
            || self.input.selected_text().is_some()
            || self.input.has_active_composition()
            || (self.route == ComposerRoute::Agent
                && text[..cursor]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace))
        {
            return None;
        }
        Some(self.classifier.shell_completion_snapshot(text, cursor))
    }

    pub fn mark_agent_message_submitted(&mut self, text: &str) {
        self.classifier
            .record_submission(InputHistoryEntry::agent(text));
        self.conversation = InputConversation::Standalone;
        self.agent_response = AgentResponseState::Pending;
    }

    pub fn mark_shell_command_submitted(&mut self, command: &str) {
        self.classifier
            .record_submission(InputHistoryEntry::shell(command));
        self.conversation = InputConversation::Standalone;
        self.agent_response = AgentResponseState::None;
    }

    pub fn replace_classification_history(
        &mut self,
        entries: impl IntoIterator<Item = InputHistoryEntry>,
    ) {
        self.classifier.replace_history(entries);
    }

    pub fn mark_agent_response_started(&mut self) {
        self.conversation = InputConversation::Standalone;
        self.agent_response = AgentResponseState::Pending;
        self.refresh_classification();
    }

    pub fn mark_agent_turn_completed(&mut self) {
        if self.agent_response == AgentResponseState::Pending {
            self.conversation = InputConversation::AgentFollowUp;
            self.agent_response = AgentResponseState::None;
            self.refresh_classification();
        }
    }

    pub fn mark_agent_turn_ended_without_response(&mut self) {
        self.agent_response = AgentResponseState::None;
    }

    pub fn synchronize_conversation(&mut self, conversation: InputConversation) {
        self.conversation = conversation;
        self.agent_response = AgentResponseState::None;
        self.refresh_classification();
    }

    pub fn set_input_style(&mut self, style: CodeEditorStyle) {
        self.input.set_style(style);
    }

    pub fn apply(&mut self, command: CodeEditorCommand) {
        if self.route == ComposerRoute::Shell {
            match command {
                CodeEditorCommand::MoveUp(CodeEditorSelectionMode::Move)
                    if self.input.is_collapsed_at_first_row() =>
                {
                    if self.older_shell_history() {
                        return;
                    }
                }
                CodeEditorCommand::MoveDown(CodeEditorSelectionMode::Move)
                    if self.input.is_collapsed_at_last_row() =>
                {
                    if self.newer_shell_history() {
                        return;
                    }
                }
                _ => self.leave_shell_history(),
            }
        }
        self.input.apply(command);
        self.refresh_classification();
    }

    pub fn apply_composition(&mut self, event: TextInputCompositionEvent) {
        self.leave_shell_history();
        self.input.apply_composition(event);
        self.refresh_classification();
    }

    pub fn cancel_composition(&mut self) {
        self.input.cancel_composition();
        self.refresh_shell_suggestion();
    }

    pub fn clear_after_submit(&mut self) {
        if self.route == ComposerRoute::Shell {
            let command = self.input.text().to_owned();
            if self.shell_history.last() != Some(&command) {
                self.shell_history.push(command);
            }
        }
        self.input.clear();
        self.route = ComposerRoute::Agent;
        self.refresh_editor_language();
        self.leave_shell_history();
        self.dismissed_shell_suggestion_input = None;
        self.refresh_shell_suggestion();
        self.refresh_interaction();
    }

    pub fn move_caret_to_point(
        &mut self,
        bounds: Rect,
        point: Point,
        mode: CodeEditorSelectionMode,
    ) -> bool {
        self.leave_shell_history();
        let moved = self.input.move_caret_to_point(bounds, point, mode);
        if moved {
            self.refresh_shell_suggestion();
        }
        moved
    }

    fn older_shell_history(&mut self) -> bool {
        if self.shell_history.is_empty() {
            return false;
        }
        let index = match self.shell_history_index {
            Some(index) => index.saturating_sub(1),
            None => {
                self.shell_history_draft = Some(self.input.text().to_owned());
                self.shell_history.len() - 1
            }
        };
        self.shell_history_index = Some(index);
        self.input.set_text(self.shell_history[index].clone());
        self.refresh_editor_language();
        self.refresh_shell_suggestion();
        true
    }

    fn newer_shell_history(&mut self) -> bool {
        let Some(index) = self.shell_history_index else {
            return false;
        };
        if index + 1 < self.shell_history.len() {
            let next = index + 1;
            self.shell_history_index = Some(next);
            self.input.set_text(self.shell_history[next].clone());
            self.refresh_editor_language();
            self.refresh_shell_suggestion();
        } else {
            let draft = self.shell_history_draft.take().unwrap_or_default();
            self.shell_history_index = None;
            self.input.set_text(draft);
            self.refresh_editor_language();
            self.refresh_shell_suggestion();
        }
        true
    }

    fn refresh_classification(&mut self) {
        let text = self.input.text();
        self.route = if text.trim_start().starts_with('/') {
            ComposerRoute::Agent
        } else {
            let current_route = match self.route {
                ComposerRoute::Agent => InputRoute::Agent,
                ComposerRoute::Shell => InputRoute::Shell,
            };
            let context = InputClassificationContext::new(current_route, self.conversation);
            match self.classifier.classify(text, context).route {
                InputRoute::Agent => ComposerRoute::Agent,
                InputRoute::Shell => ComposerRoute::Shell,
            }
        };
        self.refresh_editor_language();
        self.refresh_shell_suggestion();
        self.refresh_interaction();
    }

    fn refresh_editor_language(&mut self) {
        self.input.set_language(match self.route {
            ComposerRoute::Agent => CodeEditorLanguage::PlainText,
            ComposerRoute::Shell => CodeEditorLanguage::Shell,
        });
    }

    fn refresh_shell_suggestion(&mut self) {
        let text = self.input.text().to_owned();
        if self.dismissed_shell_suggestion_input.as_deref() == Some(&text) {
            self.shell_suggestion = None;
            self.input.hide_ghost_text();
            return;
        }
        self.dismissed_shell_suggestion_input = None;
        let suggestion = self
            .shell_completion_snapshot()
            .filter(|snapshot| !snapshot.has_exact_match())
            .and_then(|snapshot| {
                shell_ghost_suggestion(&text, self.input.cursor(), snapshot.into_completions())
            });
        if let Some(suggestion) = &suggestion {
            let typed = &text[suggestion.edit.range.clone()];
            let ghost_text = suggestion.edit.new_text[typed.len()..].to_owned();
            self.input.show_ghost_text(ghost_text);
        } else {
            self.input.hide_ghost_text();
        }
        self.shell_suggestion = suggestion;
    }

    fn refresh_interaction(&mut self) {
        let was_visible = self.interaction.is_visible();
        let text = self.input.text().to_owned();
        self.interaction.sync_for_composer(&text, self.route);
        if was_visible != self.interaction.is_visible() {
            self.interaction_pane.reset();
        }
    }

    fn leave_shell_history(&mut self) {
        self.shell_history_index = None;
        self.shell_history_draft = None;
    }
}

fn shell_ghost_suggestion(
    input: &str,
    cursor: usize,
    completions: Vec<ShellCompletion>,
) -> Option<ShellGhostSuggestion> {
    let first = completions.iter().find(|completion| {
        let range = completion.replace_range();
        range.end == cursor
            && input
                .get(range)
                .is_some_and(|typed| completion.replacement().starts_with(typed))
    })?;
    let range = first.replace_range();
    let typed = input.get(range.clone())?;
    let replacements = completions
        .iter()
        .filter(|completion| completion.replace_range() == range)
        .map(ShellCompletion::replacement)
        .filter(|replacement| replacement.starts_with(typed))
        .collect::<Vec<_>>();
    let first_replacement = *replacements.first()?;
    let common_prefix_length = replacements
        .iter()
        .skip(1)
        .fold(first_replacement.len(), |length, replacement| {
            common_prefix_length(&first_replacement[..length], replacement)
        });
    if common_prefix_length <= typed.len() {
        return None;
    }
    Some(ShellGhostSuggestion {
        edit: CodeEditorTextEdit {
            range,
            new_text: first_replacement[..common_prefix_length].to_owned(),
        },
    })
}

fn common_prefix_length(left: &str, right: &str) -> usize {
    for ((offset, left), right) in left.char_indices().zip(right.chars()) {
        if left != right {
            return offset;
        }
    }
    left.len().min(right.len())
}

#[cfg(test)]
#[path = "composer_tests.rs"]
mod tests;
