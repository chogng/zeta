use super::command::AppCommand;
use super::escape::RootEscapeOutcome;
use super::escape::RootEscapeSequence;
use super::event::AppEvent;
use crate::components::chat_history::ChatHistoryScroll;
use crate::components::chat_history::Message;
use crate::components::chat_input::ChatInputItem;
use crate::components::chat_input::MentionPluginItem;
use crate::components::chat_input::SkillSelectorItem;
use crate::components::chat_input_area::ChatInputArea;
use crate::components::chat_input_area::ChatInputAreaHeightEntryView;
use crate::components::chat_input_area::ChatInputAreaInteractionId;
use crate::components::chat_input_area::ChatInputAreaOutcome;
use crate::components::chat_input_area::ChatInputAreaOverlayView;
use crate::components::list_selection::ListSelectionItemId;
use crate::components::list_selection::ListSelectionModel;
use crate::components::list_selection::ListSelectionState;
use crate::components::pane::PaneId;
use crate::components::pane::PaneSpec;
use crate::components::pane::PaneView;
use crate::components::welcome::WelcomeModel;
use crate::features::additional_directories::AdditionalDirectoryPaneSpec;
use crate::features::additional_directories::AdditionalDirectorySelectionAction;
use crate::features::config::ConfigSelectionAction;
use crate::features::config::TerminalSettings;
use crate::features::connectors::ConnectorPaneSpec;
use crate::features::connectors::ConnectorSelectionAction;
use crate::features::interactions::InteractionBinding;
use crate::features::interactions::InteractionRequest;
use crate::features::keymap::KeymapAction;
use crate::features::keymap::KeymapCaptureOutcome;
use crate::features::keymap::KeymapCaptureState;
use crate::features::keymap::KeymapEdit;
use crate::features::keymap::KeymapEditKind;
use crate::features::keymap::KeymapPaneSpec;
use crate::features::keymap::keymap_action_menu;
use crate::features::keymap::keymap_capture_pane_spec;
use crate::features::mcp::McpPaneSpec;
use crate::features::mcp::McpSelectionAction;
use crate::features::models::ModelPaneSpec;
use crate::features::models::ModelSelectionAction;
use crate::features::rewind::RewindPaneSpec;
use crate::features::rewind::RewindSelectionAction;
use crate::features::sessions::SessionPaneSpec;
use crate::features::sessions::SessionSelectionAction;
use crate::features::skills::{SkillPaneSpec, SkillSelectionAction};
use crate::features::status_line::ApprovalModeStatus;
use crate::features::status_line::StatusLineModel;
use crate::features::status_line::StatusLinePaneSpec;
use crate::features::status_line::StatusLineSelectionAction;
use crate::features::theme::ThemePaneSpec;
use crate::features::theme::ThemeSelectionAction;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TurnInputMode {
    StartTurn,
    SteerTurn,
}

#[derive(Debug)]
pub(crate) struct App {
    chat_input_area: ChatInputArea,
    pub(super) app_keymap: AppKeymap,
    thread: ThreadFeatureState,
    transcript_scroll: ChatHistoryScroll,
    welcome: WelcomeModel,
    pane_actions: BTreeMap<PaneId, PaneActions>,
    interaction_bindings: BTreeMap<ChatInputAreaInteractionId, InteractionBinding>,
    root_escape_sequence: RootEscapeSequence,
    status: Status,
    turn_input_mode: TurnInputMode,
    status_line: StatusLineModel,
    terminal_settings: TerminalSettings,
    approval_mode_status: ApprovalModeStatus,
}

#[derive(Debug)]
enum PaneActions {
    ReadOnly,
    AdditionalDirectories(BTreeMap<ListSelectionItemId, AdditionalDirectorySelectionAction>),
    Config(BTreeMap<ListSelectionItemId, ConfigSelectionAction>),
    ConfigTextPrompt { provider: String },
    Connectors(BTreeMap<ListSelectionItemId, ConnectorSelectionAction>),
    Mcp(BTreeMap<ListSelectionItemId, McpSelectionAction>),
    Model(BTreeMap<ListSelectionItemId, ModelSelectionAction>),
    Rewind(BTreeMap<ListSelectionItemId, RewindSelectionAction>),
    Sessions(BTreeMap<ListSelectionItemId, SessionSelectionAction>),
    Skills(BTreeMap<ListSelectionItemId, SkillSelectionAction>),
    StatusLine(BTreeMap<ListSelectionItemId, StatusLineSelectionAction>),
    Theme(BTreeMap<ListSelectionItemId, ThemeSelectionAction>),
    Keymap(BTreeMap<ListSelectionItemId, KeymapAction>),
    KeymapCapture(KeymapCaptureState),
}

