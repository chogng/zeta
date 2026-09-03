use super::chat_panel::ChatPanel;
use super::command::AppCommand;
use super::command_panel::CommandPanel;
use super::command_panel::CommandPanelOutcome;
use super::escape::ScreenEscapeOutcome;
use super::escape::ScreenEscapeSequence;
use super::event::AppEvent;
use super::frame::InputPointerTarget;
use super::help::help_choices;
use crate::app::top_tip::TopTip;
use crate::app::welcome::WelcomeModel;
use crate::config::ConfigSelectionAction;
use crate::config::FollowUpMode;
use crate::config::TerminalSettings;
use crate::connectors::ConnectorChoices;
use crate::connectors::ConnectorSelectionAction;
use crate::dirs::DirChoices;
use crate::dirs::DirSelectionAction;
use crate::host::clipboard::ClipboardImageAvailability;
use crate::keymap::AppChordMatch;
use crate::keymap::AppKeymap;
use crate::keymap::AppKeymapAction;
use crate::keymap::AppKeymapContext;
use crate::keymap::KeymapChoices;
use crate::keymap::KeymapEditorOutcome;
use crate::mcp::McpChoices;
use crate::mcp::McpSelectionAction;
use crate::models::ModelChoices;
use crate::models::ModelSelectionAction;
use crate::render::RenderContext;
use crate::render::RenderTheme;
use crate::sessions::SessionChoices;
use crate::sessions::SessionManagerPointerTarget;
use crate::sessions::SessionManagerView;
use crate::sessions::SessionSelectionAction;
use crate::sessions::SessionsState;
use crate::sessions::TerminalScreen;
use crate::skills::{SkillChoices, SkillDiagnosticWarnings, SkillSelectionAction};
use crate::status::StatusLineChoices;
use crate::status::StatusLineModel;
use crate::status::StatusLineRuntime;
use crate::status::StatusLineSelectionAction;
use crate::terminal::mouse::MouseMode;
use crate::terminal::mouse::PointerInteraction;
use crate::terminal::screen_selection::ScreenSelection;
use crate::terminal::screen_selection::ScreenSelectionOutcome;
use crate::theme::ThemeChoices;
use crate::theme::ThemePickerOutcome;
use crate::thread::AgentThreadSwitcher;
use crate::thread::AgentThreadSwitcherView;
use crate::thread::ThreadPresentationEvent;
use crate::thread::ThreadPresentationStore;
use crate::thread::ThreadRequestIdentity;
use crate::thread::ThreadState;
use crate::thread::TurnActivity;
use crate::thread::TurnApprovalModes;
use crate::thread::composer::ChatComposerOutcome;
use crate::thread::composer::ChatComposerView;
use crate::thread::composer::ChatInputCatalog;
use crate::thread::composer::ChatInputItem;
use crate::thread::queue::QueueChoices;
use crate::thread::queue::QueueInput;
use crate::thread::queue::QueueSelectionAction;
use crate::thread::queue::QueueView;
use crate::thread::rewind::RewindChoices;
use crate::thread::rewind::RewindSelectionAction;
use crate::thread::transcript::ChatHistoryRenderCache;
use crate::thread::transcript::ChatHistoryScroll;
use crate::thread::transcript::Message;
use crate::thread::transcript::TranscriptScrollDirection;
use crate::widgets::detail_list::DetailList;
use crate::widgets::detail_list::DetailListRow;
use crate::widgets::list_selection::ListSelectionAdjustment;
use crate::widgets::list_selection::ListSelectionState;
use crate::widgets::overlay::DetailOverlay;
use crate::widgets::overlay::OverlayInputOutcome;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::layout::Position;
use std::path::Path;
use std::path::PathBuf;
use std::time::Instant;
use zeta_protocol::ApprovalMode;
use zeta_protocol::Turn;
use zeta_protocol::TurnId;

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
enum EmptyInputNavigation {
    PreviousScreen,
    NextScreen,
    FocusManager,
    FocusAgentThreads,
}

fn empty_input_navigation(
    screen: Option<&TerminalScreen>,
    key: KeyCode,
) -> Option<EmptyInputNavigation> {
    match key {
        KeyCode::Left => Some(EmptyInputNavigation::PreviousScreen),
        KeyCode::Right => Some(EmptyInputNavigation::NextScreen),
        KeyCode::Esc if matches!(screen, Some(TerminalScreen::Manager)) => {
            Some(EmptyInputNavigation::NextScreen)
        }
        KeyCode::Up if matches!(screen, Some(TerminalScreen::Manager)) => {
            Some(EmptyInputNavigation::FocusManager)
        }
        KeyCode::Down if matches!(screen, Some(TerminalScreen::Session(_))) => {
            Some(EmptyInputNavigation::FocusAgentThreads)
        }
        _ => None,
    }
}

#[derive(Debug)]
pub(crate) struct App {
    chat_panel: ChatPanel,
    pub(super) app_keymap: AppKeymap,
    thread: ThreadState,
    thread_presentations: ThreadPresentationStore,
    sessions: SessionsState,
    agent_thread_switcher: AgentThreadSwitcher,
    overlay: Option<DetailOverlay>,
    welcome: WelcomeModel,
    screen_escape_sequence: ScreenEscapeSequence,
    status: Status,
    terminal_settings: TerminalSettings,
    pointer: PointerInteraction<InputPointerTarget>,
    screen_selection: ScreenSelection,
    render_theme: RenderTheme,
    render_theme_revision: u64,
    skill_diagnostic_warnings: SkillDiagnosticWarnings,
}

