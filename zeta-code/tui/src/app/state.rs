use super::command::AppCommand;
use super::event::AppEvent;
use crate::components::composer::ComposerInput;
use crate::components::interaction::InteractionPane;
use crate::components::interaction::InteractionPaneOutcome;
use crate::components::pane::PaneView;
use crate::components::pane::PaneViewModel;
use crate::components::selection::SelectionItemId;
use crate::components::selection::SelectionViewModel;
use crate::components::selection::SelectionViewState;
use crate::components::transcript::Message;
use crate::components::transcript::TranscriptScroll;
use crate::features::interactions::InteractionSelectionOutcome;
use crate::features::interactions::InteractionSelectionState;
use crate::features::interactions::InteractionSelectionView;
use crate::features::mcp::McpSelectionAction;
use crate::features::mcp::McpSelectionView;
use crate::features::models::ModelSelectionAction;
use crate::features::models::ModelSelectionView;
use crate::features::rewind::RewindSelectionAction;
use crate::features::rewind::RewindSelectionView;
use crate::features::sessions::SessionSelectionAction;
use crate::features::sessions::SessionSelectionView;
use crate::features::sessions::ThreadSelectionAction;
use crate::features::sessions::ThreadSelectionView;
use crate::features::skills::{SkillSelectionAction, SkillSelectionView};
use crate::features::status_line::StatusLineModel;
use crate::features::theme::ThemeSelectionAction;
use crate::features::theme::ThemeSelectionView;
use crate::features::thread::ThreadFeatureState;
use crate::features::thread::ThreadPresentationEvent;
use crate::features::thread::TurnActivity;
use crate::features::workspace_files::FileSelectionAction;
use crate::features::workspace_files::FileSelectionView;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;
use zeta_protocol::ApprovalMode;

const DOUBLE_ESCAPE_WINDOW: Duration = Duration::from_millis(500);

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
    transcript_scroll: TranscriptScroll,
    selection_actions: Vec<SelectionActions>,
    last_root_escape: Option<Instant>,
    status: Status,
    status_line: StatusLineModel,
    approval_mode: ApprovalMode,
}

#[derive(Debug)]
enum SelectionActions {
    ReadOnly,
    Interaction(InteractionSelectionState),
    Mcp(BTreeMap<SelectionItemId, McpSelectionAction>),
    Files(BTreeMap<SelectionItemId, FileSelectionAction>),
    Model(BTreeMap<SelectionItemId, ModelSelectionAction>),
    Rewind(BTreeMap<SelectionItemId, RewindSelectionAction>),
    Sessions(BTreeMap<SelectionItemId, SessionSelectionAction>),
    Threads(BTreeMap<SelectionItemId, ThreadSelectionAction>),
    Skills(BTreeMap<SelectionItemId, SkillSelectionAction>),
    Theme(BTreeMap<SelectionItemId, ThemeSelectionAction>),
}

impl App {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            interaction_pane: InteractionPane::new(),
            thread: ThreadFeatureState::default(),
            transcript_scroll: TranscriptScroll::default(),
            selection_actions: Vec::new(),
            last_root_escape: None,
            status: Status::Ready,
            status_line: StatusLineModel::for_workspace(Path::new(".")),
            approval_mode: ApprovalMode::AskPermissions,
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
            transcript_scroll: TranscriptScroll::default(),
            selection_actions: Vec::new(),
            last_root_escape: None,
            status: Status::Ready,
            status_line: StatusLineModel::for_workspace(workspace_root),
            approval_mode: ApprovalMode::AskPermissions,
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Option<AppCommand> {
        self.handle_key_at(key, Instant::now())
    }