impl App {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            chat_input_area: ChatInputArea::new(),
            app_keymap: AppKeymap::default(),
            thread: ThreadFeatureState::default(),
            transcript_scroll: ChatHistoryScroll::default(),
            welcome: WelcomeModel::for_workspace(Path::new(".")),
            pane_actions: BTreeMap::new(),
            interaction_bindings: BTreeMap::new(),
            root_escape_sequence: RootEscapeSequence::default(),
            status: Status::Ready,
            turn_input_mode: TurnInputMode::StartTurn,
            status_line: StatusLineModel::new(),
            terminal_settings: TerminalSettings::default(),
            approval_mode_status: ApprovalModeStatus::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_workspace(workspace_root: &Path) -> Self {
        Self::for_workspace_with_slash_commands(
            workspace_root,
            crate::components::chat_input::default_slash_command_catalog(),
        )
    }

    pub(crate) fn for_workspace_with_slash_commands(
        workspace_root: &Path,
        slash_commands: SlashCommandCatalog,
    ) -> Self {
        Self {
            chat_input_area: ChatInputArea::with_slash_commands(slash_commands),
            app_keymap: AppKeymap::default(),
            thread: ThreadFeatureState::default(),
            transcript_scroll: ChatHistoryScroll::default(),
            welcome: WelcomeModel::for_workspace(workspace_root),
            pane_actions: BTreeMap::new(),
            interaction_bindings: BTreeMap::new(),
            root_escape_sequence: RootEscapeSequence::default(),
            status: Status::Ready,
            turn_input_mode: TurnInputMode::StartTurn,
            status_line: StatusLineModel::new(),
            terminal_settings: TerminalSettings::default(),
            approval_mode_status: ApprovalModeStatus::default(),
        }
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Option<AppCommand> {
        self.handle_key_at(key, Instant::now())
    }

    fn top_pane_actions(&self) -> Option<&PaneActions> {
        let pane_id = self.chat_input_area.top_pane_id()?;
        self.pane_actions.get(&pane_id)
    }

    fn top_pane_actions_mut(&mut self) -> Option<&mut PaneActions> {
        let pane_id = self.chat_input_area.top_pane_id()?;
        self.pane_actions.get_mut(&pane_id)
    }

    fn handle_key_at(&mut self, key: KeyEvent, now: Instant) -> Option<AppCommand> {
        if matches!(self.top_pane_actions(), Some(PaneActions::KeymapCapture(_))) {
            let outcome = match self.top_pane_actions_mut() {
                Some(PaneActions::KeymapCapture(capture)) => capture.handle_key(key),
                _ => unreachable!("the keymap capture state was checked above"),
            };
            return match outcome {
                KeymapCaptureOutcome::Pending(model) => {
                    self.chat_input_area.update_top_key_capture(model);
                    None
                }
                KeymapCaptureOutcome::Cancelled => {
                    self.close_list_selection_pane();
                    None
                }
                KeymapCaptureOutcome::Edit(edit) => Some(AppCommand::EditKeymap(edit)),
            };
        }
        let temporary_interaction_active =
            self.chat_input_area.pane_active() || self.suggest().is_some();
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

        let outcome = if self.turn_input_mode == TurnInputMode::SteerTurn {
            self.chat_input_area.handle_active_turn_key(key)
        } else {
            self.chat_input_area.handle_key(key)
        };
        if matches!(outcome, ChatInputAreaOutcome::Unhandled) {
            return self.handle_app_key(key, now);
        }
        self.handle_chat_input_area_outcome(outcome)
    }

    fn handle_chat_input_area_outcome(
        &mut self,
        outcome: ChatInputAreaOutcome,
    ) -> Option<AppCommand> {
        match outcome {
            ChatInputAreaOutcome::ActivateSelectionItem { pane_id, item_id } => {
                self.activate_selection_item(pane_id, &item_id)
            }
            ChatInputAreaOutcome::Command(command) => self.handle_slash_command(command),
            ChatInputAreaOutcome::ApprovalResponse {
                interaction_id,
                decision,
            } => {
                let response = self
                    .interaction_bindings
                    .get(&interaction_id)?
                    .approval_response(interaction_id, decision);
                self.interaction_response_command(interaction_id, response)
            }
            ChatInputAreaOutcome::QueryResponse {
                interaction_id,
                answers,
            } => {
                let response = self
                    .interaction_bindings
                    .get(&interaction_id)?
                    .query_response(interaction_id, answers);
                self.interaction_response_command(interaction_id, response)
            }
            ChatInputAreaOutcome::Queue(submission) => {
                self.thread.update(ThreadPresentationEvent::UserSubmitted(
                    submission.display_text.clone(),
                ));
                Some(AppCommand::SubmitTurn { submission })
            }
            ChatInputAreaOutcome::SubmissionRejected(error) => {
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(error));
                None
            }
            ChatInputAreaOutcome::Submit(submission) => {
                self.thread.update(ThreadPresentationEvent::UserSubmitted(
                    submission.display_text.clone(),
                ));
                if self.turn_input_mode == TurnInputMode::SteerTurn {
                    let steer_id = self
                        .chat_input_area
                        .begin_steer(submission.display_text.clone());
                    return Some(AppCommand::SteerTurn {
                        steer_id,
                        submission,
                    });
                }
                self.status = Status::Working;
                self.turn_input_mode = TurnInputMode::SteerTurn;
                Some(AppCommand::SubmitTurn { submission })
            }
            ChatInputAreaOutcome::TextPromptSubmitted { pane_id, value } => {
                let Some(PaneActions::ConfigTextPrompt { provider }) =
                    self.pane_actions.get(&pane_id)
                else {
                    return None;
                };
                Some(AppCommand::SetProviderApiKey(
                    crate::features::config::ProviderApiKeyEdit::new(provider.clone(), value),
                ))
            }
            ChatInputAreaOutcome::Consumed => None,
            ChatInputAreaOutcome::Unhandled => None,
            ChatInputAreaOutcome::PaneDismissed(pane_id) => {
                self.pane_actions.remove(&pane_id);
                self.root_escape_sequence.reset();
                None
            }
        }
    }