impl App {
    #[cfg(test)]
    pub(crate) fn new() -> Self {
        Self {
            chat_panel: ChatPanel::new(),
            app_keymap: AppKeymap::default(),
            thread: ThreadState::default(),
            thread_presentations: ThreadPresentationStore::new(
                zeta_protocol::ThreadId::new("tui-local").expect("the local Thread ID is valid"),
            ),
            sessions: SessionsState::default(),
            agent_thread_switcher: AgentThreadSwitcher::default(),
            overlay: None,
            welcome: WelcomeModel::for_workspace(Path::new(".")),
            screen_escape_sequence: ScreenEscapeSequence::default(),
            status: Status::Ready,
            terminal_settings: TerminalSettings::default(),
            pointer: PointerInteraction::default(),
            screen_selection: ScreenSelection::default(),
            render_theme: RenderTheme::fallback(),
            render_theme_revision: 0,
            skill_diagnostic_warnings: SkillDiagnosticWarnings::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_dir(dir_root: &Path) -> Self {
        Self::for_dir_with_input_catalog(dir_root, ChatInputCatalog::default())
    }

    #[cfg(test)]
    pub(crate) fn for_dir_with_slash_commands(
        dir_root: &Path,
        slash_commands: SlashCommandCatalog,
    ) -> Self {
        Self::for_dir_with_input_catalog(
            dir_root,
            ChatInputCatalog::with_slash_commands(slash_commands),
        )
    }

    pub(crate) fn for_dir_with_input_catalog(
        dir_root: &Path,
        input_catalog: ChatInputCatalog,
    ) -> Self {
        Self {
            chat_panel: ChatPanel::new(),
            app_keymap: AppKeymap::default(),
            thread: ThreadState::default(),
            thread_presentations: ThreadPresentationStore::with_input_catalog(
                zeta_protocol::ThreadId::new("tui-local").expect("the local Thread ID is valid"),
                input_catalog,
            ),
            sessions: SessionsState::default(),
            agent_thread_switcher: AgentThreadSwitcher::default(),
            overlay: None,
            welcome: WelcomeModel::for_workspace(dir_root),
            screen_escape_sequence: ScreenEscapeSequence::default(),
            status: Status::Ready,
            terminal_settings: TerminalSettings::default(),
            pointer: PointerInteraction::default(),
            screen_selection: ScreenSelection::default(),
            render_theme: RenderTheme::fallback(),
            render_theme_revision: 0,
            skill_diagnostic_warnings: SkillDiagnosticWarnings::default(),
        }
    }

    pub(crate) fn render_context(&self) -> RenderContext<'_> {
        RenderContext::new(&self.render_theme, self.render_theme_revision)
    }

    pub(crate) fn handle_key(&mut self, key: KeyEvent) -> Option<AppCommand> {
        self.handle_key_at(key, Instant::now())
    }

    fn handle_key_at(&mut self, key: KeyEvent, now: Instant) -> Option<AppCommand> {
        if key.kind == KeyEventKind::Press {
            self.pointer.clear();
        }
        if let Some(overlay) = self.overlay.as_mut() {
            if overlay.handle_key(key) == OverlayInputOutcome::Dismiss {
                self.close_overlay();
            }
            return None;
        }
        if matches!(self.sessions.screen(), Some(TerminalScreen::Session(_))) {
            if let Some(command) = self.handle_thread_request_key(key) {
                return command;
            }
            if self.chat_panel.request_active() {
                return None;
            }
        }
        if let Some(outcome) = self.chat_panel.handle_command_key(key) {
            return self.handle_command_panel_outcome(outcome);
        }
        let temporary_interaction_active = self.completion().is_some();
        let is_screen_escape_press = key.kind == KeyEventKind::Press
            && key.code == KeyCode::Esc
            && key.modifiers.is_empty()
            && !temporary_interaction_active;
        if key.kind == KeyEventKind::Press && !is_screen_escape_press {
            self.screen_escape_sequence.reset();
        }
        let keymap_context = self.app_keymap_context(key.kind == KeyEventKind::Press);
        match self.app_keymap.route_chord(&key, keymap_context, now) {
            AppChordMatch::PassThrough => {}
            AppChordMatch::Pending | AppChordMatch::Consumed => return None,
            AppChordMatch::Command(action) => {
                return self.apply_app_keymap_action(action, now);
            }
        }
        if let Some(command) = self.handle_screen_navigation_key(key) {
            return command;
        }
        if self.handle_transcript_selection_key(key) {
            return None;
        }
        if !self.accepts_input() {
            self.screen_escape_sequence.reset();
            return self.handle_app_key(key, now);
        }

        let outcome = self.chat_panel.handle_composer_key(
            &mut self.thread_presentations.active_mut().input,
            key,
            self.terminal_settings.follow_up_mode(),
        );
        if matches!(outcome, ChatComposerOutcome::Unhandled) {
            return self.handle_app_key(key, now);
        }
        self.handle_chat_composer_outcome(outcome, now)
    }

    fn handle_chat_composer_outcome(
        &mut self,
        outcome: ChatComposerOutcome,
        now: Instant,
    ) -> Option<AppCommand> {
        match outcome {
            ChatComposerOutcome::Command(command) => self.handle_slash_command(command),
            ChatComposerOutcome::SubmissionRejected(error) => {
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(error));
                None
            }
            ChatComposerOutcome::Queued(input) => {
                self.thread_presentations.active_mut().queue.push(input);
                None
            }
            ChatComposerOutcome::Submit(submission) => {
                if matches!(self.sessions.screen(), Some(TerminalScreen::Manager)) {
                    self.status = Status::Working;
                    return Some(AppCommand::CreateSessionAndEnter { submission });
                }
                let starts_conversation = !self.thread.has_user_message();
                self.thread.update(ThreadPresentationEvent::UserSubmitted(
                    submission.display_text.clone(),
                ));
                if starts_conversation {
                    self.chat_panel.show_policy_tip(now);
                }
                if self.chat_panel.is_steering() {
                    let steer_id = self.chat_panel.begin_steer(submission.display_text.clone());
                    return Some(AppCommand::SteerTurn {
                        steer_id,
                        submission,
                    });
                }
                self.status = Status::Working;
                self.chat_panel.queue_input();
                Some(AppCommand::SubmitTurn { submission })
            }
            ChatComposerOutcome::Consumed => None,
            ChatComposerOutcome::Unhandled => None,
        }
    }

    fn handle_thread_request_key(&mut self, key: KeyEvent) -> Option<Option<AppCommand>> {
        self.chat_panel
            .handle_request_key(key)
            .map(|response| response.map(AppCommand::ResolveThreadRequest))
    }

    fn close_thread_request(&mut self, request: &ThreadRequestIdentity) {
        self.chat_panel.close_request(request);
    }

    fn fail_thread_request(&mut self, request: &ThreadRequestIdentity, error: String) {
        self.chat_panel.fail_request(request, error);
    }

    fn handle_command_panel_outcome(&mut self, outcome: CommandPanelOutcome) -> Option<AppCommand> {
        match outcome {
            CommandPanelOutcome::Dirs(DirSelectionAction::Remove { path }) => {
                Some(AppCommand::RemoveDir { path })
            }
            CommandPanelOutcome::Dirs(DirSelectionAction::SetPermissions(params)) => {
                Some(AppCommand::SetDirPermissions(params))
            }
            CommandPanelOutcome::Config(outcome) => self.handle_config_editor_outcome(outcome),
            CommandPanelOutcome::Connectors(ConnectorSelectionAction::ConnectDeviceOAuth {
                connector_id,
                connection_generation,
            }) => Some(AppCommand::ConnectConnectorDeviceOAuth {
                connector_id,
                connection_generation,
            }),
            CommandPanelOutcome::Connectors(ConnectorSelectionAction::Disconnect {
                connector_id,
            }) => Some(AppCommand::DisconnectConnector { connector_id }),
            CommandPanelOutcome::Keymap(KeymapEditorOutcome::Edit(edit)) => {
                Some(AppCommand::EditKeymap(edit))
            }
            CommandPanelOutcome::Keymap(KeymapEditorOutcome::Consumed) => None,
            CommandPanelOutcome::Keymap(KeymapEditorOutcome::Dismiss) => {
                self.close_command_panel();
                None
            }
            CommandPanelOutcome::Mcp(McpSelectionAction::SetEnablement {
                server_id,
                enablement,
            }) => Some(AppCommand::SetMcpEnablement {
                server_id,
                enablement,
            }),
            CommandPanelOutcome::Model(ModelSelectionAction::Select { preference }) => {
                Some(AppCommand::SetPreferredModel { preference })
            }
            CommandPanelOutcome::Queue(QueueSelectionAction::Select(queue_id)) => {
                self.open_queue_detail(queue_id);
                None
            }
            CommandPanelOutcome::QueueInput {
                input,
                action: QueueSelectionAction::Select(queue_id),
            } => self.handle_queue_input(input, queue_id),
            CommandPanelOutcome::Rewind(RewindSelectionAction::Rewind {
                before_turn_id,
                checkpoint_label,
            }) => Some(AppCommand::RewindToCheckpoint {
                before_turn_id,
                checkpoint_label,
            }),
            CommandPanelOutcome::Sessions(SessionSelectionAction::Resume { session_id }) => {
                Some(AppCommand::ResumeSession {
                    session_id,
                    preferred_thread_id: None,
                })
            }
            CommandPanelOutcome::Skills(SkillSelectionAction::SetEnablement {
                skill_id,
                enablement,
            }) => Some(AppCommand::SetSkillEnablement {
                skill_id,
                enablement,
            }),
            CommandPanelOutcome::StatusLine(StatusLineSelectionAction::SetEnabled(edit)) => {
                Some(AppCommand::EditStatusLine(edit))
            }
            CommandPanelOutcome::Theme(outcome) => self.handle_theme_picker_outcome(outcome),
            CommandPanelOutcome::Consumed => None,
            CommandPanelOutcome::Dismiss => {
                self.close_command_panel();
                None
            }
        }
    }

