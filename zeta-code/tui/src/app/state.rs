use super::command::AppCommand;
use super::escape::RootEscapeOutcome;
use super::escape::RootEscapeSequence;
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
use crate::components::welcome::WelcomeModel;
use crate::features::additional_directories::AdditionalDirectorySelectionAction;
use crate::features::additional_directories::AdditionalDirectorySelectionView;
use crate::features::config::ConfigSelectionAction;
use crate::features::config::TerminalSettings;
use crate::features::connectors::ConnectorSelectionAction;
use crate::features::connectors::ConnectorSelectionView;
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
use crate::features::shortcuts::ShortcutAction;
use crate::features::shortcuts::ShortcutCaptureOutcome;
use crate::features::shortcuts::ShortcutCaptureState;
use crate::features::shortcuts::ShortcutEdit;
use crate::features::shortcuts::ShortcutEditKind;
use crate::features::shortcuts::ShortcutView;
use crate::features::shortcuts::action_menu;
use crate::features::shortcuts::capture_view;
use crate::features::skills::{SkillSelectionAction, SkillSelectionView};
use crate::features::status_line::ApprovalModeStatus;
use crate::features::status_line::StatusLineModel;
use crate::features::status_line::StatusLineSelectionAction;
use crate::features::status_line::StatusLineSelectionView;
use crate::features::theme::ThemeSelectionAction;
use crate::features::theme::ThemeSelectionView;
use crate::features::thread::ThreadFeatureState;
use crate::features::thread::ThreadPresentationEvent;
use crate::features::thread::TurnActivity;
use crate::keymap::AppChordMatch;
use crate::keymap::AppKeymap;
use crate::keymap::AppKeymapAction;
use crate::keymap::AppKeymapContext;
use crate::mouse::MouseMode;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use zeta_protocol::ApprovalMode;

#[path = "config_state.rs"]
mod config_state;

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
    pub(super) app_keymap: AppKeymap,
    thread: ThreadFeatureState,
    transcript_scroll: TranscriptScroll,
    welcome: WelcomeModel,
    selection_actions: Vec<SelectionActions>,
    root_escape_sequence: RootEscapeSequence,
    status: Status,
    status_line: StatusLineModel,
    terminal_settings: TerminalSettings,
    approval_mode_status: ApprovalModeStatus,
}

#[derive(Debug)]
enum SelectionActions {
    ReadOnly,
    AdditionalDirectories(BTreeMap<SelectionItemId, AdditionalDirectorySelectionAction>),
    Config(BTreeMap<SelectionItemId, ConfigSelectionAction>),
    Interaction(InteractionSelectionState),
    Connectors(BTreeMap<SelectionItemId, ConnectorSelectionAction>),
    Mcp(BTreeMap<SelectionItemId, McpSelectionAction>),
    Model(BTreeMap<SelectionItemId, ModelSelectionAction>),
    Rewind(BTreeMap<SelectionItemId, RewindSelectionAction>),
    Sessions(BTreeMap<SelectionItemId, SessionSelectionAction>),
    Skills(BTreeMap<SelectionItemId, SkillSelectionAction>),
    StatusLine(BTreeMap<SelectionItemId, StatusLineSelectionAction>),
    Theme(BTreeMap<SelectionItemId, ThemeSelectionAction>),
    Shortcuts(BTreeMap<SelectionItemId, ShortcutAction>),
    ShortcutCapture(ShortcutCaptureState),
}

impl App {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            interaction_pane: InteractionPane::new(),
            app_keymap: AppKeymap::default(),
            thread: ThreadFeatureState::default(),
            transcript_scroll: TranscriptScroll::default(),
            welcome: WelcomeModel::for_workspace(Path::new(".")),
            selection_actions: Vec::new(),
            root_escape_sequence: RootEscapeSequence::default(),
            status: Status::Ready,
            status_line: StatusLineModel::new(),
            terminal_settings: TerminalSettings::default(),
            approval_mode_status: ApprovalModeStatus::default(),
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
            app_keymap: AppKeymap::default(),
            thread: ThreadFeatureState::default(),
            transcript_scroll: TranscriptScroll::default(),
            welcome: WelcomeModel::for_workspace(workspace_root),
            selection_actions: Vec::new(),
            root_escape_sequence: RootEscapeSequence::default(),
            status: Status::Ready,
            status_line: StatusLineModel::new(),
            terminal_settings: TerminalSettings::default(),
            approval_mode_status: ApprovalModeStatus::default(),
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Option<AppCommand> {
        self.handle_key_at(key, Instant::now())
    }

