use std::path::{Path, PathBuf};

use zeta_editor::{
    CodeEditorCommand, CodeEditorLanguage, CodeEditorSelectionMode, CodeEditorStyle,
};
use zeta_input_classifier::InputClassificationContext;
use zeta_input_classifier::InputClassifier;
use zeta_input_classifier::InputConversation;
use zeta_input_classifier::InputHistoryEntry;
use zeta_input_classifier::InputRoute;
use zeta_ui::{Point, Rect, TextInputCompositionEvent};

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
    }

    pub(crate) fn move_caret_to_point(
        &mut self,
        bounds: Rect,
        point: Point,
        mode: CodeEditorSelectionMode,
    ) -> bool {
        self.leave_shell_history();
        self.editor.move_caret_to_point(bounds, point, mode)
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
        } else {
            let draft = self.shell_history_draft.take().unwrap_or_default();
            self.shell_history_index = None;
            self.editor.set_text(draft);
            self.refresh_editor_language();
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
    }

    fn refresh_editor_language(&mut self) {
        self.editor.set_language(match self.mode {
            ComposerMode::Agent => CodeEditorLanguage::PlainText,
            ComposerMode::Shell => CodeEditorLanguage::Shell,
        });
    }

    fn leave_shell_history(&mut self) {
        self.shell_history_index = None;
        self.shell_history_draft = None;
    }
}

#[cfg(test)]
#[path = "agent_composer_tests.rs"]
mod tests;
