use super::command::AppCommand;
use super::event::AppEvent;
use crate::components::interaction::InteractionPane;
use crate::components::interaction::InteractionPaneOutcome;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionViewModel;
use crate::components::selection::SelectionViewState;
use crate::components::transcript::Message;
use crate::features::skills::{SkillSelectionAction, SkillSelectionView};
use crate::features::status_line::StatusLineModel;
use crate::features::theme::ThemeSelectionAction;
use crate::features::theme::ThemeSelectionView;
use crate::features::thread::ThreadFeatureState;
use crate::features::thread::ThreadPresentationEvent;
use crate::features::thread::TurnActivity;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Status {
    Ready,
    Working,
    WaitingForApproval,
    WaitingForUserInput,
    WaitingForCapability,
    Cancelling,
    Error,
}

#[derive(Debug)]
pub(crate) struct App {
    interaction_pane: InteractionPane,
    thread: ThreadFeatureState,
    selection_actions: Vec<SelectionActions>,
    status: Status,
    status_line: StatusLineModel,
}

#[derive(Debug)]
enum SelectionActions {
    ReadOnly,
    Skills(BTreeMap<SelectionItemId, SkillSelectionAction>),
    Theme(BTreeMap<SelectionItemId, ThemeSelectionAction>),
}

impl App {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            interaction_pane: InteractionPane::new(),
            thread: ThreadFeatureState::default(),
            selection_actions: Vec::new(),
            status: Status::Ready,
            status_line: StatusLineModel::for_workspace(Path::new(".")),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_workspace(workspace_root: &Path) -> Self {
        Self::for_workspace_with_slash_commands(
            workspace_root,
            crate::components::composer::default_slash_command_catalog(),
        )
    }

    pub(crate) fn for_workspace_with_slash_commands(
        workspace_root: &Path,
        slash_commands: SlashCommandCatalog,
    ) -> Self {
        Self {
            interaction_pane: InteractionPane::with_slash_commands(slash_commands),
            thread: ThreadFeatureState::default(),
            selection_actions: Vec::new(),
            status: Status::Ready,
            status_line: StatusLineModel::for_workspace(workspace_root),
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Option<AppCommand> {
        if !self.accepts_input() {
            return self.handle_global_key(key);
        }

        let outcome = self.interaction_pane.handle_key(key);
        if matches!(outcome, InteractionPaneOutcome::Unhandled) {
            return self.handle_global_key(key);
        }
        self.handle_interaction_pane_outcome(outcome)
    }

    fn handle_interaction_pane_outcome(
        &mut self,
        outcome: InteractionPaneOutcome,
    ) -> Option<AppCommand> {
        match outcome {
            InteractionPaneOutcome::ActivateSelectionItem(item_id) => {
                self.activate_selection_item(&item_id)
            }
            InteractionPaneOutcome::Command(command) => self.handle_slash_command(command),
            InteractionPaneOutcome::Submit(submission) => {
                self.thread.update(ThreadPresentationEvent::UserSubmitted(
                    submission.display_text.clone(),
                ));
                self.status = Status::Working;
                Some(AppCommand::SubmitTurn(submission))
            }
            InteractionPaneOutcome::Consumed => None,
            InteractionPaneOutcome::Unhandled => None,
            InteractionPaneOutcome::ViewDismissed => {
                self.selection_actions.pop();
                None
            }
        }
    }

    fn activate_selection_item(&self, item_id: &SelectionItemId) -> Option<AppCommand> {
        match self.selection_actions.last()? {
            SelectionActions::ReadOnly => None,
            SelectionActions::Skills(actions) => match actions.get(item_id)? {
                SkillSelectionAction::SetEnablement {
                    skill_id,
                    enablement,
                } => Some(AppCommand::SetSkillEnablement {
                    skill_id: skill_id.clone(),
                    enablement: *enablement,
                }),
            },
            SelectionActions::Theme(actions) => match actions.get(item_id)? {
                ThemeSelectionAction::Select { preference } => Some(AppCommand::SetTheme {
                    preference: preference.clone(),
                }),
                ThemeSelectionAction::SelectCustom { preference } => {
                    Some(AppCommand::SetCustomTheme {
                        preference: preference.clone(),
                    })
                }
                ThemeSelectionAction::OpenCustomThemes => Some(AppCommand::OpenCustomThemePane),
            },
        }
    }

    pub(crate) fn activate_slash_command(&mut self, index: usize) -> Option<AppCommand> {
        if !self.accepts_input() {
            return None;
        }
        let outcome = self.interaction_pane.activate_slash_command(index)?;
        self.handle_interaction_pane_outcome(outcome)
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, text: &str) {
        if self.accepts_input() {
            self.interaction_pane.insert_text(text);
        }
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        if self.accepts_input()
            && let Err(error) = self.interaction_pane.handle_paste(pasted)
        {
            self.thread
                .update(ThreadPresentationEvent::FailureReported(error));
        }
    }

    fn attach_image_bytes(&mut self, bytes: Vec<u8>) {
        if self.accepts_input()
            && let Err(error) = self.interaction_pane.attach_image_bytes(bytes)
        {
            self.thread
                .update(ThreadPresentationEvent::FailureReported(error));
        }
    }

    fn record_clipboard_error(&mut self, error: String) {
        if self.accepts_input() {
            self.thread
                .update(ThreadPresentationEvent::FailureReported(format!(
                    "could not paste clipboard image: {error}"
                )));
        }
    }

    pub(crate) fn input(&self) -> &str {
        self.interaction_pane.text()
    }

    pub(crate) fn input_cursor_width(&self) -> usize {
        self.interaction_pane.cursor_display_width()
    }

    pub(crate) fn slash_popup(&self) -> Option<SlashCommandsView<'_>> {
        self.interaction_pane.slash_popup()
    }

    pub(crate) fn mention_popup(&self) -> Option<MentionPopupView<'_>> {
        self.interaction_pane.mention_popup()
    }