    fn handle_key_at(&mut self, key: KeyEvent, now: Instant) -> Option<AppCommand> {
        if matches!(
            self.selection_actions.last(),
            Some(SelectionActions::ShortcutCapture(_))
        ) {
            let outcome = match self.selection_actions.last_mut() {
                Some(SelectionActions::ShortcutCapture(capture)) => capture.handle_key(key),
                _ => unreachable!("the shortcut capture state was checked above"),
            };
            return match outcome {
                ShortcutCaptureOutcome::Pending(model) => {
                    self.interaction_pane.replace_selection_view(model);
                    None
                }
                ShortcutCaptureOutcome::Cancelled => {
                    self.close_selection_view();
                    None
                }
                ShortcutCaptureOutcome::Edit(edit) => Some(AppCommand::EditShortcut(edit)),
            };
        }
        let temporary_interaction_active = self.selection_view().is_some()
            || self.slash_popup().is_some()
            || self.mention_popup().is_some();
        let is_root_escape_press = key.kind == KeyEventKind::Press
            && key.code == KeyCode::Esc
            && key.modifiers.is_empty()
            && !temporary_interaction_active;
        if key.kind == KeyEventKind::Press && !is_root_escape_press {
            self.root_escape_sequence.reset();
        }
        let keymap_context = self.app_keymap_context(key.kind == KeyEventKind::Press);
        match self.app_keymap.route_chord(&key, keymap_context, now) {
            AppChordMatch::PassThrough => {}
            AppChordMatch::Pending | AppChordMatch::Consumed => return None,
            AppChordMatch::Command(action) => {
                return self.apply_app_keymap_action(action, now);
            }
        }
        if !self.accepts_input() {
            self.root_escape_sequence.reset();
            return self.handle_app_key(key, now);
        }

        let outcome = self.interaction_pane.handle_key(key);
        if matches!(outcome, InteractionPaneOutcome::Unhandled) {
            return self.handle_app_key(key, now);
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
                Some(AppCommand::SubmitTurn { submission })
            }
            InteractionPaneOutcome::Consumed => None,
            InteractionPaneOutcome::Unhandled => None,
            InteractionPaneOutcome::ViewDismissed => {
                self.selection_actions.pop();
                self.root_escape_sequence.reset();
                None
            }
        }
    }

    fn activate_selection_item(&mut self, item_id: &SelectionItemId) -> Option<AppCommand> {
        if let Some(SelectionActions::Interaction(state)) = self.selection_actions.last_mut() {
            let outcome = state.activate_item(item_id)?;
            return self.apply_interaction_selection_outcome(outcome);
        }
        if let Some(SelectionActions::Shortcuts(actions)) = self.selection_actions.last() {
            let action = actions.get(item_id)?.clone();
            return self.apply_shortcut_action(action);
        }
        match self.selection_actions.last()? {
            SelectionActions::ReadOnly => None,
            SelectionActions::AdditionalDirectories(actions) => match actions.get(item_id)? {
                AdditionalDirectorySelectionAction::Remove { root } => {
                    Some(AppCommand::RemoveAdditionalDirectory { root: root.clone() })
                }
            },
            SelectionActions::Config(actions) => match actions.get(item_id)?.clone() {
                ConfigSelectionAction::SetTerminalSettings(edit) => {
                    Some(AppCommand::EditConfig(edit))
                }
                ConfigSelectionAction::SetAdditionalDirectoryPermissions(edit) => {
                    Some(AppCommand::EditAdditionalDirectoryPermissions(edit))
                }
                ConfigSelectionAction::OpenProviderApiKey {
                    provider,
                    display_name,
                } => {
                    self.show_config_view(crate::features::config::provider_api_key_view(
                        provider,
                        display_name,
                    ));
                    None
                }
                ConfigSelectionAction::SetProviderApiKey { .. } => None,
            },
            SelectionActions::Interaction(_) => None,
            SelectionActions::Connectors(actions) => match actions.get(item_id)? {
                ConnectorSelectionAction::ConnectDeviceOAuth {
                    connector_id,
                    connection_generation,
                } => Some(AppCommand::ConnectConnectorDeviceOAuth {
                    connector_id: connector_id.clone(),
                    connection_generation: *connection_generation,
                }),
                ConnectorSelectionAction::Disconnect { connector_id } => {
                    Some(AppCommand::DisconnectConnector {
                        connector_id: connector_id.clone(),
                    })
                }
            },
            SelectionActions::Mcp(actions) => match actions.get(item_id)? {
                McpSelectionAction::SetEnablement {
                    server_id,
                    enablement,
                } => Some(AppCommand::SetMcpEnablement {
                    server_id: server_id.clone(),
                    enablement: *enablement,
                }),
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
            SelectionActions::Skills(actions) => match actions.get(item_id)?.clone() {
                SkillSelectionAction::SetEnablement {
                    skill_id,
                    enablement,
                } => Some(AppCommand::SetSkillEnablement {
                    skill_id,
                    enablement,
                }),
            },
            SelectionActions::StatusLine(actions) => match actions.get(item_id)? {
                StatusLineSelectionAction::SetEnabled(edit) => {
                    Some(AppCommand::EditStatusLine(edit.clone()))
                }
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
            SelectionActions::Shortcuts(_) | SelectionActions::ShortcutCapture(_) => None,
        }
    }

    fn apply_shortcut_action(&mut self, action: ShortcutAction) -> Option<AppCommand> {
        match action {
            ShortcutAction::OpenAction { action, revision } => {
                self.show_shortcut_view(action_menu(action, revision));
                None
            }
            ShortcutAction::BeginCapture {
                action,
                revision,
                intent,
                mode,
            } => {
                let (model, capture) = capture_view(action, revision, intent, mode);
                self.push_selection_view(model, SelectionActions::ShortcutCapture(capture));
                None
            }
            ShortcutAction::ClearUser {
                command_id,
                revision,
            } => Some(AppCommand::EditShortcut(ShortcutEdit {
                expected_revision: revision,
                command_id,
                kind: ShortcutEditKind::ClearUser,
            })),
        }
    }

    fn activate_selection_free_form(
        &mut self,
        item_id: &SelectionItemId,
        value: String,
    ) -> Option<AppCommand> {
        if let Some(SelectionActions::Config(actions)) = self.selection_actions.last() {
            let ConfigSelectionAction::SetProviderApiKey { provider } = actions.get(item_id)?
            else {
                return None;
            };
            return Some(AppCommand::SetProviderApiKey(
                crate::features::config::ProviderApiKeyEdit::new(provider.clone(), value),
            ));
        }
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

    pub(crate) fn select_slash_command(&mut self, index: usize) -> bool {
        self.accepts_input() && self.interaction_pane.select_slash_command(index)
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

    pub(crate) fn mouse_mode(&self) -> MouseMode {
        if self.terminal_settings.mouse_interactions() {
            self.interaction_pane.mouse_mode()
        } else {
            MouseMode::TerminalSelection
        }
    }

    fn show_selection_view(&mut self, model: PaneViewModel<SelectionViewModel>) {
        self.push_selection_view(model, SelectionActions::ReadOnly);
    }

    fn show_additional_directories_view(&mut self, view: AdditionalDirectorySelectionView) {
        self.push_selection_view(
            view.model,
            SelectionActions::AdditionalDirectories(view.actions),
        );
    }

    fn replace_additional_directories_view(&mut self, view: AdditionalDirectorySelectionView) {
        self.replace_selection_view(
            view.model,
            SelectionActions::AdditionalDirectories(view.actions),
        );
    }

    fn show_interaction_view(&mut self, view: InteractionSelectionView) {
        self.push_selection_view(view.model, SelectionActions::Interaction(view.state));
    }

    fn show_skills_view(&mut self, view: SkillSelectionView) {
        self.push_selection_view(view.model, SelectionActions::Skills(view.actions));
    }

    fn show_mcp_view(&mut self, view: McpSelectionView) {
        self.push_selection_view(view.model, SelectionActions::Mcp(view.actions));
    }

    fn show_connector_view(&mut self, view: ConnectorSelectionView) {
        self.push_selection_view(view.model, SelectionActions::Connectors(view.actions));
    }

    fn replace_connector_view(&mut self, view: ConnectorSelectionView) {
        self.replace_selection_view(view.model, SelectionActions::Connectors(view.actions));
    }

    pub(crate) fn connector_view_open(&self) -> bool {
        matches!(
            self.selection_actions.last(),
            Some(SelectionActions::Connectors(_))
        )
    }

    fn replace_mcp_view(&mut self, view: McpSelectionView) {
        self.replace_selection_view(view.model, SelectionActions::Mcp(view.actions));
    }

    fn show_model_view(&mut self, view: ModelSelectionView) {
        self.push_selection_view(view.model, SelectionActions::Model(view.actions));
    }

    fn show_rewind_view(&mut self, view: RewindSelectionView) {
        self.push_selection_view(view.model, SelectionActions::Rewind(view.actions));
    }

    fn show_session_view(&mut self, view: SessionSelectionView) {
        self.push_selection_view(view.model, SelectionActions::Sessions(view.actions));
    }

    fn push_selection_view(
        &mut self,
        model: PaneViewModel<SelectionViewModel>,
        actions: SelectionActions,
    ) {
        self.root_escape_sequence.reset();
        self.interaction_pane.show_selection_view(model);
        self.selection_actions.push(actions);
    }

    fn replace_selection_view(
        &mut self,
        model: PaneViewModel<SelectionViewModel>,
        actions: SelectionActions,
    ) {
        self.root_escape_sequence.reset();
        self.interaction_pane.replace_selection_view(model);
        match self.selection_actions.last_mut() {
            Some(current) => *current = actions,
            None => self.selection_actions.push(actions),
        }
    }

    fn close_selection_view(&mut self) {
        self.root_escape_sequence.reset();
        self.interaction_pane.pop_selection_view();
        self.selection_actions.pop();
    }

    fn replace_skills_view(&mut self, view: SkillSelectionView) {
        self.replace_selection_view(view.model, SelectionActions::Skills(view.actions));
    }

    fn show_theme_view(&mut self, view: ThemeSelectionView) {
        self.push_selection_view(view.model, SelectionActions::Theme(view.actions));
    }

    fn show_shortcut_view(&mut self, view: ShortcutView) {
        self.push_selection_view(view.model, SelectionActions::Shortcuts(view.actions));
    }

    fn show_status_line_view(&mut self, view: StatusLineSelectionView) {
        self.push_selection_view(view.model, SelectionActions::StatusLine(view.actions));
    }

    fn replace_status_line_view(&mut self, view: StatusLineSelectionView) {
        self.replace_selection_view(view.model, SelectionActions::StatusLine(view.actions));
    }

    fn close_shortcut_views(&mut self) {
        self.root_escape_sequence.reset();
        while matches!(
            self.selection_actions.last(),
            Some(SelectionActions::Shortcuts(_) | SelectionActions::ShortcutCapture(_))
        ) {
            self.interaction_pane.pop_selection_view();
            self.selection_actions.pop();
        }
    }

    fn close_theme_views(&mut self) {
        self.root_escape_sequence.reset();
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

    pub(crate) fn select_mention(&mut self, index: usize) -> bool {
        self.accepts_input() && self.interaction_pane.select_mention(index)
    }

    pub(crate) fn select_visible_item(&mut self, index: usize) -> bool {
        self.interaction_pane.select_visible_item(index)
    }

    pub(crate) fn select_tab(&mut self, index: usize) -> bool {
        self.interaction_pane.select_tab(index)
    }

    pub(crate) fn activate_visible_item(&mut self, index: usize) -> Option<AppCommand> {
        let outcome = self.interaction_pane.activate_visible_item(index)?;
        self.handle_interaction_pane_outcome(outcome)
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

    pub(crate) fn welcome(&self) -> &WelcomeModel {
        &self.welcome
    }

    pub(crate) fn status(&self) -> &Status {
        &self.status
    }

    pub(crate) fn approval_mode_status(&self) -> ApprovalModeStatus {
        self.approval_mode_status
    }

    #[cfg(test)]
    pub(crate) fn approval_mode(&self) -> ApprovalMode {
        self.approval_mode_status.next
    }

    pub(crate) fn set_next_approval_mode(&mut self, approval_mode: ApprovalMode) {
        self.approval_mode_status.next = approval_mode;
    }

    pub(crate) fn set_current_approval_mode(&mut self, approval_mode: Option<ApprovalMode>) {
        self.approval_mode_status.current = approval_mode;
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
            AppEvent::AdditionalDirectoriesViewOpened(view) => {
                self.show_additional_directories_view(view)
            }
            AppEvent::AdditionalDirectoryRemoved { root, view } => {
                self.replace_additional_directories_view(view);
                self.thread
                    .update(ThreadPresentationEvent::NoticeReceived(format!(
                        "Removed additional directory {}",
                        root.display()
                    )));
                self.status = Status::Ready;
            }
            AppEvent::ClipboardImageRead(Ok(bytes)) => self.attach_image_bytes(bytes),
            AppEvent::ClipboardImageRead(Err(error)) => self.record_clipboard_error(error),
            AppEvent::ConfigSettingsReceived(settings) => self.terminal_settings = settings,
            AppEvent::ConfigViewOpened(view) => self.show_config_view(view),
            AppEvent::ConfigViewReplaced(view) => self.replace_config_view(view),
            AppEvent::ConfigApiKeySaved { provider, view } => {
                self.close_selection_view();
                self.replace_config_view(view);
                self.thread
                    .update(ThreadPresentationEvent::NoticeReceived(format!(
                        "Saved API key for {provider}"
                    )));
                self.status = Status::Ready;
            }
            AppEvent::PreferredModelReceived(model) => {
                self.status_line.apply_preferred_model(model.as_ref())
            }
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
            AppEvent::ShortcutViewOpened(view) => self.show_shortcut_view(view),
            AppEvent::ShortcutViewsClosed => self.close_shortcut_views(),
            AppEvent::StatusLineSettingsReceived(settings) => {
                self.status_line.apply_settings(settings)
            }
            AppEvent::StatusLineViewOpened(view) => self.show_status_line_view(view),
            AppEvent::StatusLineViewReplaced(view) => self.replace_status_line_view(view),
            AppEvent::ConnectorViewOpened(view) => self.show_connector_view(view),
            AppEvent::ConnectorViewReplaced(view) => self.replace_connector_view(view),
            AppEvent::McpViewOpened(view) => self.show_mcp_view(view),
            AppEvent::McpViewReplaced(view) => self.replace_mcp_view(view),
            AppEvent::ModelViewOpened(view) => self.show_model_view(view),
            AppEvent::RewindViewOpened(view) => self.show_rewind_view(view),
            AppEvent::SessionViewOpened(view) => self.show_session_view(view),
            AppEvent::SelectionViewClosed => self.close_selection_view(),
            AppEvent::SelectionViewOpened(model) => self.show_selection_view(model),
            AppEvent::SkillsViewOpened(view) => self.show_skills_view(view),
            AppEvent::SkillsViewReplaced(view) => self.replace_skills_view(view),
            AppEvent::ThemeViewClosed => self.close_theme_views(),
            AppEvent::ThemeViewOpened(view) => self.show_theme_view(view),
            AppEvent::ThreadTranscriptSnapshotReceived(transcript) => {
                self.thread
                    .update(ThreadPresentationEvent::TranscriptSnapshotReceived(
                        transcript,
                    ))
            }
            AppEvent::ThreadTranscriptHistoryPageReceived(transcript) => {
                self.thread
                    .update(ThreadPresentationEvent::TranscriptHistoryPageReceived(
                        transcript,
                    ))
            }
            AppEvent::ThreadTranscriptUpdateReceived(update) => self
                .thread
                .update(ThreadPresentationEvent::TranscriptUpdateReceived(update)),
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

    fn app_keymap_context(&self, is_press: bool) -> AppKeymapContext {
        AppKeymapContext {
            accepts_input: self.accepts_input(),
            has_selection: self.selection_view().is_some(),
            composer_empty: self.input().is_empty(),
            is_press,
        }
    }

    fn handle_app_key(&mut self, key: KeyEvent, now: Instant) -> Option<AppCommand> {
        let keymap_context = self.app_keymap_context(key.kind == KeyEventKind::Press);
        if let Some(action) = self.app_keymap.resolve_single(&key, keymap_context) {
            return self.apply_app_keymap_action(action, now);
        }
        if self.selection_view().is_none() && self.transcript_scroll.handle_key(key) {
            return (key.code == KeyCode::Home).then_some(AppCommand::LoadOlderHistory);
        }
        None
    }

    fn apply_app_keymap_action(
        &mut self,
        action: AppKeymapAction,
        now: Instant,
    ) -> Option<AppCommand> {
        match action {
            AppKeymapAction::CycleApprovalMode => Some(AppCommand::CycleNextApprovalMode),
            AppKeymapAction::RootEscape => match self.root_escape_sequence.press(now) {
                RootEscapeOutcome::WaitingForSecondPress => None,
                RootEscapeOutcome::OpenRewind => Some(AppCommand::OpenRewindPane),
            },
            AppKeymapAction::OpenRewind => Some(AppCommand::OpenRewindPane),
            AppKeymapAction::ReadClipboardImage => Some(AppCommand::ReadClipboardImage),
            AppKeymapAction::InterruptOrQuit => self.quit_or_interrupt(),
            AppKeymapAction::CopyLastResponse => Some(AppCommand::CopyLastResponse),
            AppKeymapAction::Suspend => Some(AppCommand::Suspend),
        }
    }

    pub(crate) fn handle_tick(&mut self, now: Instant) {
        let context = self.app_keymap_context(true);
        self.app_keymap.expire(context, now);
    }

    pub(crate) fn pending_key_chord_label(&self) -> Option<String> {
        self.app_keymap.pending_chord_label()
    }

    pub(crate) fn report_keybinding_diagnostic(&mut self, diagnostic: impl Into<String>) {
        self.thread
            .update(ThreadPresentationEvent::FailureReported(format!(
                "Keybindings: {}",
                diagnostic.into()
            )));
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
            (SlashCommandOrigin::Local, Some(TuiSlashCommandAction::Quit))
                if invocation.arguments.is_empty() =>
            {
                Some(AppCommand::Quit)
            }
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
            (SlashCommandOrigin::Local, Some(TuiSlashCommandAction::Shortcuts))
                if invocation.arguments.is_empty() =>
            {
                Some(AppCommand::OpenShortcutsPane)
            }
            (SlashCommandOrigin::Local, Some(TuiSlashCommandAction::Config))
                if invocation.arguments.is_empty() =>
            {
                Some(AppCommand::OpenConfigPane)
            }
            (SlashCommandOrigin::Local, Some(TuiSlashCommandAction::StatusLine))
                if invocation.arguments.is_empty() =>
            {
                Some(AppCommand::OpenStatusLinePane)
            }
            (SlashCommandOrigin::Server, _) => {
                let submission = invocation.into_forwarded_submission();
                self.thread.update(ThreadPresentationEvent::UserSubmitted(
                    submission.display_text.clone(),
                ));
                self.status = Status::Working;
                Some(AppCommand::SubmitTurn { submission })
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
