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
use zeta_ui::Point;
use zeta_ui::Rect;
use zeta_ui::TextInputCompositionEvent;

use crate::composer_editor::ComposerEditor;

/// Explicit submission mode for the shared Agent Console composer.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum ComposerMode {
    #[default]
    Agent,
    Shell,
}

pub(crate) enum ComposerSubmission {
    AgentMessage(String),
    ShellCommand(String),
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum ComposerModeSelection {
    #[default]
    Automatic,
    Explicit,
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

/// Host-owned editor shared by Agent messages and direct Shell commands.
pub(crate) struct AgentComposer {
    editor: ComposerEditor,
    mode: ComposerMode,
    mode_selection: ComposerModeSelection,
    classifier: InputClassifier,
    conversation: InputConversation,
    agent_response: AgentResponseState,
    shell_history: Vec<String>,
    shell_history_index: Option<usize>,
    shell_history_draft: Option<String>,
    shell_suggestion: Option<ShellGhostSuggestion>,
    dismissed_shell_suggestion_input: Option<String>,
}

impl Default for AgentComposer {
    fn default() -> Self {
        Self::for_working_directory(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

impl AgentComposer {
    pub(crate) fn for_working_directory(working_directory: impl Into<PathBuf>) -> Self {
        zeta_input_classifier::start_background_warmup();
        Self {
            editor: ComposerEditor::default(),
            mode: ComposerMode::Agent,
            mode_selection: ComposerModeSelection::Automatic,
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

    pub(crate) const fn editor(&self) -> &ComposerEditor {
        &self.editor
    }

    pub(crate) const fn mode(&self) -> ComposerMode {
        self.mode
    }

    pub(crate) fn set_mode(&mut self, mode: ComposerMode) {
        self.mode = mode;
        self.mode_selection = ComposerModeSelection::Explicit;
        self.editor.cancel_composition();
        self.leave_shell_history();
        self.refresh_editor_language();
        self.dismissed_shell_suggestion_input = None;
        self.refresh_shell_suggestion();
    }

    pub(crate) fn toggle_mode(&mut self) {
        self.set_mode(match self.mode {
            ComposerMode::Agent => ComposerMode::Shell,
            ComposerMode::Shell => ComposerMode::Agent,
        });
    }

    pub(crate) fn submission(&self) -> Option<ComposerSubmission> {
        (!self.editor.text().trim().is_empty()).then(|| match self.mode {
            ComposerMode::Agent => ComposerSubmission::AgentMessage(self.editor.text().to_owned()),
            ComposerMode::Shell => ComposerSubmission::ShellCommand(self.editor.text().to_owned()),
        })
    }

    pub(crate) fn set_text(&mut self, text: impl Into<String>) {
        self.leave_shell_history();
        self.editor.set_text(text);
        self.refresh_automatic_mode();
    }

    pub(crate) fn set_working_directory(&mut self, working_directory: &Path) {
        self.classifier.set_working_directory(working_directory);
        self.refresh_automatic_mode();
    }

    pub(crate) fn refresh_shell_workspace(&mut self) {
        self.classifier.refresh_shell_workspace();
        self.refresh_automatic_mode();
    }

    pub(crate) fn has_shell_suggestion(&self) -> bool {
        self.shell_suggestion.is_some()
    }

    pub(crate) fn accept_shell_suggestion(&mut self) -> bool {
        let Some(suggestion) = self.shell_suggestion.take() else {
            return false;
        };
        self.leave_shell_history();
        self.dismissed_shell_suggestion_input = None;
        let applied = self.editor.apply_text_edit(suggestion.edit);
        self.refresh_automatic_mode();
        applied
    }

    pub(crate) fn dismiss_shell_suggestion(&mut self) -> bool {
        if self.shell_suggestion.take().is_none() {
            return false;
        }
        self.dismissed_shell_suggestion_input = Some(self.editor.text().to_owned());
        self.editor.hide_ghost_text();
        true
    }

    fn shell_completion_snapshot(&self) -> Option<ShellCompletionSnapshot> {
        let text = self.editor.text();
        let cursor = self.editor.cursor();
        if cursor != text.len()
            || text.trim().is_empty()
            || text.trim_start().starts_with('/')
            || self.editor.selected_text().is_some()
            || self.editor.has_active_composition()
            || (self.mode_selection == ComposerModeSelection::Explicit
                && self.mode == ComposerMode::Agent)
            || (self.mode == ComposerMode::Agent
                && text[..cursor]
                    .chars()
                    .next_back()
                    .is_some_and(char::is_whitespace))
        {
            return None;
        }
        Some(self.classifier.shell_completion_snapshot(text, cursor))
    }

    pub(crate) fn mark_agent_message_submitted(&mut self, text: &str) {
        self.classifier
            .record_submission(InputHistoryEntry::agent(text));
        self.conversation = InputConversation::Standalone;
        self.agent_response = AgentResponseState::Pending;
    }

    pub(crate) fn mark_shell_command_submitted(&mut self, command: &str) {
        self.classifier
            .record_submission(InputHistoryEntry::shell(command));
        self.conversation = InputConversation::Standalone;
        self.agent_response = AgentResponseState::None;
    }

    pub(crate) fn replace_classification_history(
        &mut self,
        entries: impl IntoIterator<Item = InputHistoryEntry>,
    ) {
        self.classifier.replace_history(entries);
    }

    pub(crate) fn mark_agent_response_started(&mut self) {
        self.conversation = InputConversation::Standalone;
        self.agent_response = AgentResponseState::Pending;
        self.refresh_automatic_mode();
    }

    pub(crate) fn mark_agent_turn_completed(&mut self) {
        if self.agent_response == AgentResponseState::Pending {
            self.conversation = InputConversation::AgentFollowUp;
            self.agent_response = AgentResponseState::None;
            self.refresh_automatic_mode();
        }
    }

    pub(crate) fn mark_agent_turn_ended_without_response(&mut self) {
        self.agent_response = AgentResponseState::None;
    }

    pub(crate) fn synchronize_conversation(&mut self, conversation: InputConversation) {
        self.conversation = conversation;
        self.agent_response = AgentResponseState::None;
        self.refresh_automatic_mode();
    }

    pub(crate) fn set_editor_style(&mut self, style: CodeEditorStyle) {
        self.editor.set_style(style);
    }

    pub(crate) fn apply(&mut self, command: CodeEditorCommand) {
        if self.mode == ComposerMode::Shell {
            match command {
                CodeEditorCommand::MoveUp(CodeEditorSelectionMode::Move)
                    if self.editor.is_collapsed_at_first_row() =>
                {
                    if self.older_shell_history() {
                        return;
                    }
                }
                CodeEditorCommand::MoveDown(CodeEditorSelectionMode::Move)
                    if self.editor.is_collapsed_at_last_row() =>
                {
                    if self.newer_shell_history() {
                        return;
                    }
                }
                _ => self.leave_shell_history(),
            }
        }
        self.editor.apply(command);
        self.refresh_automatic_mode();
    }

    pub(crate) fn apply_composition(&mut self, event: TextInputCompositionEvent) {
        self.leave_shell_history();
        self.editor.apply_composition(event);
        self.refresh_automatic_mode();
    }

    pub(crate) fn cancel_composition(&mut self) {
        self.editor.cancel_composition();
        self.refresh_shell_suggestion();
    }

    pub(crate) fn clear_after_submit(&mut self) {
        if self.mode == ComposerMode::Shell {
            let command = self.editor.text().to_owned();
            if self.shell_history.last() != Some(&command) {
                self.shell_history.push(command);
            }
        }
        self.editor.clear();
        if self.mode_selection == ComposerModeSelection::Automatic {
            self.mode = ComposerMode::Agent;
        }
        self.refresh_editor_language();
        self.leave_shell_history();
        self.dismissed_shell_suggestion_input = None;
        self.refresh_shell_suggestion();
    }

    pub(crate) fn move_caret_to_point(
        &mut self,
        bounds: Rect,
        point: Point,
        mode: CodeEditorSelectionMode,
    ) -> bool {
        self.leave_shell_history();
        let moved = self.editor.move_caret_to_point(bounds, point, mode);
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
                self.shell_history_draft = Some(self.editor.text().to_owned());
                self.shell_history.len() - 1
            }
        };
        self.shell_history_index = Some(index);
        self.editor.set_text(self.shell_history[index].clone());
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
            self.editor.set_text(self.shell_history[next].clone());
            self.refresh_editor_language();
            self.refresh_shell_suggestion();
        } else {
            let draft = self.shell_history_draft.take().unwrap_or_default();
            self.shell_history_index = None;
            self.editor.set_text(draft);
            self.refresh_editor_language();
            self.refresh_shell_suggestion();
        }
        true
    }

    fn refresh_automatic_mode(&mut self) {
        if self.mode_selection == ComposerModeSelection::Automatic {
            let text = self.editor.text();
            self.mode = if text.trim_start().starts_with('/') {
                ComposerMode::Agent
            } else {
                let current_route = match self.mode {
                    ComposerMode::Agent => InputRoute::Agent,
                    ComposerMode::Shell => InputRoute::Shell,
                };
                let context = InputClassificationContext::new(current_route, self.conversation);
                match self.classifier.classify(text, context).route {
                    InputRoute::Agent => ComposerMode::Agent,
                    InputRoute::Shell => ComposerMode::Shell,
                }
            };
        }
        self.refresh_editor_language();
        self.refresh_shell_suggestion();
    }

    fn refresh_editor_language(&mut self) {
        self.editor.set_language(match self.mode {
            ComposerMode::Agent => CodeEditorLanguage::PlainText,
            ComposerMode::Shell => CodeEditorLanguage::Shell,
        });
    }

    fn refresh_shell_suggestion(&mut self) {
        let text = self.editor.text().to_owned();
        if self.dismissed_shell_suggestion_input.as_deref() == Some(&text) {
            self.shell_suggestion = None;
            self.editor.hide_ghost_text();
            return;
        }
        self.dismissed_shell_suggestion_input = None;
        let suggestion = self
            .shell_completion_snapshot()
            .filter(|snapshot| !snapshot.has_exact_match())
            .and_then(|snapshot| {
                shell_ghost_suggestion(&text, self.editor.cursor(), snapshot.into_completions())
            });
        if let Some(suggestion) = &suggestion {
            let typed = &text[suggestion.edit.range.clone()];
            let ghost_text = suggestion.edit.new_text[typed.len()..].to_owned();
            self.editor.show_ghost_text(ghost_text);
        } else {
            self.editor.hide_ghost_text();
        }
        self.shell_suggestion = suggestion;
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
#[path = "agent_composer_tests.rs"]
mod tests;