    fn activate_selection_item(
        &mut self,
        pane_id: PaneId,
        item_id: &ListSelectionItemId,
    ) -> Option<AppCommand> {
        if let Some(PaneActions::Keymap(actions)) = self.pane_actions.get(&pane_id) {
            let action = actions.get(item_id)?.clone();
            return self.apply_keymap_action(action);
        }
        match self.pane_actions.get(&pane_id)? {
            PaneActions::ReadOnly => None,
            PaneActions::AdditionalDirectories(actions) => match actions.get(item_id)? {
                AdditionalDirectorySelectionAction::Remove { root } => {
                    Some(AppCommand::RemoveAdditionalDirectory { root: root.clone() })
                }
            },
            PaneActions::Config(actions) => match actions.get(item_id)?.clone() {
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
                    let prompt =
                        crate::features::config::provider_api_key_prompt(provider, display_name);
                    let pane_id = self.chat_input_area.push_text_prompt(prompt.spec);
                    self.pane_actions.insert(
                        pane_id,
                        PaneActions::ConfigTextPrompt {
                            provider: prompt.provider,
                        },
                    );
                    None
                }
            },
            PaneActions::Connectors(actions) => match actions.get(item_id)? {
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
            PaneActions::Mcp(actions) => match actions.get(item_id)? {
                McpSelectionAction::SetEnablement {
                    server_id,
                    enablement,
                } => Some(AppCommand::SetMcpEnablement {
                    server_id: server_id.clone(),
                    enablement: *enablement,
                }),
            },
            PaneActions::Model(actions) => match actions.get(item_id)? {
                ModelSelectionAction::Select { preference } => {
                    Some(AppCommand::SetPreferredModel {
                        preference: preference.clone(),
                    })
                }
            },
            PaneActions::Rewind(actions) => match actions.get(item_id)? {
                RewindSelectionAction::Rewind {
                    before_turn_id,
                    checkpoint_label,
                } => Some(AppCommand::RewindToCheckpoint {
                    before_turn_id: before_turn_id.clone(),
                    checkpoint_label: checkpoint_label.clone(),
                }),
            },
            PaneActions::Sessions(actions) => match actions.get(item_id)? {
                SessionSelectionAction::Resume { session_id } => Some(AppCommand::ResumeSession {
                    session_id: session_id.clone(),
                }),
            },
            PaneActions::Skills(actions) => match actions.get(item_id)?.clone() {
                SkillSelectionAction::SetEnablement {
                    skill_id,
                    enablement,
                } => Some(AppCommand::SetSkillEnablement {
                    skill_id,
                    enablement,
                }),
            },
            PaneActions::StatusLine(actions) => match actions.get(item_id)? {
                StatusLineSelectionAction::SetEnabled(edit) => {
                    Some(AppCommand::EditStatusLine(edit.clone()))
                }
            },
            PaneActions::Theme(actions) => match actions.get(item_id)? {
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
            PaneActions::ConfigTextPrompt { .. }
            | PaneActions::Keymap(_)
            | PaneActions::KeymapCapture(_) => None,
        }
    }

    fn apply_keymap_action(&mut self, action: KeymapAction) -> Option<AppCommand> {
        match action {
            KeymapAction::OpenAction { action, revision } => {
                self.show_keymap_pane(keymap_action_menu(action, revision));
                None
            }
            KeymapAction::BeginCapture {
                action,
                revision,
                intent,
                mode,
            } => {
                let (model, capture) = keymap_capture_pane_spec(action, revision, intent, mode);
                self.root_escape_sequence.reset();
                let pane_id = self.chat_input_area.push_key_capture(model);
                self.pane_actions
                    .insert(pane_id, PaneActions::KeymapCapture(capture));
                None
            }
            KeymapAction::ClearUser {
                command_id,
                revision,
            } => Some(AppCommand::EditKeymap(KeymapEdit {
                expected_revision: revision,
                command_id,
                kind: KeymapEditKind::ClearUser,
            })),
        }
    }

    fn interaction_response_command(
        &mut self,
        interaction_id: ChatInputAreaInteractionId,
        response: Result<crate::features::interactions::InteractionResponse, String>,
    ) -> Option<AppCommand> {
        match response {
            Ok(response) => Some(AppCommand::ResolveInteraction(response)),
            Err(error) => {
                self.chat_input_area
                    .interaction_submission_failed(interaction_id, error.clone());
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(error));
                None
            }
        }
    }