    fn handle_config_editor_outcome(
        &mut self,
        outcome: crate::config::ConfigEditorOutcome,
    ) -> Option<AppCommand> {
        match outcome {
            crate::config::ConfigEditorOutcome::Action(
                ConfigSelectionAction::SetTerminalSettings(edit),
            ) => Some(AppCommand::EditConfig(edit)),
            crate::config::ConfigEditorOutcome::Action(
                ConfigSelectionAction::ChooseFollowUpMode { queue, steer },
            ) => Some(AppCommand::EditConfig(
                match self.terminal_settings.follow_up_mode() {
                    FollowUpMode::Queue => *steer,
                    FollowUpMode::Steer => *queue,
                },
            )),
            crate::config::ConfigEditorOutcome::Action(
                ConfigSelectionAction::ChooseInputMode { standard, vim },
            ) => Some(AppCommand::EditConfig(
                match self.terminal_settings.input_mode() {
                    crate::thread::composer::ChatInputMode::Standard => *vim,
                    crate::thread::composer::ChatInputMode::Vim => *standard,
                },
            )),
            crate::config::ConfigEditorOutcome::Action(
                ConfigSelectionAction::SetShowGitChangesAsDiff(edit),
            ) => Some(AppCommand::EditConfig(edit)),
            crate::config::ConfigEditorOutcome::Action(
                ConfigSelectionAction::OpenProviderApiKey { .. },
            ) => None,
            crate::config::ConfigEditorOutcome::Adjust(action, adjustment) => {
                Some(AppCommand::EditConfig(match action {
                    ConfigSelectionAction::ChooseFollowUpMode { queue, steer } => {
                        match adjustment {
                            ListSelectionAdjustment::Previous => *queue,
                            ListSelectionAdjustment::Next => *steer,
                        }
                    }
                    ConfigSelectionAction::ChooseInputMode { standard, vim } => match adjustment {
                        ListSelectionAdjustment::Previous => *standard,
                        ListSelectionAdjustment::Next => *vim,
                    },
                    _ => return None,
                }))
            }
            crate::config::ConfigEditorOutcome::SaveApiKey(edit) => {
                Some(AppCommand::SetProviderApiKey(edit))
            }
            crate::config::ConfigEditorOutcome::Consumed => None,
            crate::config::ConfigEditorOutcome::Dismiss => {
                self.close_command_panel();
                None
            }
        }
    }

    fn handle_theme_picker_outcome(&mut self, outcome: ThemePickerOutcome) -> Option<AppCommand> {
        match outcome {
            ThemePickerOutcome::Select { preference } => {
                self.close_command_panel();
                Some(AppCommand::SetTheme { preference })
            }
            ThemePickerOutcome::SelectCustom { preference } => {
                self.close_command_panel();
                Some(AppCommand::SetCustomTheme { preference })
            }
            ThemePickerOutcome::OpenCustomThemes => Some(AppCommand::OpenCustomThemePicker),
            ThemePickerOutcome::Consumed => None,
            ThemePickerOutcome::Dismiss => {
                self.close_command_panel();
                None
            }
        }
    }

    pub(crate) fn activate_input_completion(&mut self, index: usize) -> Option<AppCommand> {
        if !self.accepts_input() {
            return None;
        }
        let outcome = self
            .chat_panel
            .activate_completion(&mut self.thread_presentations.active_mut().input, index)?;
        self.handle_chat_composer_outcome(outcome, Instant::now())
    }

    pub(crate) fn activate_thread_request_choice(&mut self, index: usize) -> Option<AppCommand> {
        self.chat_panel
            .activate_request_choice(index)
            .map(AppCommand::ResolveThreadRequest)
    }

    pub(crate) fn toggle_transcript_cell(&mut self, render_key: &str) -> bool {
        let cell_id = crate::thread::TranscriptCellId::from_render_key(render_key);
        if !self
            .thread
            .cells()
            .iter()
            .any(|cell| cell.cell_id() == &cell_id && cell.can_expand())
        {
            return false;
        }
        self.thread_presentations.active_mut().toggle_cell(&cell_id);
        true
    }

    pub(crate) fn open_transcript_cell_details(&mut self, render_key: &str) -> bool {
        let cell_id = crate::thread::TranscriptCellId::from_render_key(render_key);
        let Some(details) = self.thread.details(&cell_id) else {
            return false;
        };
        self.thread_presentations.active_mut().selected_cell = Some(cell_id);
        self.show_overlay(DetailList::new(
            "Transcript cell",
            vec![DetailListRow::new("Content", details)],
        ));
        true
    }

    pub(crate) fn replace_chat_input_catalog(&mut self, catalog: ChatInputCatalog) {
        self.thread_presentations.replace_input_catalog(catalog);
    }

    #[cfg(test)]
    pub(crate) fn insert_text(&mut self, text: &str) {
        if self.accepts_input() {
            self.chat_panel
                .insert_text(&mut self.thread_presentations.active_mut().input, text);
        }
    }

    pub(crate) fn handle_paste(&mut self, pasted: String) {
        self.pointer.clear();
        if self.overlay.is_some() {
            return;
        }
        if matches!(self.sessions.screen(), Some(TerminalScreen::Session(_)))
            && self.chat_panel.request_active()
        {
            self.chat_panel.handle_request_paste(pasted);
            return;
        }
        if self.chat_panel.command_active() {
            self.chat_panel.handle_command_paste(pasted);
            return;
        }
        if self.accepts_input()
            && let Err(error) = self
                .chat_panel
                .handle_input_paste(&mut self.thread_presentations.active_mut().input, pasted)
        {
            self.thread
                .update(ThreadPresentationEvent::FailureReported(error));
        }
    }

    fn attach_image_bytes(&mut self, bytes: Vec<u8>) {
        if self.accepts_input()
            && let Err(error) = self
                .chat_panel
                .attach_image_bytes(&mut self.thread_presentations.active_mut().input, bytes)
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
        self.thread_presentations.active().input.text()
    }