    fn show_selection_view(&mut self, model: SelectionViewModel) {
        self.interaction_pane.show_selection_view(model);
        self.selection_actions.push(SelectionActions::ReadOnly);
    }

    fn show_skills_view(&mut self, view: SkillSelectionView) {
        self.interaction_pane.show_selection_view(view.model);
        self.selection_actions
            .push(SelectionActions::Skills(view.actions));
    }

    fn replace_skills_view(&mut self, view: SkillSelectionView) {
        self.interaction_pane.replace_selection_view(view.model);
        match self.selection_actions.last_mut() {
            Some(actions) => *actions = SelectionActions::Skills(view.actions),
            None => self
                .selection_actions
                .push(SelectionActions::Skills(view.actions)),
        }
    }

    fn show_theme_view(&mut self, view: ThemeSelectionView) {
        self.interaction_pane.show_selection_view(view.model);
        self.selection_actions
            .push(SelectionActions::Theme(view.actions));
    }

    fn close_theme_views(&mut self) {
        while matches!(
            self.selection_actions.last(),
            Some(SelectionActions::Theme(_))
        ) {
            self.interaction_pane.pop_selection_view();
            self.selection_actions.pop();
        }
    }

    pub(crate) fn skills_view_is_active(&self) -> bool {
        matches!(
            self.selection_actions.last(),
            Some(SelectionActions::Skills(_))
        )
    }

    pub(crate) fn selection_view(&self) -> Option<&SelectionViewState> {
        self.interaction_pane.selection_view()
    }

    pub(crate) fn activate_mention(&mut self, index: usize) -> bool {
        self.accepts_input() && self.interaction_pane.activate_mention(index)
    }

    pub(crate) fn mention_query(&self) -> Option<&str> {
        self.interaction_pane.mention_query()
    }

    pub(crate) fn messages(&self) -> &[Message] {
        self.thread.messages()
    }

    pub(crate) fn status(&self) -> &Status {
        &self.status
    }

    pub(crate) fn status_line(&self) -> &StatusLineModel {
        &self.status_line
    }

    pub(crate) fn accepts_input(&self) -> bool {
        matches!(&self.status, Status::Ready | Status::Error)
    }