    fn handle_key_at(&mut self, key: KeyEvent, now: Instant) -> Option<AppCommand> {
        let temporary_interaction_active = self.selection_view().is_some()
            || self.slash_popup().is_some()
            || self.mention_popup().is_some();
        if key.kind == KeyEventKind::Press
            && (key.code != KeyCode::Esc || temporary_interaction_active)
        {
            self.last_root_escape = None;
        }
        if !self.accepts_input() {
            self.last_root_escape = None;
            return self.handle_global_key(key, now);
        }

        let outcome = self.interaction_pane.handle_key(key);
        if matches!(outcome, InteractionPaneOutcome::Unhandled) {
            return self.handle_global_key(key, now);
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
            InteractionPaneOutcome::ActivateSelectionFreeForm { item_id, value } => {
                self.activate_selection_free_form(&item_id, value)
            }
            InteractionPaneOutcome::Command(command) => self.handle_slash_command(command),
            InteractionPaneOutcome::Submit(submission) => {
                self.thread.update(ThreadPresentationEvent::UserSubmitted(
                    submission.display_text.clone(),
                ));
                self.status = Status::Working;
                Some(AppCommand::SubmitTurn {
                    submission,
                    approval_mode: self.approval_mode,
                })
            }
            InteractionPaneOutcome::Consumed => None,
            InteractionPaneOutcome::Unhandled => None,
            InteractionPaneOutcome::ViewDismissed => {
                self.selection_actions.pop();
                None
            }
        }
    }