    pub(crate) fn chat_composer_view(&self) -> ChatComposerView<'_> {
        self.chat_panel
            .composer_view(&self.thread_presentations.active().input)
    }

    pub(crate) fn command_panel_key_hints(&self) -> Option<&str> {
        self.chat_panel.command_key_hints()
    }

    pub(crate) fn command_panel(&self) -> Option<&CommandPanel> {
        self.chat_panel.command()
    }

    pub(crate) fn overlay(&self) -> Option<&DetailOverlay> {
        self.overlay.as_ref()
    }

    pub(crate) fn completion(&self) -> Option<CompletionView<'_>> {
        if self.chat_panel.command_active() || self.overlay.is_some() {
            return None;
        }
        self.thread_presentations.active().input.completion()
    }

    pub(crate) fn chat_input_focused(&self) -> bool {
        self.overlay.is_none()
            && self.approval_view().is_none()
            && self.query_view().is_none()
            && !self.sessions.manager().focused()
            && !self.agent_thread_switcher.focused()
            && self.thread_presentations.active().selected_cell.is_none()
            && !self.chat_panel.command_active()
            && self.completion().is_none()
    }

    pub(crate) fn mouse_mode(&self) -> MouseMode {
        if self.terminal_settings.mouse_interactions() {
            MouseMode::TuiCapture
        } else {
            MouseMode::TerminalSelection
        }
    }

    pub(crate) fn update_pointer_hover(&mut self, target: Option<InputPointerTarget>) {
        self.pointer.update_hover(target);
    }

    pub(crate) fn hovered_pointer_target(&self) -> Option<&InputPointerTarget> {
        self.pointer.hovered()
    }

    pub(crate) fn update_pointer_pressed(&mut self, target: Option<InputPointerTarget>) {
        self.pointer.update_pressed(target);
    }

    pub(crate) fn clear_pointer_pressed(&mut self) {
        self.pointer.clear_pressed();
    }

    pub(crate) fn clear_pointer_interaction(&mut self) {
        self.pointer.clear();
    }

    pub(crate) fn pressed_pointer_target(&self) -> Option<&InputPointerTarget> {
        self.pointer.pressed()
    }

    pub(crate) const fn screen_selection(&self) -> &ScreenSelection {
        &self.screen_selection
    }

    pub(crate) fn begin_screen_selection(&mut self, position: Position) {
        self.screen_selection.begin(position);
    }

    pub(crate) fn drag_screen_selection(&mut self, position: Position) {
        self.screen_selection.drag(position);
    }

    pub(crate) fn finish_screen_selection(
        &mut self,
        position: Position,
        now: Instant,
    ) -> Option<ScreenSelectionOutcome> {
        self.screen_selection.finish(position, now)
    }

    pub(crate) fn select_screen_range(
        &mut self,
        range: crate::terminal::screen_selection::ScreenSelectionRange,
    ) {
        self.screen_selection.select(range);
    }

    #[cfg(test)]
    fn show_help(&mut self, model: crate::widgets::list_selection::ListSelectionModel) {
        self.open_command_panel(CommandPanel::help(model));
    }

    pub(crate) fn approval_view(
        &self,
    ) -> Option<crate::thread::interaction::approval::ApprovalView<'_>> {
        matches!(self.sessions.screen(), Some(TerminalScreen::Session(_)))
            .then(|| self.chat_panel.approval_view())
            .flatten()
    }

    pub(crate) fn query_view(&self) -> Option<crate::thread::interaction::query::QueryView<'_>> {
        matches!(self.sessions.screen(), Some(TerminalScreen::Session(_)))
            .then(|| self.chat_panel.query_view())
            .flatten()
    }

    pub(crate) fn transcript_selection_active(&self) -> bool {
        matches!(self.sessions.screen(), Some(TerminalScreen::Session(_)))
            && self.thread_presentations.active().selected_cell.is_some()
    }

    fn open_command_panel(&mut self, panel: CommandPanel) {
        self.screen_escape_sequence.reset();
        self.overlay = None;
        self.chat_panel.open_command(panel);
        self.pointer.clear();
    }

    fn close_command_panel(&mut self) {
        self.screen_escape_sequence.reset();
        self.chat_panel.close_command();
        self.pointer.clear();
    }

    fn show_overlay(&mut self, detail: DetailList) {
        self.screen_escape_sequence.reset();
        self.overlay = Some(DetailOverlay::new(detail));
        self.pointer.clear();
    }

    fn close_overlay(&mut self) {
        self.screen_escape_sequence.reset();
        self.overlay = None;
        self.pointer.clear();
    }

    fn close_transient_surfaces(&mut self) {
        self.screen_escape_sequence.reset();
        self.chat_panel.close_command();
        self.overlay = None;
        self.pointer.clear();
    }

    fn show_dirs_picker(&mut self, spec: DirChoices) {
        self.open_command_panel(CommandPanel::dirs(spec));
    }

    fn show_queue_picker(&mut self, spec: QueueChoices) {
        self.open_command_panel(CommandPanel::queue(spec));
    }

    fn update_queue_picker(&mut self) {
        let spec = crate::thread::queue::choices(&self.queue_view());
        self.chat_panel.replace_queue(spec);
    }

    fn open_queue_detail(&mut self, queue_id: crate::thread::queue::QueueId) {
        let Some(text) = self
            .queue_view()
            .items
            .into_iter()
            .find(|item| item.id == queue_id)
            .map(|item| item.text.to_owned())
        else {
            return;
        };
        self.show_overlay(DetailList::new(
            "Queued message",
            vec![DetailListRow::new("Message", text)],
        ));
    }

    fn handle_queue_input(
        &mut self,
        input: QueueInput,
        queue_id: crate::thread::queue::QueueId,
    ) -> Option<AppCommand> {
        match input {
            QueueInput::Restore => {
                let state = self.thread_presentations.active_mut();
                match state.queue.restore(queue_id, &mut state.input) {
                    Ok(()) => self.close_command_panel(),
                    Err(error) => self
                        .thread
                        .update(ThreadPresentationEvent::FailureReported(error)),
                }
                None
            }
            QueueInput::Delete => {
                self.thread_presentations
                    .active_mut()
                    .queue
                    .delete(queue_id);
                self.update_queue_picker();
                None
            }
            QueueInput::MoveUp => {
                self.thread_presentations
                    .active_mut()
                    .queue
                    .move_up(queue_id);
                self.update_queue_picker();
                None
            }
            QueueInput::MoveDown => {
                self.thread_presentations
                    .active_mut()
                    .queue
                    .move_down(queue_id);
                self.update_queue_picker();
                None
            }
            QueueInput::Send => {
                if matches!(self.status, Status::Working) {
                    self.thread.update(ThreadPresentationEvent::FailureReported(
                        "finish or interrupt the active Turn before sending this queued message"
                            .into(),
                    ));
                    return None;
                }
                let submission = self
                    .thread_presentations
                    .active_mut()
                    .queue
                    .begin_send(queue_id)?;
                self.close_command_panel();
                self.thread.update(ThreadPresentationEvent::UserSubmitted(
                    submission.display_text.clone(),
                ));
                self.status = Status::Working;
                self.chat_panel.queue_input();
                Some(AppCommand::SubmitQueuedTurn {
                    queue_id,
                    submission,
                })
            }
        }
    }

    fn update_dirs_picker(&mut self, spec: DirChoices) {
        self.chat_panel.replace_dirs(spec);
    }

    fn show_skill_settings(&mut self, choices: SkillChoices) {
        let SkillChoices {
            model,
            actions,
            diagnostics,
        } = choices;
        self.report_skill_diagnostics(&diagnostics);
        self.open_command_panel(CommandPanel::skills(SkillChoices {
            model,
            actions,
            diagnostics: Vec::new(),
        }));
    }

    fn show_mcp_settings(&mut self, spec: McpChoices) {
        self.open_command_panel(CommandPanel::mcp(spec));
    }

    fn show_connector_picker(&mut self, spec: ConnectorChoices) {
        self.open_command_panel(CommandPanel::connectors(spec));
    }

    fn update_connector_picker(&mut self, spec: ConnectorChoices) {
        self.chat_panel.replace_connectors(spec);
    }

    pub(crate) fn connector_picker_open(&self) -> bool {
        self.chat_panel.command_is_connectors()
    }

    fn update_mcp_settings(&mut self, spec: McpChoices) {
        self.chat_panel.replace_mcp(spec);
    }

    fn show_model_picker(&mut self, spec: ModelChoices) {
        self.open_command_panel(CommandPanel::model(spec));
    }

    fn show_rewind_picker(&mut self, spec: RewindChoices) {
        self.open_command_panel(CommandPanel::rewind(spec));
    }

    fn show_session_picker(&mut self, spec: SessionChoices) {
        self.open_command_panel(CommandPanel::sessions(spec));
    }

    fn update_skill_settings(&mut self, choices: SkillChoices) {
        let SkillChoices {
            model,
            actions,
            diagnostics,
        } = choices;
        self.report_skill_diagnostics(&diagnostics);
        self.chat_panel.replace_skills(SkillChoices {
            model,
            actions,
            diagnostics: Vec::new(),
        });
    }

    fn report_skill_diagnostics(
        &mut self,
        diagnostics: &[zeta_app_server_protocol::protocol::skills::SkillDiagnosticDto],
    ) {
        for notice in self.skill_diagnostic_warnings.update(diagnostics) {
            self.thread
                .update(ThreadPresentationEvent::NoticeReceived(notice));
        }
    }

    fn show_theme_picker(&mut self, spec: ThemeChoices) {
        self.open_command_panel(CommandPanel::theme(spec));
    }

    fn show_keymap_editor(&mut self, spec: KeymapChoices) {
        self.open_command_panel(CommandPanel::keymap(spec));
    }

    fn show_status_line_editor(&mut self, spec: StatusLineChoices) {
        self.open_command_panel(CommandPanel::status_line(spec));
    }

    fn show_status_panel(&mut self, panel: crate::status::StatusPanel) {
        self.open_command_panel(CommandPanel::status(panel));
    }

    fn update_status_line_editor(&mut self, spec: StatusLineChoices) {
        self.chat_panel.replace_status_line(spec);
    }

    pub(crate) fn skills_view_is_active(&self) -> bool {
        self.chat_panel.command_is_skills()
    }

    pub(crate) fn list_selection(&self) -> Option<&ListSelectionState> {
        self.chat_panel.command_list_selection()
    }

    pub(crate) fn select_tab(&mut self, index: usize) -> bool {
        self.chat_panel.select_command_tab(index)
    }

    pub(crate) fn focus_composer_search(&mut self) -> bool {
        self.chat_panel.focus_command_search()
    }

    pub(crate) fn activate_visible_item(&mut self, index: usize) -> Option<AppCommand> {
        let outcome = self.chat_panel.activate_command_item(index)?;
        self.handle_command_panel_outcome(outcome)
    }

    pub(crate) fn mention_query(&self) -> Option<&str> {
        if self.chat_panel.command_active() {
            return None;
        }
        self.thread_presentations.active().input.mention_query()
    }

    pub(crate) fn messages(&self) -> &[Message] {
        self.thread.messages()
    }

    pub(crate) fn transcript_views(&self) -> Vec<Message> {
        self.thread.views(
            &self.thread_presentations.active().expanded_cells,
            self.thread_presentations.active().selected_cell.as_ref(),
        )
    }

    pub(crate) fn latest_agent_response(&self) -> Option<&str> {
        crate::thread::transcript::latest_agent_response(self.messages())
    }

    pub(crate) fn transcript_markdown(&self) -> String {
        crate::thread::transcript::export_markdown(self.messages())
    }

    pub(crate) fn transcript_scroll(&self) -> &ChatHistoryScroll {
        &self.thread_presentations.active().scroll
    }

    pub(crate) fn scroll_transcript(&mut self, direction: TranscriptScrollDirection) -> bool {
        self.thread_presentations
            .active_mut()
            .scroll
            .scroll(direction)
    }

    pub(crate) fn follow_latest_transcript(&mut self) {
        self.thread_presentations
            .active_mut()
            .scroll
            .follow_latest();
    }

    pub(crate) fn transcript_render_cache(&self) -> &ChatHistoryRenderCache {
        &self.thread_presentations.active().render_cache
    }

    pub(crate) fn welcome(&self) -> &WelcomeModel {
        &self.welcome
    }

    pub(crate) fn status(&self) -> &Status {
        &self.status
    }

    pub(crate) fn active_turn(&self) -> Option<&TurnId> {
        self.thread.active_turn()
    }

    pub(crate) fn set_active_turn(&mut self, turn_id: TurnId) {
        self.thread.set_active_turn(turn_id);
    }

    pub(crate) fn set_active_turn_if_idle(&mut self, turn_id: TurnId) {
        self.thread.set_active_turn_if_idle(turn_id);
    }

    pub(crate) fn clear_active_turn(&mut self) {
        self.thread.clear_active_turn();
    }

    pub(crate) fn sync_active_turn(
        &mut self,
        turns: &[Turn],
    ) -> Vec<crate::thread::ActiveTurnUpdate> {
        self.thread.sync_active_turn(turns)
    }

    pub(crate) fn steers_active_turn(&self) -> bool {
        self.chat_panel.is_steering()
    }

    pub(crate) fn queue_view(&self) -> QueueView<'_> {
        self.thread_presentations.active().queue.view()
    }

    pub(crate) fn goal_view(&self) -> Option<&zeta_protocol::ThreadGoal> {
        self.thread_presentations.active().goal.as_ref()
    }

    pub(crate) fn plan_view(&self) -> Option<crate::thread::plan::PlanInlineView<'_>> {
        self.thread_presentations.active().plan.view()
    }

    pub(crate) fn session_manager_view(&self) -> Option<SessionManagerView<'_>> {
        matches!(self.sessions.screen(), Some(TerminalScreen::Manager))
            .then(|| self.sessions.manager().view(self.sessions.catalog()))
    }

    #[cfg(test)]
    pub(crate) fn session_manager_focused(&self) -> bool {
        matches!(self.sessions.screen(), Some(TerminalScreen::Manager))
            && self.sessions.manager().focused()
    }

    pub(crate) fn session_manager_hint(&self) -> &'static str {
        self.sessions.manager().status_hint()
    }

    pub(crate) fn activate_session_manager_pointer_target(
        &mut self,
        target: SessionManagerPointerTarget,
    ) {
        let catalog = self.sessions.catalog().to_vec();
        if let Some(preview) = self
            .sessions
            .manager_mut()
            .activate_pointer_target(target, &catalog)
        {
            self.show_overlay(preview);
        }
    }

    pub(crate) fn scroll_session_manager(&mut self, up: bool) -> bool {
        if !matches!(self.sessions.screen(), Some(TerminalScreen::Manager)) {
            return false;
        }
        let catalog = self.sessions.catalog().to_vec();
        let manager = self.sessions.manager_mut();
        manager.focus();
        if up {
            manager.select_previous(&catalog);
        } else {
            manager.select_next(&catalog);
        }
        true
    }

    pub(crate) fn screen_navigation_tip(&self) -> Option<&'static str> {
        if !self.chat_input_focused() || !self.input().is_empty() {
            return None;
        }
        match self.sessions.previous_screen()? {
            TerminalScreen::Manager => Some("← for agents"),
            TerminalScreen::Session(_) => None,
        }
    }

    pub(crate) fn agent_thread_switcher_view(&self) -> Option<AgentThreadSwitcherView<'_>> {
        matches!(self.sessions.screen(), Some(TerminalScreen::Session(_)))
            .then(|| self.agent_thread_switcher.view())
    }

    pub(crate) fn agent_thread_switcher_rows(&self) -> u16 {
        if matches!(self.sessions.screen(), Some(TerminalScreen::Session(_))) {
            self.agent_thread_switcher.desired_rows()
        } else {
            0
        }
    }

    pub(crate) fn agent_thread_switcher_focused(&self) -> bool {
        self.agent_thread_switcher.focused()
    }

    pub(crate) fn dispatch_next_queued_turn(&mut self) -> Option<AppCommand> {
        let (queue_id, submission) = self
            .thread_presentations
            .active_mut()
            .queue
            .begin_next_send()?;
        self.thread.update(ThreadPresentationEvent::UserSubmitted(
            submission.display_text.clone(),
        ));
        self.status = Status::Working;
        self.chat_panel.queue_input();
        Some(AppCommand::SubmitQueuedTurn {
            queue_id,
            submission,
        })
    }

    pub(crate) fn approval_mode_status(&self) -> TurnApprovalModes {
        self.thread.approval_modes()
    }

    pub(crate) fn approval_mode(&self) -> ApprovalMode {
        self.thread.approval_mode()
    }

    pub(crate) fn cycle_next_approval_mode(&mut self, now: Instant) {
        self.thread.cycle_approval_mode();
        if self.thread.has_user_message() {
            self.chat_panel.show_policy_tip(now);
        }
    }

    #[cfg(test)]
    pub(crate) fn set_next_approval_mode(&mut self, approval_mode: ApprovalMode) {
        self.thread.set_next_approval_mode(approval_mode);
    }

    pub(crate) fn set_current_approval_mode(&mut self, approval_mode: Option<ApprovalMode>) {
        self.thread.set_current_approval_mode(approval_mode);
    }

    pub(crate) fn status_line(&self) -> &StatusLineModel {
        self.chat_panel.status_line()
    }

    pub(crate) fn request_git_text_diff(&mut self) -> bool {
        self.chat_panel.status_line_mut().request_git_text_diff()
    }

    pub(crate) fn top_tip(&self) -> &TopTip {
        self.chat_panel.top_tip()
    }

    pub(crate) fn show_policy_tip(&mut self, now: Instant) {
        self.chat_panel.show_policy_tip(now);
    }

    pub(crate) fn status_line_runtime(&self) -> StatusLineRuntime {
        let plan = self.plan_view().map(|view| (view.completed, view.total));
        let queue = self.queue_view().items.len();
        let visible_session = match self.sessions.screen() {
            Some(TerminalScreen::Session(session_id)) => Some(session_id),
            Some(TerminalScreen::Manager) | None => None,
        };
        let viewed_thread =
            visible_session.and_then(|session_id| self.sessions.remembered_thread(session_id));
        let subagents = visible_session
            .and_then(|session_id| {
                self.sessions
                    .catalog()
                    .iter()
                    .find(|session| &session.session_id == session_id)
            })
            .map(|session| {
                session
                    .threads
                    .iter()
                    .filter(|thread| {
                        thread.status == zeta_protocol::ThreadStatus::Active
                            && thread.parent_thread_id.is_some()
                            && thread.forked_from_id.is_none()
                            && Some(&thread.thread_id) != viewed_thread
                    })
                    .count()
            })
            .unwrap_or(0);
        StatusLineRuntime {
            plan,
            queue,
            subagents,
        }
    }

    pub(crate) fn accepts_input(&self) -> bool {
        self.approval_view().is_none()
            && self.query_view().is_none()
            && self.viewed_thread_accepts_input()
            && matches!(
                &self.status,
                Status::Ready | Status::Working | Status::Error
            )
    }

    pub(crate) fn viewed_thread_completed(&self) -> bool {
        !self.viewed_thread_accepts_input()
            && matches!(self.sessions.screen(), Some(TerminalScreen::Session(_)))
    }

    fn viewed_thread_accepts_input(&self) -> bool {
        let Some(TerminalScreen::Session(session_id)) = self.sessions.screen() else {
            return true;
        };
        let Some(thread_id) = self.sessions.remembered_thread(session_id) else {
            return true;
        };
        self.sessions
            .catalog()
            .iter()
            .find(|session| &session.session_id == session_id)
            .and_then(|session| {
                session
                    .threads
                    .iter()
                    .find(|thread| &thread.thread_id == thread_id)
            })
            .is_none_or(|thread| thread.status == zeta_protocol::ThreadStatus::Active)
    }

    pub(crate) fn update(&mut self, event: AppEvent) {
        self.pointer.clear();
        match event {
            AppEvent::DirPickerOpened(view) => self.show_dirs_picker(view),
            AppEvent::DirRemoved { path, choices } => {
                self.update_dirs_picker(choices);
                self.thread
                    .update(ThreadPresentationEvent::NoticeReceived(format!(
                        "Removed directory {}",
                        path.display()
                    )));
                self.status = Status::Ready;
                self.chat_panel.start_input();
            }
            AppEvent::DirPermissionsUpdated(choices) => self.update_dirs_picker(choices),
            AppEvent::ClipboardImageRead(Ok(bytes)) => self.attach_image_bytes(bytes),
            AppEvent::ClipboardImageRead(Err(error)) => self.record_clipboard_error(error),
            AppEvent::ClipboardImageAvailabilityChanged(availability) => match availability {
                ClipboardImageAvailability::Available => self.chat_panel.show_clipboard_image(),
                ClipboardImageAvailability::Unavailable => self.chat_panel.hide_clipboard_image(),
            },
            AppEvent::ConfigSettingsReceived(settings) => {
                self.terminal_settings = settings;
                if !settings.mouse_interactions() {
                    self.pointer.clear();
                    self.screen_selection.clear();
                }
                self.thread_presentations
                    .set_input_mode(settings.input_mode());
            }
            AppEvent::ConfigUpdated(result) => {
                self.terminal_settings = result.terminal;
                self.chat_panel
                    .status_line_mut()
                    .apply_settings(result.status_line);
                if !result.terminal.mouse_interactions() {
                    self.screen_selection.clear();
                }
                self.thread_presentations
                    .set_input_mode(result.terminal.input_mode());
                self.chat_panel.replace_config(result.choices);
            }
            AppEvent::ConfigEditorOpened(view) => {
                self.open_command_panel(CommandPanel::config(view))
            }
            AppEvent::ConfigApiKeySaved { provider, choices } => {
                self.chat_panel.finish_config_prompt(choices);
                self.thread
                    .update(ThreadPresentationEvent::NoticeReceived(format!(
                        "Saved API key for {provider}"
                    )));
                self.status = Status::Ready;
                self.chat_panel.start_input();
            }
            AppEvent::ModelSummaryReceived(summary) => {
                self.chat_panel
                    .status_line_mut()
                    .apply_preferred_model(summary.preferred_model());
                self.welcome.apply_model_summary(&summary);
            }
            AppEvent::CommandStarted(command) => {
                self.thread
                    .update(ThreadPresentationEvent::CommandStarted(command));
            }
            AppEvent::CommandCompleted { command, result } => {
                self.thread
                    .update(ThreadPresentationEvent::CommandCompleted { command, result });
                self.status = Status::Ready;
                self.chat_panel.start_input();
            }
            AppEvent::FailureReported(error) => {
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(error));
                self.status = Status::Error;
                self.chat_panel.start_input();
            }
            AppEvent::FileSearchSnapshotReceived(snapshot) => {
                self.thread_presentations
                    .active_mut()
                    .input
                    .apply_file_search_snapshot(snapshot);
            }
            AppEvent::GitStatusReceived(status) => {
                self.chat_panel.status_line_mut().apply_git_status(&status)
            }
            AppEvent::GitTextDiffReceived { status, statistics } => self
                .chat_panel
                .status_line_mut()
                .apply_git_text_diff(status, statistics),
            AppEvent::HostOperationCompleted(Ok(notice)) => {
                self.thread
                    .update(ThreadPresentationEvent::NoticeReceived(notice));
            }
            AppEvent::HostOperationCompleted(Err(error)) => {
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(error));
            }
            AppEvent::TopTipNoticeShown(notice) => {
                self.chat_panel.show_notice(notice, Instant::now());
            }
            AppEvent::InterruptFailed(error) => {
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(format!(
                        "could not interrupt turn: {error}"
                    )));
                self.status = Status::Working;
                self.chat_panel.steer_input();
            }
            AppEvent::ProductNotice(notice) => {
                self.thread
                    .update(ThreadPresentationEvent::NoticeReceived(notice));
                self.status = Status::Ready;
                self.chat_panel.start_input();
            }
            AppEvent::ApprovalRequested(approval) => self.chat_panel.show_approval(approval),
            AppEvent::QueryRequested(query) => self.chat_panel.show_query(query),
            AppEvent::ThreadRequestResolved(request) => self.close_thread_request(&request),
            AppEvent::ThreadRequestSubmissionFailed { request, error } => {
                self.fail_thread_request(&request, error)
            }
            AppEvent::KeymapSettingsReceived(settings) => {
                self.app_keymap = settings.keymap;
                for diagnostic in settings.diagnostics {
                    self.report_keybinding_diagnostic(diagnostic);
                }
            }
            AppEvent::KeymapEditorOpened(update) => {
                self.app_keymap = update.settings.keymap;
                for diagnostic in update.settings.diagnostics {
                    self.report_keybinding_diagnostic(diagnostic);
                }
                if let Some(notice) = update.notice {
                    self.thread
                        .update(ThreadPresentationEvent::NoticeReceived(notice));
                }
                if self.chat_panel.command_is_keymap() {
                    self.chat_panel.replace_keymap(update.choices);
                } else {
                    self.show_keymap_editor(update.choices);
                }
            }
            AppEvent::StatusLineSettingsReceived(settings) => {
                self.chat_panel.status_line_mut().apply_settings(settings)
            }
            AppEvent::StatusLineEditorOpened(update) => {
                self.chat_panel
                    .status_line_mut()
                    .apply_settings(update.settings);
                self.show_status_line_editor(update.choices);
            }
            AppEvent::StatusLineEditorUpdated(update) => {
                self.chat_panel
                    .status_line_mut()
                    .apply_settings(update.settings);
                self.update_status_line_editor(update.choices);
            }
            AppEvent::ConnectorPickerOpened(view) => self.show_connector_picker(view),
            AppEvent::ConnectorPickerUpdated(view) => self.update_connector_picker(view),
            AppEvent::McpSettingsOpened(view) => self.show_mcp_settings(view),
            AppEvent::McpSettingsUpdated(view) => self.update_mcp_settings(view),
            AppEvent::ModelPickerOpened(view) => self.show_model_picker(view),
            AppEvent::RewindPickerOpened(view) => self.show_rewind_picker(view),
            AppEvent::SessionPickerOpened(view) => self.show_session_picker(view),
            AppEvent::SessionCatalogReceived(catalog) => {
                self.sessions.refresh_catalog(catalog);
                self.reconcile_agent_thread_switcher();
            }
            AppEvent::ThreadContextChanged {
                session_id,
                thread_id,
            } => {
                let context_changed = self.sessions.active_session_id() != Some(&session_id)
                    || self.sessions.remembered_thread(&session_id) != Some(&thread_id);
                self.close_transient_surfaces();
                self.thread_presentations.switch(thread_id.clone());
                self.sessions.activate_context(session_id, thread_id);
                self.chat_panel.status_line_mut().clear_thread_accounting();
                if context_changed {
                    self.chat_panel.reset_top_tip();
                }
                self.reconcile_agent_thread_switcher();
            }
            AppEvent::ThreadAccountingChanged {
                usage,
                reference_cost,
            } => self
                .chat_panel
                .status_line_mut()
                .apply_thread_accounting(&usage, &reference_cost),
            AppEvent::ThreadGoalChanged(goal) => {
                self.thread_presentations.active_mut().goal = goal;
            }
            AppEvent::StatusPanelOpened(panel) => {
                self.show_status_panel(panel);
            }
            AppEvent::CommandPanelClosed => self.close_command_panel(),
            #[cfg(test)]
            AppEvent::HelpOpened(model) => self.show_help(model),
            AppEvent::SkillSettingsOpened(view) => self.show_skill_settings(view),
            AppEvent::SkillSettingsUpdated(view) => self.update_skill_settings(view),
            AppEvent::SkillDiagnosticsReceived(diagnostics) => {
                self.report_skill_diagnostics(&diagnostics)
            }
            AppEvent::SteerCompleted(steer_id) => {
                self.chat_panel.finish_steer(steer_id);
            }
            AppEvent::SteerSubmissionFailed { steer_id, error } => {
                self.chat_panel.finish_steer(steer_id);
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(format!(
                        "could not steer the active Turn: {error}"
                    )));
            }
            AppEvent::QueueSubmissionCompleted(queue_id) => {
                self.thread_presentations
                    .active_mut()
                    .queue
                    .finish_send(queue_id);
            }
            AppEvent::QueueSubmissionFailed { queue_id, error } => {
                self.thread_presentations
                    .active_mut()
                    .queue
                    .fail_send(queue_id);
                self.thread
                    .update(ThreadPresentationEvent::FailureReported(format!(
                        "could not send the queued Turn: {error}"
                    )));
                self.status = Status::Error;
                self.chat_panel.start_input();
            }
            AppEvent::ThemePickerOpened(view) => {
                if self.chat_panel.command_is_theme() {
                    self.chat_panel.push_custom_theme(view);
                } else {
                    self.show_theme_picker(view);
                }
            }
            AppEvent::RenderThemeChanged(theme) => {
                self.render_theme = theme;
                self.render_theme_revision = self.render_theme_revision.wrapping_add(1).max(1);
            }
            AppEvent::ThreadTranscriptSnapshotReceived(transcript) => {
                self.thread
                    .update(ThreadPresentationEvent::TranscriptSnapshotReceived(
                        transcript,
                    ));
                self.hide_navigation_for_existing_conversation();
            }
            AppEvent::ThreadTranscriptHistoryPageReceived(transcript) => {
                self.thread
                    .update(ThreadPresentationEvent::TranscriptHistoryPageReceived(
                        transcript,
                    ));
                self.hide_navigation_for_existing_conversation();
            }
            AppEvent::ThreadTranscriptUpdateReceived(update) => {
                self.thread
                    .update(ThreadPresentationEvent::TranscriptUpdateReceived(update));
                self.hide_navigation_for_existing_conversation();
            }
            AppEvent::TranscriptCleared => {
                self.thread.update(ThreadPresentationEvent::Cleared);
                self.chat_panel.reset_top_tip();
                self.skill_diagnostic_warnings.clear();
                self.thread_presentations
                    .active_mut()
                    .scroll
                    .follow_latest();
                self.chat_panel.clear_steers();
                self.thread_presentations.active_mut().queue.clear();
                self.status = Status::Ready;
                self.chat_panel.start_input();
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
                self.chat_panel.apply_turn_activity(activity);
            }
            AppEvent::TurnPlanChanged(plan) => {
                self.thread_presentations.active_mut().plan.replace(plan)
            }
            AppEvent::PendingInteractionChanged(pending) => {
                self.chat_panel.reconcile_request(pending.as_ref());
            }
            AppEvent::TurnCompleted => {
                self.status = Status::Ready;
                self.chat_panel.start_input();
                self.chat_panel.clear_steers();
                self.thread_presentations.active_mut().plan.replace(None);
            }
            AppEvent::TurnInterrupted => {
                self.thread.update(ThreadPresentationEvent::Interrupted);
                self.status = Status::Ready;
                self.chat_panel.start_input();
                self.chat_panel.clear_steers();
                self.thread_presentations.active_mut().plan.replace(None);
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

    fn handle_screen_navigation_key(&mut self, key: KeyEvent) -> Option<Option<AppCommand>> {
        if key.kind != KeyEventKind::Press {
            return None;
        }
        if matches!(self.sessions.screen(), Some(TerminalScreen::Manager))
            && self.sessions.manager().focused()
        {
            let catalog = self.sessions.catalog().to_vec();
            if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('x') {
                let session_ids = self.sessions.manager().selected_archive_ids(&catalog);
                return Some(
                    (!session_ids.is_empty())
                        .then_some(AppCommand::ArchiveSessions { session_ids }),
                );
            }
            if !key.modifiers.is_empty() {
                return None;
            }
            return match key.code {
                KeyCode::Up => {
                    self.sessions.manager_mut().select_previous(&catalog);
                    Some(None)
                }
                KeyCode::Down => {
                    if !self.sessions.manager_mut().select_next(&catalog) {
                        self.sessions.manager_mut().blur();
                    }
                    Some(None)
                }
                KeyCode::Enter => Some(self.sessions.manager().selected_session().map(
                    |session_id| AppCommand::ResumeSession {
                        session_id: session_id.to_string(),
                        preferred_thread_id: self.sessions.remembered_thread(session_id).cloned(),
                    },
                )),
                KeyCode::Char(' ') => {
                    let preview = self.sessions.manager().preview_selected(&catalog);
                    if let Some(preview) = preview {
                        self.show_overlay(preview);
                    }
                    Some(None)
                }
                KeyCode::Char('p') => {
                    self.sessions.manager_mut().toggle_selected_pin();
                    Some(None)
                }
                KeyCode::Esc => {
                    self.sessions.manager_mut().blur();
                    Some(None)
                }
                _ => None,
            };
        }
        if !key.modifiers.is_empty() {
            return None;
        }
        if self.agent_thread_switcher.focused() {
            return match key.code {
                KeyCode::Up => {
                    if !self.agent_thread_switcher.select_previous() {
                        self.agent_thread_switcher.blur();
                    }
                    Some(None)
                }
                KeyCode::Down => {
                    self.agent_thread_switcher.select_next();
                    Some(None)
                }
                KeyCode::Enter => Some(
                    self.agent_thread_switcher
                        .selected()
                        .cloned()
                        .map(|thread_id| AppCommand::SwitchThread { thread_id }),
                ),
                KeyCode::Esc => {
                    self.agent_thread_switcher.blur();
                    Some(None)
                }
                _ => None,
            };
        }
        if !self.chat_input_focused() || !self.input().is_empty() {
            return None;
        }
        let target = match empty_input_navigation(self.sessions.screen(), key.code)? {
            EmptyInputNavigation::PreviousScreen => match self.sessions.previous_screen() {
                Some(target) => target,
                None => return Some(None),
            },
            EmptyInputNavigation::NextScreen => match self.sessions.next_screen() {
                Some(target) => target,
                None => return Some(None),
            },
            EmptyInputNavigation::FocusManager => {
                self.sessions.manager_mut().focus();
                return Some(None);
            }
            EmptyInputNavigation::FocusAgentThreads => {
                self.agent_thread_switcher.focus();
                return Some(None);
            }
        };
        match target {
            TerminalScreen::Manager => {
                self.close_transient_surfaces();
                self.sessions.show_manager();
                Some(None)
            }
            TerminalScreen::Session(session_id) => {
                if self.sessions.active_session_id() == Some(&session_id) {
                    let viewed = self
                        .sessions
                        .restorable_thread(&session_id)
                        .expect("the active Session has an active Thread");
                    self.close_transient_surfaces();
                    self.sessions.show_session(session_id, viewed);
                    Some(None)
                } else {
                    Some(Some(AppCommand::ResumeSession {
                        session_id: session_id.to_string(),
                        preferred_thread_id: self.sessions.remembered_thread(&session_id).cloned(),
                    }))
                }
            }
        }
    }

    fn reconcile_agent_thread_switcher(&mut self) {
        let visible_session_id = match self.sessions.screen() {
            Some(TerminalScreen::Session(session_id)) => Some(session_id.clone()),
            Some(TerminalScreen::Manager) | None => None,
        };
        let viewed_thread = visible_session_id
            .as_ref()
            .and_then(|session_id| self.sessions.remembered_thread(session_id))
            .cloned();
        let session = visible_session_id.as_ref().and_then(|session_id| {
            self.sessions
                .catalog()
                .iter()
                .find(|session| &session.session_id == session_id)
        });
        self.agent_thread_switcher
            .reconcile(session, viewed_thread.as_ref());
    }

    fn handle_app_key(&mut self, key: KeyEvent, now: Instant) -> Option<AppCommand> {
        let keymap_context = self.app_keymap_context(key.kind == KeyEventKind::Press);
        if let Some(action) = self.app_keymap.resolve_single(&key, keymap_context) {
            return self.apply_app_keymap_action(action, now);
        }
        if self.list_selection().is_none()
            && self
                .thread_presentations
                .active_mut()
                .scroll
                .handle_key(key)
        {
            return (key.code == KeyCode::Home).then_some(AppCommand::LoadOlderHistory);
        }
        None
    }

    fn handle_transcript_selection_key(&mut self, key: KeyEvent) -> bool {
        if key.kind != KeyEventKind::Press
            || !matches!(self.sessions.screen(), Some(TerminalScreen::Session(_)))
            || self.chat_panel.command_active()
            || self.completion().is_some()
            || !self.input().is_empty()
        {
            return false;
        }
        let cell_ids = self
            .thread
            .cells()
            .iter()
            .map(|cell| cell.cell_id().clone())
            .collect::<Vec<_>>();
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Up) => self
                .thread_presentations
                .active_mut()
                .select_previous_cell(&cell_ids),
            (KeyModifiers::CONTROL, KeyCode::Down) => self
                .thread_presentations
                .active_mut()
                .select_next_cell(&cell_ids),
            (KeyModifiers::NONE, KeyCode::Up)
                if self.thread_presentations.active().selected_cell.is_some() =>
            {
                self.thread_presentations
                    .active_mut()
                    .select_previous_cell(&cell_ids);
                true
            }
            (KeyModifiers::NONE, KeyCode::Down)
                if self.thread_presentations.active().selected_cell.is_some() =>
            {
                self.thread_presentations
                    .active_mut()
                    .select_next_cell(&cell_ids);
                true
            }
            (KeyModifiers::NONE, KeyCode::Char(' ')) => {
                let Some(selected) = self.thread_presentations.active().selected_cell.clone()
                else {
                    return false;
                };
                if self
                    .thread
                    .cells()
                    .iter()
                    .any(|cell| cell.cell_id() == &selected && cell.can_expand())
                {
                    self.thread_presentations
                        .active_mut()
                        .toggle_cell(&selected);
                }
                true
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                let Some(selected) = self.thread_presentations.active().selected_cell.clone()
                else {
                    return false;
                };
                self.open_transcript_cell_details(selected.as_str());
                true
            }
            (KeyModifiers::NONE, KeyCode::Esc)
                if self.thread_presentations.active().selected_cell.is_some() =>
            {
                self.thread_presentations.active_mut().selected_cell = None;
                true
            }
            _ => false,
        }
    }

    fn apply_app_keymap_action(
        &mut self,
        action: AppKeymapAction,
        now: Instant,
    ) -> Option<AppCommand> {
        match action {
            AppKeymapAction::CycleApprovalMode => Some(AppCommand::CycleNextApprovalMode),
            AppKeymapAction::ScreenEscape => match self.screen_escape_sequence.press(now) {
                ScreenEscapeOutcome::WaitingForSecondPress => None,
                ScreenEscapeOutcome::OpenRewind => Some(AppCommand::OpenRewindPicker),
            },
            AppKeymapAction::OpenRewind => Some(AppCommand::OpenRewindPicker),
            AppKeymapAction::ReadClipboardImage => Some(AppCommand::ReadClipboardImage),
            AppKeymapAction::InterruptOrQuit => self.quit_or_interrupt(),
            AppKeymapAction::CopyLastResponse => Some(AppCommand::CopyLastResponse),
            AppKeymapAction::Suspend => Some(AppCommand::Suspend),
        }
    }

    pub(crate) fn handle_tick(&mut self, now: Instant) -> bool {
        let context = self.app_keymap_context(true);
        let chord_expired = self.app_keymap.expire(context, now);
        let top_tip_changed = self.chat_panel.poll_top_tip(now);
        let elapsed_changed = self.agent_thread_switcher.refresh_elapsed();
        let manager_changed = matches!(self.sessions.screen(), Some(TerminalScreen::Manager))
            && self.sessions.refresh_manager_time(now);
        chord_expired || top_tip_changed || elapsed_changed || manager_changed
    }

    fn hide_navigation_for_existing_conversation(&mut self) {
        if self.thread.has_user_message() {
            self.chat_panel.hide_navigation();
        }
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
        if invocation.origin == SlashCommandOrigin::Local
            && local.is_some()
            && !matches!(local, Some(TuiSlashCommandAction::Quit))
        {
            self.thread
                .update(ThreadPresentationEvent::CommandSubmitted(
                    invocation.display_text(),
                ));
        }
        if invocation.origin == SlashCommandOrigin::Local && invocation.arguments.is_empty() {
            match local {
                Some(TuiSlashCommandAction::Sessions | TuiSlashCommandAction::Agents) => {
                    self.agent_thread_switcher.blur();
                    self.close_transient_surfaces();
                    self.sessions.show_manager();
                    return None;
                }
                Some(TuiSlashCommandAction::Subagents) => {
                    self.agent_thread_switcher.focus();
                    return None;
                }
                Some(TuiSlashCommandAction::Queue) => {
                    let spec = crate::thread::queue::choices(&self.queue_view());
                    self.show_queue_picker(spec);
                    return None;
                }
                _ => {}
            }
        }
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
            && !matches!(local, Some(TuiSlashCommandAction::Export))
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
            (SlashCommandOrigin::Local, Some(TuiSlashCommandAction::Export)) => {
                let requested_path = (!invocation.display_arguments.trim().is_empty())
                    .then(|| PathBuf::from(invocation.display_arguments.trim()));
                Some(AppCommand::ExportTranscript { requested_path })
            }
            (SlashCommandOrigin::Local, Some(TuiSlashCommandAction::Shortcuts))
                if invocation.arguments.is_empty() =>
            {
                Some(AppCommand::OpenKeymapEditor)
            }
            (SlashCommandOrigin::Local, Some(TuiSlashCommandAction::Config))
                if invocation.arguments.is_empty() =>
            {
                Some(AppCommand::OpenConfigEditor)
            }
            (SlashCommandOrigin::Local, Some(TuiSlashCommandAction::StatusLine))
                if invocation.arguments.is_empty() =>
            {
                Some(AppCommand::OpenStatusLineEditor)
            }
            (SlashCommandOrigin::Local, Some(TuiSlashCommandAction::Theme))
                if invocation.arguments.is_empty() =>
            {
                Some(AppCommand::OpenThemePicker)
            }
            (SlashCommandOrigin::Local, Some(TuiSlashCommandAction::Theme)) => {
                Some(AppCommand::SetTheme {
                    preference: invocation.display_arguments.trim().to_owned(),
                })
            }
            (SlashCommandOrigin::Local, Some(TuiSlashCommandAction::Help)) => {
                let spec = help_choices(
                    self.thread_presentations.slash_commands(),
                    self.app_keymap.setup_actions(),
                );
                self.open_command_panel(CommandPanel::help(spec));
                None
            }
            (SlashCommandOrigin::Server, _) => {
                let submission = invocation.into_forwarded_submission();
                self.thread.update(ThreadPresentationEvent::UserSubmitted(
                    submission.display_text.clone(),
                ));
                if self.chat_panel.is_steering() {
                    let steer_id = self.chat_panel.begin_steer(submission.display_text.clone());
                    return Some(AppCommand::SteerTurn {
                        steer_id,
                        submission,
                    });
                }
                self.status = Status::Working;
                self.chat_panel.queue_input();
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
use crate::thread::composer::CompletionView;
#[cfg(test)]
use crate::thread::composer::SlashCommandCatalog;
use crate::thread::composer::SlashCommandInvocation;
use crate::thread::composer::TuiSlashCommandAction;
use zeta_slash_commands::SlashCommandOrigin;
