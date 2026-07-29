use crate::toppane::ComposerSubmission;
use crate::toppane::InteractionPane;
use crate::toppane::InteractionPaneOutcome;
use crate::toppane::MentionPopupView;
use crate::toppane::SelectionItemId;
use crate::toppane::SelectionViewModel;
use crate::toppane::SelectionViewState;
use crate::toppane::SlashCommandInvocation;
use crate::toppane::SlashCommandRegistry;
use crate::toppane::SlashPopupView;
use crossterm::event::KeyEvent;
use zeta_file_search::PathSearchSnapshot;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MessageRole {
    User,
    Agent,
    Notice,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Message {
    pub(crate) role: MessageRole,
    pub(crate) text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ChatWidgetOutcome {
    ActivateSelectionItem(SelectionItemId),
    Command(SlashCommandInvocation),
    Consumed,
    Submit(ComposerSubmission),
    Unhandled,
    ViewDismissed,
}

/// Coordinates conversation presentation and delegates interaction to the top pane.
///
/// The widget owns transcript state, but callers remain responsible for global lifecycle,
/// product status, and executing submitted actions. Editing and composer behavior remain owned by
/// the sibling `toppane` module.
#[derive(Debug)]
pub(crate) struct ChatWidget {
    messages: Vec<Message>,
    interaction_pane: InteractionPane,
}

impl ChatWidget {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            messages: Vec::new(),
            interaction_pane: InteractionPane::new(),
        }
    }

    pub(crate) fn with_slash_commands(slash_commands: SlashCommandRegistry) -> Self {
        Self {
            messages: Vec::new(),
            interaction_pane: InteractionPane::with_slash_commands(slash_commands),
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> ChatWidgetOutcome {
        let outcome = self.interaction_pane.handle_key(key);
        self.map_interaction_pane_outcome(outcome)
    }

    fn map_interaction_pane_outcome(
        &mut self,
        outcome: InteractionPaneOutcome,
    ) -> ChatWidgetOutcome {
        match outcome {
            InteractionPaneOutcome::ActivateSelectionItem(item_id) => {
                ChatWidgetOutcome::ActivateSelectionItem(item_id)
            }
            InteractionPaneOutcome::Command(command) => ChatWidgetOutcome::Command(command),
            InteractionPaneOutcome::Consumed => ChatWidgetOutcome::Consumed,
            InteractionPaneOutcome::Submit(submission) => {
                self.messages.push(Message {
                    role: MessageRole::User,
                    text: submission.display_text.clone(),
                });
                ChatWidgetOutcome::Submit(submission)
            }
            InteractionPaneOutcome::Unhandled => ChatWidgetOutcome::Unhandled,
            InteractionPaneOutcome::ViewDismissed => ChatWidgetOutcome::ViewDismissed,
        }
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, text: &str) {
        self.interaction_pane.insert_text(text);
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) -> Result<(), String> {
        self.interaction_pane.handle_paste(pasted)
    }

    pub(crate) fn attach_image_bytes(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        self.interaction_pane.attach_image_bytes(bytes)
    }

    pub(crate) fn draft(&self) -> &str {
        self.interaction_pane.text()
    }

    pub(crate) fn draft_cursor_width(&self) -> usize {
        self.interaction_pane.cursor_display_width()
    }

    pub(crate) fn slash_popup(&self) -> Option<SlashPopupView<'_>> {
        self.interaction_pane.slash_popup()
    }

    pub(crate) fn mention_popup(&self) -> Option<MentionPopupView<'_>> {
        self.interaction_pane.mention_popup()
    }

    pub(crate) fn mention_query(&self) -> Option<&str> {
        self.interaction_pane.mention_query()
    }

    pub(crate) fn apply_file_search_snapshot(&mut self, snapshot: PathSearchSnapshot) {
        self.interaction_pane.apply_file_search_snapshot(snapshot);
    }

    pub(crate) fn activate_slash_command(&mut self, index: usize) -> Option<ChatWidgetOutcome> {
        let outcome = self.interaction_pane.activate_slash_command(index)?;
        Some(self.map_interaction_pane_outcome(outcome))
    }

    pub(crate) fn activate_mention(&mut self, index: usize) -> bool {
        self.interaction_pane.activate_mention(index)
    }

    pub(crate) fn show_selection_view(&mut self, model: SelectionViewModel) {
        self.interaction_pane.show_selection_view(model);
    }

    pub(crate) fn replace_selection_view(&mut self, model: SelectionViewModel) {
        self.interaction_pane.replace_selection_view(model);
    }

    pub(crate) fn selection_view(&self) -> Option<&SelectionViewState> {
        self.interaction_pane.selection_view()
    }

    pub(crate) fn messages(&self) -> &[Message] {
        &self.messages
    }

    pub(crate) fn push_message(&mut self, role: MessageRole, text: String) {
        self.messages.push(Message { role, text });
    }

    pub(crate) fn clear_messages(&mut self) {
        self.messages.clear();
    }
}

#[cfg(test)]
#[path = "chatwidget_tests.rs"]
mod tests;
