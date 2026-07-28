mod attachments;
mod chat_composer;
mod mentions;
mod pending_pastes;
mod slash_command_popup;
mod slash_commands;
mod slash_input;
mod textarea;

use chat_composer::ChatComposer;
pub(crate) use chat_composer::ComposerInput;
use chat_composer::ComposerOutcome;
pub(crate) use chat_composer::ComposerSubmission;
pub(crate) use chat_composer::SlashCommandInvocation;
use crossterm::event::KeyEvent;
pub(crate) use mentions::MentionPopupView;
pub(crate) use slash_command_popup::SlashPopupView;
pub(crate) use slash_commands::DynamicSlashCommand;
pub(crate) use slash_commands::SlashCommand;
pub(crate) use slash_commands::SlashCommandArgumentMode;
pub(crate) use slash_commands::SlashCommandItem;
pub(crate) use slash_commands::SlashCommandRegistry;
pub(crate) use slash_commands::built_in_slash_commands;
use zeta_file_search::PathSearchSnapshot;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum TopPaneOutcome {
    Command(SlashCommandInvocation),
    Consumed,
    Submit(ComposerSubmission),
    Unhandled,
}

/// Owns the active interaction surface used by the sibling chat widget.
///
/// The top pane routes input to its composer today. Future interaction modes can be added here
/// without moving slash semantics into `ChatWidget` or editor behavior into the application
/// coordinator.
#[derive(Debug)]
pub(crate) struct TopPane {
    composer: ChatComposer,
}

impl TopPane {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            composer: ChatComposer::new(),
        }
    }

    pub(crate) fn with_slash_commands(slash_commands: SlashCommandRegistry) -> Self {
        Self {
            composer: ChatComposer::with_slash_commands(slash_commands),
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> TopPaneOutcome {
        map_composer_outcome(self.composer.handle_key(key))
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, text: &str) {
        self.composer.insert_text(text);
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) -> Result<(), String> {
        self.composer.handle_paste(pasted)
    }

    pub(crate) fn attach_image_bytes(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        self.composer.attach_image_bytes(bytes)
    }

    pub(crate) fn text(&self) -> &str {
        self.composer.text()
    }

    pub(crate) fn cursor_display_width(&self) -> usize {
        self.composer.cursor_display_width()
    }

    pub(crate) fn slash_popup(&self) -> Option<SlashPopupView<'_>> {
        self.composer.slash_popup()
    }

    pub(crate) fn mention_popup(&self) -> Option<MentionPopupView<'_>> {
        self.composer.mention_popup()
    }

    pub(crate) fn mention_query(&self) -> Option<&str> {
        self.composer.mention_query()
    }

    pub(crate) fn apply_file_search_snapshot(&mut self, snapshot: PathSearchSnapshot) {
        self.composer.apply_file_search_snapshot(snapshot);
    }

    pub(crate) fn activate_slash_command(&mut self, index: usize) -> Option<TopPaneOutcome> {
        self.composer
            .activate_slash_command(index)
            .map(map_composer_outcome)
    }

    pub(crate) fn activate_mention(&mut self, index: usize) -> bool {
        self.composer.activate_mention(index)
    }
}

fn map_composer_outcome(outcome: ComposerOutcome) -> TopPaneOutcome {
    match outcome {
        ComposerOutcome::Command(command) => TopPaneOutcome::Command(command),
        ComposerOutcome::Consumed => TopPaneOutcome::Consumed,
        ComposerOutcome::Submit(prompt) => TopPaneOutcome::Submit(prompt),
        ComposerOutcome::Unhandled => TopPaneOutcome::Unhandled,
    }
}

#[cfg(test)]
#[path = "toppane_tests.rs"]
mod tests;
