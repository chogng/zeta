mod attachments;
mod chat_composer;
mod mentions;
mod pending_pastes;
mod selection;
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
use selection::SelectionInputOutcome;
pub(crate) use selection::SelectionItem;
pub(crate) use selection::SelectionItemId;
pub(crate) use selection::SelectionTab;
pub(crate) use selection::SelectionViewModel;
pub(crate) use selection::SelectionViewState;
pub(crate) use slash_command_popup::SlashPopupView;
pub(crate) use slash_commands::DynamicSlashCommand;
pub(crate) use slash_commands::SlashCommand;
pub(crate) use slash_commands::SlashCommandArgumentMode;
pub(crate) use slash_commands::SlashCommandItem;
pub(crate) use slash_commands::SlashCommandRegistry;
pub(crate) use slash_commands::built_in_slash_commands;
use zeta_file_search::PathSearchSnapshot;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InteractionPaneOutcome {
    ActivateSelectionItem(SelectionItemId),
    Command(SlashCommandInvocation),
    Consumed,
    Submit(ComposerSubmission),
    Unhandled,
    ViewDismissed,
}

/// Owns the active interaction surface used by the sibling chat widget.
///
/// The composer remains alive while temporary views are stacked above it, preserving draft state
/// when a selection flow is dismissed. Slash semantics stay in the composer and product feature
/// state stays outside this presentation container.
#[derive(Debug)]
pub(crate) struct InteractionPane {
    composer: ChatComposer,
    views: Vec<InteractionView>,
}

#[derive(Debug)]
enum InteractionView {
    Selection(SelectionViewState),
}

impl InteractionPane {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            composer: ChatComposer::new(),
            views: Vec::new(),
        }
    }

    pub(crate) fn with_slash_commands(slash_commands: SlashCommandRegistry) -> Self {
        Self {
            composer: ChatComposer::with_slash_commands(slash_commands),
            views: Vec::new(),
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> InteractionPaneOutcome {
        if let Some(InteractionView::Selection(view)) = self.views.last_mut() {
            return match view.handle_key(key) {
                SelectionInputOutcome::Activate(item_id) => {
                    InteractionPaneOutcome::ActivateSelectionItem(item_id)
                }
                SelectionInputOutcome::Consumed => InteractionPaneOutcome::Consumed,
                SelectionInputOutcome::Dismiss => {
                    self.views.pop();
                    InteractionPaneOutcome::ViewDismissed
                }
            };
        }
        map_composer_outcome(self.composer.handle_key(key))
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, text: &str) {
        self.composer.insert_text(text);
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) -> Result<(), String> {
        if let Some(InteractionView::Selection(view)) = self.views.last_mut() {
            view.handle_paste(pasted);
            return Ok(());
        }
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
        if !self.views.is_empty() {
            return None;
        }
        self.composer.slash_popup()
    }

    pub(crate) fn mention_popup(&self) -> Option<MentionPopupView<'_>> {
        if !self.views.is_empty() {
            return None;
        }
        self.composer.mention_popup()
    }

    pub(crate) fn mention_query(&self) -> Option<&str> {
        if !self.views.is_empty() {
            return None;
        }
        self.composer.mention_query()
    }

    pub(crate) fn apply_file_search_snapshot(&mut self, snapshot: PathSearchSnapshot) {
        self.composer.apply_file_search_snapshot(snapshot);
    }

    pub(crate) fn activate_slash_command(
        &mut self,
        index: usize,
    ) -> Option<InteractionPaneOutcome> {
        if !self.views.is_empty() {
            return None;
        }
        self.composer
            .activate_slash_command(index)
            .map(map_composer_outcome)
    }

    pub(crate) fn activate_mention(&mut self, index: usize) -> bool {
        if !self.views.is_empty() {
            return false;
        }
        self.composer.activate_mention(index)
    }

    pub(crate) fn show_selection_view(&mut self, model: SelectionViewModel) {
        self.views
            .push(InteractionView::Selection(SelectionViewState::new(model)));
    }

    pub(crate) fn replace_selection_view(&mut self, model: SelectionViewModel) {
        match self.views.last_mut() {
            Some(InteractionView::Selection(view)) => view.replace_model(model),
            None => self.show_selection_view(model),
        }
    }

    pub(crate) fn selection_view(&self) -> Option<&SelectionViewState> {
        match self.views.last() {
            Some(InteractionView::Selection(view)) => Some(view),
            None => None,
        }
    }
}

fn map_composer_outcome(outcome: ComposerOutcome) -> InteractionPaneOutcome {
    match outcome {
        ComposerOutcome::Command(command) => InteractionPaneOutcome::Command(command),
        ComposerOutcome::Consumed => InteractionPaneOutcome::Consumed,
        ComposerOutcome::Submit(prompt) => InteractionPaneOutcome::Submit(prompt),
        ComposerOutcome::Unhandled => InteractionPaneOutcome::Unhandled,
    }
}

#[cfg(test)]
#[path = "toppane_tests.rs"]
mod tests;
