use crate::components::composer::ChatComposer;
use crate::components::composer::ComposerOutcome;
use crate::components::composer::ComposerSubmission;
use crate::components::composer::MentionPopupView;
use crate::components::composer::SlashCommandCatalog;
use crate::components::composer::SlashCommandInvocation;
use crate::components::composer::SlashCommandsView;
use crate::components::pane::PaneView;
use crate::components::pane::PaneViewModel;
use crate::components::selection::SelectionInputOutcome;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionViewModel;
use crate::components::selection::SelectionViewState;
use crate::mouse::MouseMode;
use crossterm::event::KeyEvent;
use std::collections::BTreeMap;
use zeta_file_search::PathSearchSnapshot;
use zeta_protocol::SkillRef;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InteractionPaneOutcome {
    ActivateSelectionItem(SelectionItemId),
    ActivateSelectionFreeForm {
        item_id: SelectionItemId,
        value: String,
    },
    Command(SlashCommandInvocation),
    Consumed,
    Submit(ComposerSubmission),
    Unhandled,
    ViewDismissed,
}

/// Owns focus and routing for the composer and temporary interaction views.
///
/// The composer remains alive while temporary views are stacked above it, preserving draft state
/// when a selection flow is dismissed. Product feature state remains outside this component.
#[derive(Debug)]
pub(crate) struct InteractionPane {
    composer: ChatComposer,
    views: Vec<InteractionView>,
}

#[derive(Debug)]
enum InteractionView {
    Selection(PaneView<SelectionViewState>),
}

impl InteractionPane {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            composer: ChatComposer::new(),
            views: Vec::new(),
        }
    }

    pub(crate) fn with_slash_commands(slash_commands: SlashCommandCatalog) -> Self {
        Self {
            composer: ChatComposer::with_slash_commands(slash_commands),
            views: Vec::new(),
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> InteractionPaneOutcome {
        if let Some(InteractionView::Selection(view)) = self.views.last_mut() {
            return match view.body_mut().handle_key(key) {
                SelectionInputOutcome::Activate(item_id) => {
                    InteractionPaneOutcome::ActivateSelectionItem(item_id)
                }
                SelectionInputOutcome::ActivateFreeForm { item_id, value } => {
                    InteractionPaneOutcome::ActivateSelectionFreeForm { item_id, value }
                }
                SelectionInputOutcome::Consumed => InteractionPaneOutcome::Consumed,
                SelectionInputOutcome::Dismiss => {
                    self.views.pop();
                    InteractionPaneOutcome::ViewDismissed
                }
                SelectionInputOutcome::Unhandled => InteractionPaneOutcome::Unhandled,
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
            view.body_mut().handle_paste(pasted);
            return Ok(());
        }
        self.composer.handle_paste(pasted)
    }

    pub(crate) fn attach_image_bytes(&mut self, bytes: Vec<u8>) -> Result<(), String> {
        self.composer.attach_image_bytes(bytes)
    }

    pub(crate) fn replace_slash_commands(
        &mut self,
        slash_commands: SlashCommandCatalog,
        skill_commands: BTreeMap<String, SkillRef>,
    ) {
        self.composer
            .replace_slash_commands(slash_commands, skill_commands);
    }

    pub(crate) fn text(&self) -> &str {
        self.composer.text()
    }

    pub(crate) fn cursor_display_width(&self) -> usize {
        self.composer.cursor_display_width()
    }

    pub(crate) fn cursor_line(&self) -> usize {
        self.composer.cursor_line()
    }

    pub(crate) fn composer_desired_height(&self, available_width: u16) -> u16 {
        self.composer.desired_height(available_width)
    }

    pub(crate) fn slash_popup(&self) -> Option<SlashCommandsView<'_>> {
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

    pub(crate) fn mouse_mode(&self) -> MouseMode {
        if self.slash_popup().is_some() || self.mention_popup().is_some() {
            MouseMode::UiClick
        } else {
            MouseMode::TerminalSelection
        }
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

    pub(crate) fn select_slash_command(&mut self, index: usize) -> bool {
        self.views.is_empty() && self.composer.select_slash_command(index)
    }

    pub(crate) fn activate_mention(&mut self, index: usize) -> bool {
        if !self.views.is_empty() {
            return false;
        }
        self.composer.activate_mention(index)
    }

    pub(crate) fn select_mention(&mut self, index: usize) -> bool {
        self.views.is_empty() && self.composer.select_mention(index)
    }

    pub(crate) fn show_selection_view(&mut self, model: PaneViewModel<SelectionViewModel>) {
        let (body, key_hints) = model.into_parts();
        self.views.push(InteractionView::Selection(PaneView::new(
            SelectionViewState::new(body),
            key_hints,
        )));
    }

    pub(crate) fn replace_selection_view(&mut self, model: PaneViewModel<SelectionViewModel>) {
        let (body, key_hints) = model.into_parts();
        match self.views.last_mut() {
            Some(InteractionView::Selection(view)) => {
                view.body_mut().replace_model(body);
                view.replace_key_hints(key_hints);
            }
            None => self.show_selection_view(PaneViewModel::new(body, key_hints)),
        }
    }

    pub(crate) fn pop_selection_view(&mut self) {
        if matches!(self.views.last(), Some(InteractionView::Selection(_))) {
            self.views.pop();
        }
    }

    pub(crate) fn selection_view(&self) -> Option<&SelectionViewState> {
        match self.views.last() {
            Some(InteractionView::Selection(view)) => Some(view.body()),
            None => None,
        }
    }

    pub(crate) fn selection_pane(&self) -> Option<&PaneView<SelectionViewState>> {
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
#[path = "state_tests.rs"]
mod tests;