    fn activate_selection_item(&mut self, item_id: &SelectionItemId) -> Option<AppCommand> {
        if let Some(SelectionActions::Interaction(state)) = self.selection_actions.last_mut() {
            let outcome = state.activate_item(item_id)?;
            return self.apply_interaction_selection_outcome(outcome);
        }
        match self.selection_actions.last()? {
            SelectionActions::ReadOnly => None,
            SelectionActions::Interaction(_) => None,
            SelectionActions::Mcp(actions) => match actions.get(item_id)? {
                McpSelectionAction::SetEnablement {
                    server_id,
                    enablement,
                } => Some(AppCommand::SetMcpEnablement {
                    server_id: server_id.clone(),
                    enablement: *enablement,
                }),
            },
            SelectionActions::Files(actions) => match actions.get(item_id)? {
                FileSelectionAction::OpenDirectory { path } => {
                    Some(AppCommand::OpenWorkspaceDirectory { path: path.clone() })
                }
                FileSelectionAction::PreviewFile { path } => {
                    Some(AppCommand::PreviewWorkspaceFile { path: path.clone() })
                }
            },
            SelectionActions::Model(actions) => match actions.get(item_id)? {
                ModelSelectionAction::Select { preference } => {
                    Some(AppCommand::SetPreferredModel {
                        preference: preference.clone(),
                    })
                }
            },
            SelectionActions::Rewind(actions) => match actions.get(item_id)? {
                RewindSelectionAction::Rewind {
                    before_turn_id,
                    checkpoint_label,
                } => Some(AppCommand::RewindToCheckpoint {
                    before_turn_id: before_turn_id.clone(),
                    checkpoint_label: checkpoint_label.clone(),
                }),
            },
            SelectionActions::Sessions(actions) => match actions.get(item_id)? {
                SessionSelectionAction::Resume { session_id } => Some(AppCommand::ResumeSession {
                    session_id: session_id.clone(),
                }),
            },
            SelectionActions::Threads(actions) => match actions.get(item_id)? {
                ThreadSelectionAction::Archive { thread_id } => Some(AppCommand::ArchiveThread {
                    thread_id: thread_id.clone(),
                }),
                ThreadSelectionAction::Switch { thread_id } => Some(AppCommand::SwitchThread {
                    thread_id: thread_id.clone(),
                }),
            },
            SelectionActions::Skills(actions) => match actions.get(item_id)?.clone() {
                SkillSelectionAction::SetEnablement {
                    skill_id,
                    enablement,
                } => Some(AppCommand::SetSkillEnablement {
                    skill_id,
                    enablement,
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

    fn activate_selection_free_form(
        &mut self,
        item_id: &SelectionItemId,
        value: String,
    ) -> Option<AppCommand> {
        let Some(SelectionActions::Interaction(state)) = self.selection_actions.last_mut() else {
            return None;
        };
        let outcome = state.activate_free_form(item_id, value)?;
        self.apply_interaction_selection_outcome(outcome)
    }

    fn apply_interaction_selection_outcome(
        &mut self,
        outcome: InteractionSelectionOutcome,
    ) -> Option<AppCommand> {
        match outcome {
            InteractionSelectionOutcome::Continue(model) => {
                self.interaction_pane.replace_selection_view(model);
                None
            }
            InteractionSelectionOutcome::Resolve(response) => {
                Some(AppCommand::ResolveInteraction(response))
            }
        }
    }

    pub(crate) fn activate_slash_command(&mut self, index: usize) -> Option<AppCommand> {
        if !self.accepts_input() {
            return None;
        }
        let outcome = self.interaction_pane.activate_slash_command(index)?;
        self.handle_interaction_pane_outcome(outcome)
    }

    pub(crate) fn replace_slash_commands(
        &mut self,
        slash_commands: SlashCommandCatalog,
        skill_commands: BTreeMap<String, zeta_protocol::SkillRef>,
    ) {
        self.interaction_pane
            .replace_slash_commands(slash_commands, skill_commands);
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

    pub(crate) fn input_cursor_line(&self) -> usize {
        self.interaction_pane.cursor_line()
    }

    pub(crate) fn composer_desired_height(&self, available_width: u16) -> u16 {
        self.interaction_pane
            .composer_desired_height(available_width)
    }

    pub(crate) fn slash_popup(&self) -> Option<SlashCommandsView<'_>> {
        self.interaction_pane.slash_popup()
    }

    pub(crate) fn mention_popup(&self) -> Option<MentionPopupView<'_>> {
        self.interaction_pane.mention_popup()
    }

    fn show_selection_view(&mut self, model: PaneViewModel<SelectionViewModel>) {
        self.interaction_pane.show_selection_view(model);
        self.selection_actions.push(SelectionActions::ReadOnly);
    }

    fn show_interaction_view(&mut self, view: InteractionSelectionView) {
        self.interaction_pane.show_selection_view(view.model);
        self.selection_actions
            .push(SelectionActions::Interaction(view.state));
    }

    fn show_skills_view(&mut self, view: SkillSelectionView) {
        self.interaction_pane.show_selection_view(view.model);
        self.selection_actions
            .push(SelectionActions::Skills(view.actions));
    }

    fn show_mcp_view(&mut self, view: McpSelectionView) {
        self.interaction_pane.show_selection_view(view.model);
        self.selection_actions
            .push(SelectionActions::Mcp(view.actions));
    }

    fn show_file_view(&mut self, view: FileSelectionView) {
        self.interaction_pane.show_selection_view(view.model);
        self.selection_actions
            .push(SelectionActions::Files(view.actions));
    }

    fn replace_mcp_view(&mut self, view: McpSelectionView) {
        self.interaction_pane.replace_selection_view(view.model);
        match self.selection_actions.last_mut() {
            Some(actions) => *actions = SelectionActions::Mcp(view.actions),
            None => self
                .selection_actions
                .push(SelectionActions::Mcp(view.actions)),
        }
    }

    fn show_model_view(&mut self, view: ModelSelectionView) {
        self.interaction_pane.show_selection_view(view.model);
        self.selection_actions
            .push(SelectionActions::Model(view.actions));
    }

    fn show_rewind_view(&mut self, view: RewindSelectionView) {
        self.interaction_pane.show_selection_view(view.model);
        self.selection_actions
            .push(SelectionActions::Rewind(view.actions));
    }

    fn show_session_view(&mut self, view: SessionSelectionView) {
        self.interaction_pane.show_selection_view(view.model);
        self.selection_actions
            .push(SelectionActions::Sessions(view.actions));
    }

    fn show_thread_view(&mut self, view: ThreadSelectionView) {
        self.interaction_pane.show_selection_view(view.model);
        self.selection_actions
            .push(SelectionActions::Threads(view.actions));
    }

    fn close_selection_view(&mut self) {
        self.interaction_pane.pop_selection_view();
        self.selection_actions.pop();
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

    pub(crate) fn selection_pane(&self) -> Option<&PaneView<SelectionViewState>> {
        self.interaction_pane.selection_pane()
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

    pub(crate) fn latest_agent_response(&self) -> Option<&str> {
        crate::components::transcript::latest_agent_response(self.messages())
    }

    pub(crate) fn transcript_markdown(&self) -> String {
        crate::components::transcript::export_markdown(self.messages())
    }

    pub(crate) fn transcript_scroll(&self) -> &TranscriptScroll {
        &self.transcript_scroll
    }

    pub(crate) fn status(&self) -> &Status {
        &self.status
    }

    pub(crate) fn approval_mode(&self) -> ApprovalMode {
        self.approval_mode
    }

    pub(crate) fn status_line(&self) -> &StatusLineModel {
        &self.status_line
    }

    pub(crate) fn accepts_input(&self) -> bool {
        matches!(
            &self.status,
            Status::Ready | Status::Working | Status::Error
        ) || matches!(
            self.selection_actions.last(),
            Some(SelectionActions::Interaction(_))
        )
    }

    pub(crate) fn update(&mut self, event: AppEvent) {
        match event {
            AppEvent::ClipboardImageRead(Ok(bytes)) => self.attach_image_bytes(bytes),
            AppEvent::ClipboardImageRead(Err(error)) => self.record_clipboard_error(error),
            AppEvent::ConfigSnapshotReceived(config) => self.status_line.apply_config(&config),
            AppEvent::CommandStarted(command) => {
                self.thread
                    .update(ThreadPresentationEvent::CommandStarted(command));
            }
            AppEvent::CommandCompleted { command, result } => {
                self.thread
                    .update(ThreadPresentationEvent::CommandCompleted { command, result });
                self.status = Status::Ready;
            }
            AppEvent::FailureReported(error) => {
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(error));
                self.status = Status::Error;
            }
            AppEvent::FileSearchSnapshotReceived(snapshot) => {
                self.interaction_pane.apply_file_search_snapshot(snapshot);
            }
            AppEvent::FileViewOpened(view) => self.show_file_view(view),
            AppEvent::GitStatusReceived(status) => self.status_line.apply_git_status(&status),
            AppEvent::HostOperationCompleted(Ok(notice)) => {
                self.thread
                    .update(ThreadPresentationEvent::NoticeReceived(notice));
            }
            AppEvent::HostOperationCompleted(Err(error)) => {
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(error));
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
            AppEvent::InteractionViewOpened(view) => self.show_interaction_view(view),
            AppEvent::McpViewOpened(view) => self.show_mcp_view(view),
            AppEvent::McpViewReplaced(view) => self.replace_mcp_view(view),
            AppEvent::ModelViewOpened(view) => self.show_model_view(view),
            AppEvent::RewindViewOpened(view) => self.show_rewind_view(view),
            AppEvent::SessionViewOpened(view) => self.show_session_view(view),
            AppEvent::ThreadViewOpened(view) => self.show_thread_view(view),
            AppEvent::SelectionViewClosed => self.close_selection_view(),
            AppEvent::SelectionViewOpened(model) => self.show_selection_view(model),
            AppEvent::SkillsViewOpened(view) => self.show_skills_view(view),
            AppEvent::SkillsViewReplaced(view) => self.replace_skills_view(view),
            AppEvent::ThemeViewClosed => self.close_theme_views(),
            AppEvent::ThemeViewOpened(view) => self.show_theme_view(view),
            AppEvent::ThreadSnapshotReceived(thread) => self
                .thread
                .update(ThreadPresentationEvent::SnapshotReceived(thread)),
            AppEvent::ThreadHistoryPageReceived(thread) => self
                .thread
                .update(ThreadPresentationEvent::HistoryPageReceived(thread)),
            AppEvent::TransientThreadStreamReset => {
                self.thread
                    .update(ThreadPresentationEvent::TransientStreamReset);
            }
            AppEvent::TransientThreadUpdateReceived(update) => self
                .thread
                .update(ThreadPresentationEvent::TransientUpdateReceived(update)),
            AppEvent::TranscriptCleared => {
                self.thread.update(ThreadPresentationEvent::Cleared);
                self.transcript_scroll.follow_latest();
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

    fn handle_global_key(&mut self, key: KeyEvent, now: Instant) -> Option<AppCommand> {
        if self.selection_view().is_none()
            && self.accepts_input()
            && key.kind == KeyEventKind::Press
            && key.code == KeyCode::BackTab
        {
            self.approval_mode = match self.approval_mode {
                ApprovalMode::AskPermissions => ApprovalMode::AutoReview,
                ApprovalMode::AutoReview => ApprovalMode::BypassPermissions,
                ApprovalMode::BypassPermissions => ApprovalMode::AskPermissions,
            };
            return None;
        }
        if self.selection_view().is_none() && self.transcript_scroll.handle_key(key) {
            return (key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Home)
                .then_some(AppCommand::LoadOlderHistory);
        }
        if key.code == KeyCode::Esc
            && key.modifiers.is_empty()
            && key.kind == KeyEventKind::Press
            && self.accepts_input()
        {
            if self
                .last_root_escape
                .take()
                .is_some_and(|previous| now.duration_since(previous) <= DOUBLE_ESCAPE_WINDOW)
            {
                return Some(AppCommand::OpenRewindPane);
            }
            self.last_root_escape = Some(now);
            return None;
        }
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
                KeyCode::Char('o') => Some(AppCommand::CopyLastResponse),
                KeyCode::Char('z') => Some(AppCommand::Suspend),
                _ => None,
            };
        }
        None
    }

    fn handle_slash_command(&mut self, invocation: SlashCommandInvocation) -> Option<AppCommand> {
        let local = invocation
            .command
            .name
            .parse::<TuiSlashCommandAction>()
            .ok();
        if matches!(local, Some(TuiSlashCommandAction::Export))
            && invocation
                .arguments
                .iter()
                .any(|argument| matches!(argument, ComposerInput::Image { .. }))
        {
            self.thread.update(ThreadPresentationEvent::FailureReported(
                "/export accepts a relative text path, not image arguments".into(),
            ));
            return None;
        }
        if matches!(self.status, Status::Working)
            && invocation.origin == SlashCommandOrigin::Local
            && !matches!(
                local,
                Some(TuiSlashCommandAction::Copy | TuiSlashCommandAction::Export)
            )
        {
            self.thread
                .update(ThreadPresentationEvent::FailureReported(format!(
                    "/{} is unavailable while a turn is running; submit a follow-up prompt or wait for the turn to finish",
                    invocation.command.name
                )));
            return None;
        }
        match (invocation.origin, local) {
            (
                SlashCommandOrigin::Local,
                Some(TuiSlashCommandAction::Quit | TuiSlashCommandAction::Exit),
            ) if invocation.arguments.is_empty() => Some(AppCommand::Quit),
            (SlashCommandOrigin::Local, Some(TuiSlashCommandAction::Copy))
                if invocation.arguments.is_empty() =>
            {
                Some(AppCommand::CopyLastResponse)
            }
            (SlashCommandOrigin::Local, Some(TuiSlashCommandAction::Export)) => {
                let requested_path = (!invocation.display_arguments.trim().is_empty())
                    .then(|| PathBuf::from(invocation.display_arguments.trim()));
                Some(AppCommand::ExportTranscript { requested_path })
            }
            (SlashCommandOrigin::Server, _) => {
                let submission = invocation.into_forwarded_submission();
                self.thread.update(ThreadPresentationEvent::UserSubmitted(
                    submission.display_text.clone(),
                ));
                self.status = Status::Working;
                Some(AppCommand::SubmitTurn {
                    submission,
                    approval_mode: self.approval_mode,
                })
            }
            (SlashCommandOrigin::Skill, _) => None,
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
