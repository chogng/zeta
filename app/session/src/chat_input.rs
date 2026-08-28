mod catalog;
mod editor;
mod interaction;
mod interaction_view;
mod layout;
mod shell_completion;
mod toolbar;
mod view;

use std::path::Path;
use std::path::PathBuf;

pub use catalog::composer_model_options;
use editor::ChatInputEditor;
pub use interaction::ChatInputInteractionItem;
use interaction::ChatInputInteractionState;
pub use interaction::ChatInputInteractionView;
pub use interaction::ComposerInteractionActivation;
pub use interaction::ComposerModelOption;
pub use interaction::SelectionDirection;
pub use layout::ComposerPanelLayout;
pub use layout::INTERACTION_ROW_HEIGHT;
pub use layout::interaction_content_size;
pub use layout::interaction_list_bounds;
pub use layout::interaction_preferred_height;
pub use layout::interaction_selection_scroll_command;
use shell_completion::ShellGhostSuggestion;
use shell_completion::shell_ghost_suggestion;
pub(crate) use view::ChatInputView;
pub(crate) use view::draw_chat_input;
use zeta_editor::CodeEditorCommand;
use zeta_editor::CodeEditorLanguage;
use zeta_editor::CodeEditorSelectionMode;
use zeta_editor::CodeEditorStyle;
use zeta_input_classifier::InputClassificationContext;
use zeta_input_classifier::InputClassifier;
use zeta_input_classifier::InputConversation;
use zeta_input_classifier::InputHistoryEntry;
use zeta_input_classifier::InputRoute;
use zeta_input_classifier::ShellCompletionSnapshot;
use zeta_slash_commands::SlashCommandCatalogError;
use zeta_slash_commands::SlashCommandDefinition;
use zeta_ui_components::ScrollAxis;
use zeta_ui_components::ScrollCommand;
use zeta_ui_components::ScrollMetrics;
use zeta_ui_components::ScrollState;
use zui::ui::Point;
use zui::ui::Rect;
use zui::ui::Size;
use zui::ui::TextInputCompositionEvent;

/// Current classifier-selected submission route for the Session ChatInput.
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

/// Input, routing, history, and completion state owned by one Session Pane.
pub struct ChatInput {
    input: ChatInputEditor,
    interaction: ChatInputInteractionState,
    interaction_scroll: ScrollState,
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

impl Default for ChatInput {
    fn default() -> Self {
        Self::for_working_directory(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }
}

impl ChatInput {
    pub fn for_working_directory(working_directory: impl Into<PathBuf>) -> Self {
        zeta_input_classifier::start_background_warmup();
        Self {
            input: ChatInputEditor::default(),
            interaction: ChatInputInteractionState::new(),
            interaction_scroll: ScrollState::default(),
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

    pub const fn input(&self) -> &ChatInputEditor {
        &self.input
    }

    pub const fn interaction(&self) -> &ChatInputInteractionState {
        &self.interaction
    }

    pub(crate) const fn interaction_scroll(&self) -> ScrollState {
        self.interaction_scroll
    }

    pub fn set_interaction_catalog(
        &mut self,
        slash_commands: Vec<SlashCommandDefinition>,
        models: Vec<ComposerModelOption>,
    ) -> Result<(), SlashCommandCatalogError> {
        self.update_interaction(|interaction| interaction.set_catalog(slash_commands, models))
    }

    pub fn move_interaction_selection(&mut self, direction: SelectionDirection) {
        self.update_interaction(|interaction| interaction.move_selection(direction));
    }

    pub fn activate_selected_interaction(&mut self) -> Option<ComposerInteractionActivation> {
        self.update_interaction(ChatInputInteractionState::activate_selected)
    }

    pub fn complete_selected_slash(&mut self) -> Option<String> {
        self.update_interaction(ChatInputInteractionState::complete_selected_slash)
    }

    pub fn dismiss_interaction(&mut self) -> bool {
        let text = self.input.text().to_owned();
        self.update_interaction(|interaction| interaction.dismiss(&text))
    }

    pub fn select_interaction_item(&mut self, index: usize) -> bool {
        self.update_interaction(|interaction| interaction.select_item(index))
    }

    pub fn reset_interaction_scroll(&mut self) {
        self.interaction_scroll = ScrollState::default();
    }

    pub fn scroll_interaction(
        &mut self,
        command: ScrollCommand,
        viewport: Size,
        content: Size,
    ) -> bool {
        self.interaction_scroll.apply(
            command,
            ScrollMetrics::new(viewport, content),
            ScrollAxis::Vertical,
        )
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

    #[cfg(test)]
    pub fn mark_agent_turn_completed(&mut self) {
        if self.agent_response == AgentResponseState::Pending {
            self.conversation = InputConversation::AgentFollowUp;
            self.agent_response = AgentResponseState::None;
            self.refresh_classification();
        }
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
        let text = self.input.text().to_owned();
        let route = self.route;
        self.update_interaction(|interaction| interaction.sync_input(&text, route));
    }

    fn update_interaction<R>(
        &mut self,
        update: impl FnOnce(&mut ChatInputInteractionState) -> R,
    ) -> R {
        let previous_surface = self.interaction.surface();
        let result = update(&mut self.interaction);
        if previous_surface != self.interaction.surface() {
            self.reset_interaction_scroll();
        }
        result
    }

    fn leave_shell_history(&mut self) {
        self.shell_history_index = None;
        self.shell_history_draft = None;
    }
}

#[cfg(test)]
#[path = "chat_input/chat_input_tests.rs"]
mod tests;