    pub(crate) fn select_input_overlay_choice(&mut self, index: usize) -> bool {
        self.accepts_input() && self.chat_input_area.select_overlay_choice(index)
    }

    pub(crate) fn activate_input_overlay_choice(&mut self, index: usize) -> Option<AppCommand> {
        if !self.accepts_input() {
            return None;
        }
        let outcome = self.chat_input_area.activate_overlay_choice(index)?;
        self.handle_chat_input_area_outcome(outcome)
    }

    pub(crate) fn replace_chat_input_catalog(
        &mut self,
        slash_commands: SlashCommandCatalog,
        skills: Vec<SkillSelectorItem>,
        plugins: Vec<MentionPluginItem>,
    ) {
        self.chat_input_area
            .replace_chat_input_catalog(slash_commands, skills, plugins);
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, text: &str) {
        if self.accepts_input() {
            self.chat_input_area.insert_text(text);
        }
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        if self.accepts_input()
            && let Err(error) = self.chat_input_area.handle_paste(pasted)
        {
            self.thread
                .update(ThreadPresentationEvent::FailureReported(error));
        }
    }

    fn attach_image_bytes(&mut self, bytes: Vec<u8>) {
        if self.accepts_input()
            && let Err(error) = self.chat_input_area.attach_image_bytes(bytes)
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
        self.chat_input_area.text()
    }

    pub(crate) fn input_cursor_width(&self) -> usize {
        self.chat_input_area.cursor_display_width()
    }

    pub(crate) fn input_cursor_line(&self) -> usize {
        self.chat_input_area.cursor_line()
    }

    pub(crate) fn chat_input_desired_height(&self, available_width: u16) -> u16 {
        self.chat_input_area
            .chat_input_desired_height(available_width)
    }