    pub(crate) fn update(&mut self, event: AppEvent) {
        match event {
            AppEvent::ClipboardImageRead(Ok(bytes)) => self.attach_image_bytes(bytes),
            AppEvent::ClipboardImageRead(Err(error)) => self.record_clipboard_error(error),
            AppEvent::ConfigSnapshotReceived(config) => self.status_line.apply_config(&config),
            AppEvent::FailureReported(error) => {
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(error));
                self.status = Status::Error;
            }
            AppEvent::FileSearchSnapshotReceived(snapshot) => {
                self.interaction_pane.apply_file_search_snapshot(snapshot);
            }
            AppEvent::InterruptFailed(error) => {
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(format!(
                        "could not interrupt turn: {error}"
                    )));
                self.status = Status::Working;
            }
            AppEvent::ProductNotice(notice) => {
                self.thread
                    .update(ThreadPresentationEvent::NoticeReceived(notice));
                self.status = Status::Ready;
            }
            AppEvent::SelectionViewOpened(model) => self.show_selection_view(model),
            AppEvent::SkillsViewOpened(view) => self.show_skills_view(view),
            AppEvent::SkillsViewReplaced(view) => self.replace_skills_view(view),
            AppEvent::ThemeViewClosed => self.close_theme_views(),
            AppEvent::ThemeViewOpened(view) => self.show_theme_view(view),
            AppEvent::ThreadSnapshotReceived(thread) => self
                .thread
                .update(ThreadPresentationEvent::SnapshotReceived(thread)),
            AppEvent::TranscriptCleared => {
                self.thread.update(ThreadPresentationEvent::Cleared);
                self.status = Status::Ready;
            }
            AppEvent::TurnActivityChanged(activity) => {
                self.status = match activity {
                    TurnActivity::Working => Status::Working,
                    TurnActivity::WaitingForApproval => Status::WaitingForApproval,
                    TurnActivity::WaitingForUserInput => Status::WaitingForUserInput,
                    TurnActivity::WaitingForCapability => Status::WaitingForCapability,
                    TurnActivity::Cancelling => Status::Cancelling,
                };
            }
            AppEvent::TurnCompleted => {
                self.status = Status::Ready;
            }
            AppEvent::TurnInterrupted => {
                self.thread.update(ThreadPresentationEvent::Interrupted);
                self.status = Status::Ready;
            }
        }
    }

    fn handle_global_key(&mut self, key: KeyEvent) -> Option<AppCommand> {
        if self.accepts_input()
            && key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char(character) if character.eq_ignore_ascii_case(&'v'))
        {
            return Some(AppCommand::ReadClipboardImage);
        }
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            return match key.code {
                KeyCode::Char('c') => self.quit_or_interrupt(),
                KeyCode::Char('d') if self.input().is_empty() => self.quit_or_interrupt(),
                _ => None,
            };
        }
        if key.code == KeyCode::Esc {
            return self.quit_or_interrupt();
        }
        None
    }

    fn handle_slash_command(&mut self, invocation: SlashCommandInvocation) -> Option<AppCommand> {
        let local = invocation
            .command
            .name
            .parse::<TuiSlashCommandAction>()
            .ok();
        match (invocation.origin, local) {
            (
                SlashCommandOrigin::Local,
                Some(TuiSlashCommandAction::Quit | TuiSlashCommandAction::Exit),
            ) if invocation.arguments.is_empty() => Some(AppCommand::Quit),
            (SlashCommandOrigin::Server, _) => {
                let submission = invocation.into_forwarded_submission();
                self.thread.update(ThreadPresentationEvent::UserSubmitted(
                    submission.display_text.clone(),
                ));
                self.status = Status::Working;
                Some(AppCommand::SubmitTurn(submission))
            }
            (SlashCommandOrigin::Local, Some(_)) => {
                Some(AppCommand::ExecuteProductCommand(invocation))
            }
            (SlashCommandOrigin::Local, None) => None,
        }
    }

    fn quit_or_interrupt(&mut self) -> Option<AppCommand> {
        match &self.status {
            Status::Working
            | Status::WaitingForApproval
            | Status::WaitingForUserInput
            | Status::WaitingForCapability => {
                self.status = Status::Cancelling;
                Some(AppCommand::Interrupt)
            }
            Status::Cancelling => None,
            Status::Ready | Status::Error => Some(AppCommand::Quit),
        }
    }
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
use crate::components::composer::MentionPopupView;
use crate::components::composer::SlashCommandCatalog;
use crate::components::composer::SlashCommandInvocation;
use crate::components::composer::SlashCommandsView;
use crate::components::composer::TuiSlashCommandAction;
use zeta_slash_commands::SlashCommandOrigin;