    pub(crate) fn suggest(&self) -> Option<SuggestView<'_>> {
        self.chat_input_area.suggest()
    }

    pub(crate) fn input_overlay(&self) -> Option<ChatInputAreaOverlayView<'_>> {
        self.chat_input_area.overlay()
    }

    pub(crate) fn input_height_entries(&self) -> Vec<ChatInputAreaHeightEntryView<'_>> {
        self.chat_input_area.height_entries()
    }

    pub(crate) fn toggle_plan_progress(&mut self) -> bool {
        self.chat_input_area.toggle_plan_progress()
    }

    pub(crate) fn chat_input_focused(&self) -> bool {
        self.chat_input_area.query_answer_active()
            || (!self.chat_input_area.pane_active() && self.input_overlay().is_none())
    }

    pub(crate) fn mouse_mode(&self) -> MouseMode {
        if self.terminal_settings.mouse_interactions() {
            self.chat_input_area.mouse_mode()
        } else {
            MouseMode::TerminalSelection
        }
    }

    fn show_list_selection_pane(&mut self, model: PaneSpec<ListSelectionModel>) {
        self.push_list_selection_pane(model, PaneActions::ReadOnly);
    }

    fn show_detail_pane(&mut self, spec: PaneSpec<crate::components::detail_list::DetailList>) {
        let pane_id = self.chat_input_area.push_detail_list(spec);
        self.pane_actions.insert(pane_id, PaneActions::ReadOnly);
    }

    fn show_additional_directories_pane(&mut self, pane_spec: AdditionalDirectoryPaneSpec) {
        self.push_list_selection_pane(
            pane_spec.model,
            PaneActions::AdditionalDirectories(pane_spec.actions),
        );
    }

    fn replace_additional_directories_pane(&mut self, pane_spec: AdditionalDirectoryPaneSpec) {
        self.replace_list_selection_pane(
            pane_spec.model,
            PaneActions::AdditionalDirectories(pane_spec.actions),
        );
    }

    fn show_interaction_request(&mut self, request: InteractionRequest) {
        let result = match request {
            InteractionRequest::Approval { binding, spec } => self
                .chat_input_area
                .show_approval(spec)
                .map(|interaction_id| (interaction_id, binding)),
            InteractionRequest::Query { binding, questions } => self
                .chat_input_area
                .show_query(questions)
                .map(|interaction_id| (interaction_id, binding)),
        };
        match result {
            Ok((interaction_id, binding)) => {
                self.interaction_bindings.insert(interaction_id, binding);
            }
            Err(error) => {
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(error));
                self.status = Status::Error;
                self.turn_input_mode = TurnInputMode::StartTurn;
            }
        }
    }

    fn show_skills_pane(&mut self, pane_spec: SkillPaneSpec) {
        self.push_list_selection_pane(pane_spec.model, PaneActions::Skills(pane_spec.actions));
    }

    fn show_mcp_pane(&mut self, pane_spec: McpPaneSpec) {
        self.push_list_selection_pane(pane_spec.model, PaneActions::Mcp(pane_spec.actions));
    }

    fn show_connector_pane(&mut self, pane_spec: ConnectorPaneSpec) {
        self.push_list_selection_pane(pane_spec.model, PaneActions::Connectors(pane_spec.actions));
    }

    fn replace_connector_pane(&mut self, pane_spec: ConnectorPaneSpec) {
        self.replace_list_selection_pane(
            pane_spec.model,
            PaneActions::Connectors(pane_spec.actions),
        );
    }

    pub(crate) fn connector_pane_open(&self) -> bool {
        matches!(self.top_pane_actions(), Some(PaneActions::Connectors(_)))
    }

    fn replace_mcp_pane(&mut self, pane_spec: McpPaneSpec) {
        self.replace_list_selection_pane(pane_spec.model, PaneActions::Mcp(pane_spec.actions));
    }

    fn show_model_pane(&mut self, pane_spec: ModelPaneSpec) {
        self.push_list_selection_pane(pane_spec.model, PaneActions::Model(pane_spec.actions));
    }

    fn show_rewind_pane(&mut self, pane_spec: RewindPaneSpec) {
        self.push_list_selection_pane(pane_spec.model, PaneActions::Rewind(pane_spec.actions));
    }

    fn show_session_pane(&mut self, pane_spec: SessionPaneSpec) {
        self.push_list_selection_pane(pane_spec.model, PaneActions::Sessions(pane_spec.actions));
    }

    fn push_list_selection_pane(
        &mut self,
        model: PaneSpec<ListSelectionModel>,
        actions: PaneActions,
    ) {
        self.root_escape_sequence.reset();
        let pane_id = self.chat_input_area.push_list_selection(model);
        self.pane_actions.insert(pane_id, actions);
    }

    fn replace_list_selection_pane(
        &mut self,
        model: PaneSpec<ListSelectionModel>,
        actions: PaneActions,
    ) {
        self.root_escape_sequence.reset();
        if let Some(pane_id) = self.chat_input_area.update_top_list_selection(model) {
            self.pane_actions.insert(pane_id, actions);
        }
    }

    fn close_list_selection_pane(&mut self) {
        self.root_escape_sequence.reset();
        if let Some(pane_id) = self.chat_input_area.pop_pane() {
            self.pane_actions.remove(&pane_id);
        }
    }

    fn replace_skills_pane(&mut self, pane_spec: SkillPaneSpec) {
        self.replace_list_selection_pane(pane_spec.model, PaneActions::Skills(pane_spec.actions));
    }

    fn show_theme_pane(&mut self, pane_spec: ThemePaneSpec) {
        self.push_list_selection_pane(pane_spec.model, PaneActions::Theme(pane_spec.actions));
    }

    fn show_keymap_pane(&mut self, pane_spec: KeymapPaneSpec) {
        self.push_list_selection_pane(pane_spec.model, PaneActions::Keymap(pane_spec.actions));
    }

    fn show_status_line_pane(&mut self, pane_spec: StatusLinePaneSpec) {
        self.push_list_selection_pane(pane_spec.model, PaneActions::StatusLine(pane_spec.actions));
    }

    fn replace_status_line_pane(&mut self, pane_spec: StatusLinePaneSpec) {
        self.replace_list_selection_pane(
            pane_spec.model,
            PaneActions::StatusLine(pane_spec.actions),
        );
    }

    fn close_keymap_panes(&mut self) {
        self.root_escape_sequence.reset();
        while matches!(
            self.top_pane_actions(),
            Some(PaneActions::Keymap(_) | PaneActions::KeymapCapture(_))
        ) {
            self.close_list_selection_pane();
        }
    }

    fn close_theme_panes(&mut self) {
        self.root_escape_sequence.reset();
        while matches!(self.top_pane_actions(), Some(PaneActions::Theme(_))) {
            self.close_list_selection_pane();
        }
    }

    pub(crate) fn skills_view_is_active(&self) -> bool {
        matches!(self.top_pane_actions(), Some(PaneActions::Skills(_)))
    }

    pub(crate) fn list_selection(&self) -> Option<&ListSelectionState> {
        self.chat_input_area.list_selection()
    }

    pub(crate) fn list_selection_pane(&self) -> Option<PaneView<'_, ListSelectionState>> {
        self.chat_input_area.list_selection_pane()
    }

    pub(crate) fn select_visible_item(&mut self, index: usize) -> bool {
        self.chat_input_area.select_visible_item(index)
    }

    pub(crate) fn select_tab(&mut self, index: usize) -> bool {
        self.chat_input_area.select_tab(index)
    }

    pub(crate) fn activate_visible_item(&mut self, index: usize) -> Option<AppCommand> {
        let outcome = self.chat_input_area.activate_visible_item(index)?;
        self.handle_chat_input_area_outcome(outcome)
    }

    pub(crate) fn mention_query(&self) -> Option<&str> {
        self.chat_input_area.mention_query()
    }

    pub(crate) fn messages(&self) -> &[Message] {
        self.thread.messages()
    }

    pub(crate) fn latest_agent_response(&self) -> Option<&str> {
        crate::components::chat_history::latest_agent_response(self.messages())
    }

    pub(crate) fn transcript_markdown(&self) -> String {
        crate::components::chat_history::export_markdown(self.messages())
    }

    pub(crate) fn transcript_scroll(&self) -> &ChatHistoryScroll {
        &self.transcript_scroll
    }

    pub(crate) fn welcome(&self) -> &WelcomeModel {
        &self.welcome
    }

    pub(crate) fn status(&self) -> &Status {
        &self.status
    }

    pub(crate) fn steers_active_turn(&self) -> bool {
        self.turn_input_mode == TurnInputMode::SteerTurn
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
        ) || self.chat_input_area.interaction_active()
    }

    pub(crate) fn update(&mut self, event: AppEvent) {
        match event {
            AppEvent::AdditionalDirectoriesPaneOpened(view) => {
                self.show_additional_directories_pane(view)
            }
            AppEvent::AdditionalDirectoryRemoved { root, pane_spec } => {
                self.replace_additional_directories_pane(pane_spec);
                self.thread
                    .update(ThreadPresentationEvent::NoticeReceived(format!(
                        "Removed additional directory {}",
                        root.display()
                    )));
                self.status = Status::Ready;
                self.turn_input_mode = TurnInputMode::StartTurn;
            }
            AppEvent::ClipboardImageRead(Ok(bytes)) => self.attach_image_bytes(bytes),
            AppEvent::ClipboardImageRead(Err(error)) => self.record_clipboard_error(error),
            AppEvent::ConfigSettingsReceived(settings) => self.terminal_settings = settings,
            AppEvent::ConfigPaneOpened(view) => self.show_config_pane(view),
            AppEvent::ConfigPaneReplaced(view) => self.replace_config_pane(view),
            AppEvent::ConfigApiKeySaved {
                provider,
                pane_spec,
            } => {
                self.close_list_selection_pane();
                self.replace_config_pane(pane_spec);
                self.thread
                    .update(ThreadPresentationEvent::NoticeReceived(format!(
                        "Saved API key for {provider}"
                    )));
                self.status = Status::Ready;
                self.turn_input_mode = TurnInputMode::StartTurn;
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
                self.turn_input_mode = TurnInputMode::StartTurn;
            }
            AppEvent::FailureReported(error) => {
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(error));
                self.status = Status::Error;
                self.turn_input_mode = TurnInputMode::StartTurn;
            }
            AppEvent::FileSearchSnapshotReceived(snapshot) => {
                self.chat_input_area.apply_file_search_snapshot(snapshot);
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
                self.turn_input_mode = TurnInputMode::SteerTurn;
            }
            AppEvent::ProductNotice(notice) => {
                self.thread
                    .update(ThreadPresentationEvent::NoticeReceived(notice));
                self.status = Status::Ready;
                self.turn_input_mode = TurnInputMode::StartTurn;
            }
            AppEvent::InteractionRequestOpened(request) => self.show_interaction_request(request),
            AppEvent::InteractionResolved(interaction_id) => {
                if self.chat_input_area.resolve_interaction(interaction_id) {
                    self.interaction_bindings.remove(&interaction_id);
                }
            }
            AppEvent::InteractionSubmissionFailed {
                interaction_id,
                error,
            } => {
                self.chat_input_area
                    .interaction_submission_failed(interaction_id, error);
            }
            AppEvent::KeymapPaneOpened(view) => self.show_keymap_pane(view),
            AppEvent::KeymapPanesClosed => self.close_keymap_panes(),
            AppEvent::StatusLineSettingsReceived(settings) => {
                self.status_line.apply_settings(settings)
            }
            AppEvent::StatusLinePaneOpened(view) => self.show_status_line_pane(view),
            AppEvent::StatusLinePaneReplaced(view) => self.replace_status_line_pane(view),
            AppEvent::ConnectorPaneOpened(view) => self.show_connector_pane(view),
            AppEvent::ConnectorPaneReplaced(view) => self.replace_connector_pane(view),
            AppEvent::McpPaneOpened(view) => self.show_mcp_pane(view),
            AppEvent::McpPaneReplaced(view) => self.replace_mcp_pane(view),
            AppEvent::ModelPaneOpened(view) => self.show_model_pane(view),
            AppEvent::RewindPaneOpened(view) => self.show_rewind_pane(view),
            AppEvent::SessionPaneOpened(view) => self.show_session_pane(view),
            AppEvent::DetailPaneOpened(spec) => self.show_detail_pane(spec),
            AppEvent::ListSelectionPaneClosed => self.close_list_selection_pane(),
            AppEvent::ListSelectionPaneOpened(model) => self.show_list_selection_pane(model),
            AppEvent::SkillsPaneOpened(view) => self.show_skills_pane(view),
            AppEvent::SkillsPaneReplaced(view) => self.replace_skills_pane(view),
            AppEvent::SteerCompleted(steer_id) => {
                self.chat_input_area.finish_steer(steer_id);
            }
            AppEvent::SteerSubmissionFailed { steer_id, error } => {
                self.chat_input_area.finish_steer(steer_id);
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(format!(
                        "could not steer the active Turn: {error}"
                    )));
            }
            AppEvent::ThemePanesClosed => self.close_theme_panes(),
            AppEvent::ThemePaneOpened(view) => self.show_theme_pane(view),
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
                self.chat_input_area.clear_steers();
                self.status = Status::Ready;
                self.turn_input_mode = TurnInputMode::StartTurn;
            }
            AppEvent::TurnActivityChanged(activity) => {
                self.status = match activity {
                    TurnActivity::Starting => Status::Working,
                    TurnActivity::Working => Status::Working,
                    TurnActivity::WaitingForApproval => Status::WaitingForApproval,
                    TurnActivity::WaitingForUserInput => Status::WaitingForUserInput,
                    TurnActivity::WaitingForCapability => Status::WaitingForCapability,
                    TurnActivity::Cancelling => Status::Cancelling,
                };
                self.turn_input_mode = match activity {
                    TurnActivity::Working => TurnInputMode::SteerTurn,
                    TurnActivity::Starting
                    | TurnActivity::WaitingForApproval
                    | TurnActivity::WaitingForUserInput
                    | TurnActivity::WaitingForCapability
                    | TurnActivity::Cancelling => TurnInputMode::StartTurn,
                };
            }
            AppEvent::TurnInputStatusChanged { plan, queued_turns } => {
                self.chat_input_area.replace_turn_status(plan, queued_turns)
            }
            AppEvent::PendingInteractionChanged(pending) => {
                let stale = self
                    .interaction_bindings
                    .iter()
                    .filter_map(|(interaction_id, binding)| {
                        let current = pending.as_ref().is_some_and(|(turn_id, request_id)| {
                            binding.matches_request(turn_id, request_id)
                        });
                        (!current).then_some(*interaction_id)
                    })
                    .collect::<Vec<_>>();
                for interaction_id in stale {
                    self.interaction_bindings.remove(&interaction_id);
                    self.chat_input_area.resolve_interaction(interaction_id);
                }
            }
            AppEvent::TurnCompleted => {
                self.status = Status::Ready;
                self.turn_input_mode = TurnInputMode::StartTurn;
                self.chat_input_area.clear_steers();
                self.chat_input_area.replace_turn_status(None, Vec::new());
            }
            AppEvent::TurnInterrupted => {
                self.thread.update(ThreadPresentationEvent::Interrupted);
                self.status = Status::Ready;
                self.turn_input_mode = TurnInputMode::StartTurn;
                self.chat_input_area.clear_steers();
                self.chat_input_area.replace_turn_status(None, Vec::new());
            }
        }
    }

    fn app_keymap_context(&self, is_press: bool) -> AppKeymapContext {
        AppKeymapContext {
            accepts_input: self.accepts_input(),
            has_selection: self.list_selection().is_some(),
            chat_input_empty: self.input().is_empty(),
            is_press,
        }
    }

    fn handle_app_key(&mut self, key: KeyEvent, now: Instant) -> Option<AppCommand> {
        let keymap_context = self.app_keymap_context(key.kind == KeyEventKind::Press);
        if let Some(action) = self.app_keymap.resolve_single(&key, keymap_context) {
            return self.apply_app_keymap_action(action, now);
        }
        if self.list_selection().is_none() && self.transcript_scroll.handle_key(key) {
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
                .any(|argument| matches!(argument, ChatInputItem::Image { .. }))
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
                Some(AppCommand::OpenKeymapPane)
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
                if self.turn_input_mode == TurnInputMode::SteerTurn {
                    let steer_id = self
                        .chat_input_area
                        .begin_steer(submission.display_text.clone());
                    return Some(AppCommand::SteerTurn {
                        steer_id,
                        submission,
                    });
                }
                self.status = Status::Working;
                self.turn_input_mode = TurnInputMode::SteerTurn;
                Some(AppCommand::SubmitTurn { submission })
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
use crate::components::chat_input::SlashCommandCatalog;
use crate::components::chat_input::SlashCommandInvocation;
use crate::components::chat_input::SuggestView;
use crate::components::chat_input::TuiSlashCommandAction;
use zeta_slash_commands::SlashCommandOrigin;
