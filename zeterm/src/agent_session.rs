use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::SyncSender;
use std::sync::mpsc::TryRecvError;
use std::thread;
use std::thread::JoinHandle;
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use zeta_app_server_client::SessionWorkspaceRoute;
use zeta_app_server_client::route_session_workspace;
use zeta_app_server_client::{
    AppServerEvent, AppServerEvents, AppServerRequestHandle, ClientError, ServerNotification,
};
use zeta_app_server_protocol::protocol::common::CommandId;
use zeta_app_server_protocol::protocol::config::{
    ConfigCommandResult, ConfigReadResult, LanguageServerConfigDto, LanguageServerConfigureParams,
    LanguageServerRemoveParams,
};
use zeta_app_server_protocol::protocol::fs::{
    FsChanged, FsGetMetadataParams, FsGetMetadataResult, FsReadDirectoryEntry,
    FsReadDirectoryParams, FsReadFileParams, FsWriteFileParams,
};
use zeta_app_server_protocol::protocol::git::{
    GitBranchDto, GitBranchSwitchParams, GitTextDiffResult,
};
use zeta_app_server_protocol::protocol::model::ModelCatalogEntry;
use zeta_app_server_protocol::protocol::session::{
    SessionCreateParams, SessionReadParams, SessionRequest, SessionRequestParams,
    SessionRequestResult, SessionSubscribeParams, SessionSubscribeResult, SessionThreadProjection,
    SessionUnsubscribeParams,
};
use zeta_app_server_protocol::protocol::slash_commands::SlashCommandDefinition;
use zeta_app_server_protocol::protocol::turn::InputItem;
use zeta_protocol::{
    ModelRef, Session, SessionId, SessionStatus, SessionThreadStatus, Thread, ThreadId,
    ThreadUpdateEnvelope,
};
use zeta_text_file::{
    TextFileAccess, TextFileDiskVersion, TextFileModifiedAt, TextFileSaveRequest, TextFileSnapshot,
};
use zui::app::AppProxy;

use crate::NativeApp;
use crate::agent_session_target::AgentSessionTarget;
use crate::composer_host::composer_model_options;
use crate::composer_host::synchronize_composer_classifier;
use crate::composer_host::update_composer_classifier;
use crate::native_event::NativeEvent;
use crate::session_switch_trace::{self, SwitchId};
use crate::sidebar_pane_workspace::AgentSidebarView;
use crate::tab_input::TabInputChange;
use crate::thread_projection::ThreadProjectionUpdate;

#[path = "agent_session_remote.rs"]
mod remote;

const COMMAND_QUEUE_CAPACITY: usize = 32;
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(25);
const FILE_SNAPSHOT_READ_ATTEMPTS: usize = 3;
pub(super) const AGENT_UNAVAILABLE_COMMAND_ERROR: &str =
    "Agent session is not connected; the command was not sent";

pub(super) struct AgentSessionFailure {
    pub(super) error: anyhow::Error,
    pub(super) retryable: bool,
    pub(super) connection_was_ready: bool,
}

impl AgentSessionFailure {
    fn connection(error: anyhow::Error) -> Self {
        Self {
            error,
            retryable: true,
            connection_was_ready: false,
        }
    }

    fn disconnected(error: anyhow::Error) -> Self {
        Self {
            error,
            retryable: true,
            connection_was_ready: true,
        }
    }

    fn fatal(error: anyhow::Error) -> Self {
        Self {
            error,
            retryable: false,
            connection_was_ready: false,
        }
    }
}

#[derive(Debug)]
struct AgentSessionConnectionLost(String);

impl fmt::Display for AgentSessionConnectionLost {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AgentSessionConnectionLost {}

#[derive(Debug)]
struct AgentSessionReconnect {
    root: PathBuf,
    preferred_session_id: Option<SessionId>,
}

impl fmt::Display for AgentSessionReconnect {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "reconnect local Workspace authority at {}",
            self.root.display()
        )
    }
}

impl std::error::Error for AgentSessionReconnect {}

#[derive(Debug)]
pub(crate) enum AgentSessionEvent {
    Catalog {
        slash_commands: Vec<SlashCommandDefinition>,
        models: Vec<ModelCatalogEntry>,
    },
    Configuration(ConfigReadResult),
    SessionCatalog(Vec<Session>),
    Snapshot {
        session: Session,
        thread: Thread,
        switch_id: Option<SwitchId>,
    },
    Update(Box<ThreadUpdateEnvelope>),
    GitProjection(Option<GitTextDiffResult>),
    FilesChanged(FsChanged),
    Error(String),
    Closed,
}

enum AgentSessionCommand {
    CreateSession,
    ActivateSession {
        session_id: SessionId,
        switch_id: SwitchId,
        response: SyncSender<std::result::Result<Option<WorkspaceSwitchProjection>, String>>,
    },
    SubmitAgentMessage(String),
    SubmitShellCommand(String),
    SelectModel(ModelRef),
    Refresh,
    RefreshGit,
    ReadDirectory {
        path: PathBuf,
        response: SyncSender<std::result::Result<Vec<FsReadDirectoryEntry>, String>>,
    },
    ReadFile {
        path: PathBuf,
        response: SyncSender<std::result::Result<TextFileSnapshot, String>>,
    },
    WriteFile {
        request: TextFileSaveRequest,
        response: SyncSender<std::result::Result<TextFileDiskVersion, String>>,
    },
    ListGitBranches(SyncSender<std::result::Result<Vec<GitBranchDto>, String>>),
    SwitchGitBranch {
        name: String,
        response: SyncSender<std::result::Result<GitTextDiffResult, String>>,
    },
    SwitchWorkspace {
        root: PathBuf,
        response: SyncSender<std::result::Result<WorkspaceSwitchProjection, String>>,
    },
    ConfigureLanguageServer {
        expected_revision: u64,
        server_id: String,
        config: LanguageServerConfigDto,
        response: SyncSender<std::result::Result<ConfigCommandResult, String>>,
    },
    RemoveLanguageServerConfiguration {
        expected_revision: u64,
        server_id: String,
        response: SyncSender<std::result::Result<ConfigCommandResult, String>>,
    },
    Shutdown,
}

pub(crate) struct WorkspaceSwitchProjection {
    pub(crate) root: PathBuf,
    pub(crate) git: Option<GitTextDiffResult>,
}

pub(crate) struct AgentSession {
    available: Arc<AtomicBool>,
    commands: SyncSender<AgentSessionCommand>,
    worker: Option<JoinHandle<()>>,
}

impl AgentSession {
    pub(crate) fn spawn(
        event_proxy: AppProxy<NativeEvent>,
        target: AgentSessionTarget,
    ) -> Result<Self> {
        let (commands, command_receiver) = mpsc::sync_channel(COMMAND_QUEUE_CAPACITY);
        let available = Arc::new(AtomicBool::new(false));
        let worker_availability = Arc::clone(&available);
        let worker = thread::Builder::new()
            .name("zeterm-agent-session".into())
            .spawn(move || {
                run_agent_session(event_proxy, command_receiver, target, worker_availability)
            })
            .context("could not start native Agent session worker")?;
        Ok(Self {
            available,
            commands,
            worker: Some(worker),
        })
    }

    pub(crate) fn submit_agent_message(&self, text: String) -> Result<()> {
        self.try_send(
            AgentSessionCommand::SubmitAgentMessage(text),
            "Agent submission queue is unavailable",
        )
    }

    pub(crate) fn create_session(&self) -> Result<()> {
        self.try_send(
            AgentSessionCommand::CreateSession,
            "Agent session creation queue is unavailable",
        )
    }

    pub(crate) fn activate_session(
        &self,
        session_id: SessionId,
        switch_id: SwitchId,
    ) -> Result<Option<WorkspaceSwitchProjection>> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::ActivateSession {
                session_id,
                switch_id,
                response,
            },
            "Agent session activation queue is unavailable",
        )?;
        result
            .recv()
            .context("Agent session activation worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    pub(crate) fn submit_shell_command(&self, command: String) -> Result<()> {
        self.try_send(
            AgentSessionCommand::SubmitShellCommand(command),
            "Shell submission queue is unavailable",
        )
    }

    pub(crate) fn select_model(&self, model: ModelRef) -> Result<()> {
        self.try_send(
            AgentSessionCommand::SelectModel(model),
            "Agent model selection queue is unavailable",
        )
    }

    pub(crate) fn refresh(&self) -> Result<()> {
        self.try_send(
            AgentSessionCommand::Refresh,
            "Agent refresh queue is unavailable",
        )
    }

    pub(crate) fn refresh_git(&self) -> Result<()> {
        self.try_send(
            AgentSessionCommand::RefreshGit,
            "Git refresh queue is unavailable",
        )
    }

    pub(crate) fn read_directory(&self, path: PathBuf) -> Result<Vec<FsReadDirectoryEntry>> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::ReadDirectory { path, response },
            "Workspace directory query queue is unavailable",
        )?;
        result
            .recv()
            .context("Workspace directory query worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    pub(crate) fn read_file(&self, path: PathBuf) -> Result<TextFileSnapshot> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::ReadFile { path, response },
            "Workspace file query queue is unavailable",
        )?;
        result
            .recv()
            .context("Workspace file query worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    pub(crate) fn write_file(&self, request: TextFileSaveRequest) -> Result<TextFileDiskVersion> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::WriteFile { request, response },
            "Workspace file mutation queue is unavailable",
        )?;
        result
            .recv()
            .context("Workspace file mutation worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    pub(crate) fn local_branches(&self) -> Result<Vec<GitBranchDto>> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::ListGitBranches(response),
            "Git branch query queue is unavailable",
        )?;
        result
            .recv()
            .context("Git branch query worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    pub(crate) fn switch_git_branch(&self, name: String) -> Result<GitTextDiffResult> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::SwitchGitBranch { name, response },
            "Git branch mutation queue is unavailable",
        )?;
        result
            .recv()
            .context("Git branch mutation worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    pub(crate) fn switch_workspace(&self, root: PathBuf) -> Result<WorkspaceSwitchProjection> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::SwitchWorkspace { root, response },
            "Workspace switch queue is unavailable",
        )?;
        result
            .recv()
            .context("Workspace switch worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    pub(crate) fn configure_language_server(
        &self,
        expected_revision: u64,
        server_id: String,
        config: LanguageServerConfigDto,
    ) -> Result<ConfigCommandResult> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::ConfigureLanguageServer {
                expected_revision,
                server_id,
                config,
                response,
            },
            "Language server configuration queue is unavailable",
        )?;
        result
            .recv()
            .context("Language server configuration worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    pub(crate) fn remove_language_server_configuration(
        &self,
        expected_revision: u64,
        server_id: String,
    ) -> Result<ConfigCommandResult> {
        let (response, result) = mpsc::sync_channel(1);
        self.try_send(
            AgentSessionCommand::RemoveLanguageServerConfiguration {
                expected_revision,
                server_id,
                response,
            },
            "Language server configuration queue is unavailable",
        )?;
        result
            .recv()
            .context("Language server configuration worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    fn try_send(&self, command: AgentSessionCommand, queue_error: &'static str) -> Result<()> {
        if !self.available.load(Ordering::Acquire) {
            return Err(anyhow!(AGENT_UNAVAILABLE_COMMAND_ERROR));
        }
        self.commands.try_send(command).context(queue_error)
    }
}

impl Drop for AgentSession {
    fn drop(&mut self) {
        self.available.store(false, Ordering::Release);
        let _ = self.commands.send(AgentSessionCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_agent_session(
    event_proxy: AppProxy<NativeEvent>,
    commands: Receiver<AgentSessionCommand>,
    target: AgentSessionTarget,
    available: Arc<AtomicBool>,
) {
    let result = run_agent_session_inner(&event_proxy, &commands, &target, &available);
    if let Err(error) = result {
        let _ = send_event(&event_proxy, AgentSessionEvent::Error(error.to_string()));
    }
    let _ = send_event(&event_proxy, AgentSessionEvent::Closed);
}

fn run_agent_session_inner(
    event_proxy: &AppProxy<NativeEvent>,
    commands: &Receiver<AgentSessionCommand>,
    target: &AgentSessionTarget,
    available: &AtomicBool,
) -> Result<()> {
    if target.is_remote() {
        return remote::run_with_recovery(event_proxy, commands, target, available);
    }
    let mut target = target.clone();
    let mut preferred_session_id = None;
    loop {
        match run_agent_session_connection(
            event_proxy,
            commands,
            &target,
            preferred_session_id.as_ref(),
            available,
        ) {
            Ok(()) => return Ok(()),
            Err(failure) => {
                let Some(reconnect) = failure.error.downcast_ref::<AgentSessionReconnect>() else {
                    return Err(failure.error);
                };
                target = AgentSessionTarget::local(reconnect.root.clone());
                preferred_session_id = reconnect.preferred_session_id.clone();
            }
        }
    }
}

fn run_agent_session_connection(
    event_proxy: &AppProxy<NativeEvent>,
    commands: &Receiver<AgentSessionCommand>,
    target: &AgentSessionTarget,
    preferred_session_id: Option<&SessionId>,
    available: &AtomicBool,
) -> std::result::Result<(), AgentSessionFailure> {
    available.store(false, Ordering::Release);
    let workspace_root = target.workspace_root();
    let mut session = target.start().map_err(AgentSessionFailure::connection)?;
    let mut client = session.client();
    let slash_commands = client
        .initialization()
        .map_err(|error| AgentSessionFailure::connection(anyhow!(error.to_string())))?
        .slash_commands
        .clone();
    let models = client
        .list_models()
        .map_err(|error| AgentSessionFailure::connection(anyhow!(error.to_string())))?
        .models;
    send_event(
        event_proxy,
        AgentSessionEvent::Catalog {
            slash_commands,
            models,
        },
    )
    .map_err(AgentSessionFailure::fatal)?;
    publish_configuration(event_proxy, &mut client).map_err(AgentSessionFailure::connection)?;
    publish_git_projection(event_proxy, &mut client).map_err(AgentSessionFailure::connection)?;
    let events = session
        .take_events()
        .map_err(|error| AgentSessionFailure::connection(anyhow!(error.to_string())))?;
    let (sessions, mut active) =
        ensure_active_session(&mut client, workspace_root, preferred_session_id)
            .map_err(AgentSessionFailure::connection)?;
    send_event(event_proxy, AgentSessionEvent::SessionCatalog(sessions))
        .map_err(AgentSessionFailure::fatal)?;
    publish_subscription(event_proxy, &active.subscription, &active.thread_id, None)
        .map_err(AgentSessionFailure::fatal)?;
    available.store(true, Ordering::Release);

    let loop_result = drive_agent_session(
        event_proxy,
        commands,
        &events,
        &mut client,
        &mut active,
        workspace_root,
        target,
    );
    available.store(false, Ordering::Release);
    match loop_result {
        Ok(()) => {
            let _ = session.shutdown();
            Ok(())
        }
        Err(error) => {
            let _ = session.shutdown();
            if error.downcast_ref::<AgentSessionConnectionLost>().is_some() {
                Err(AgentSessionFailure::disconnected(error))
            } else {
                Err(AgentSessionFailure::fatal(error))
            }
        }
    }
}

fn drive_agent_session(
    event_proxy: &AppProxy<NativeEvent>,
    commands: &Receiver<AgentSessionCommand>,
    events: &AppServerEvents,
    client: &mut AppServerRequestHandle,
    active: &mut ActiveSession,
    workspace_root: &Path,
    target: &AgentSessionTarget,
) -> Result<()> {
    loop {
        loop {
            match commands.try_recv() {
                Ok(AgentSessionCommand::CreateSession) => {
                    session_switch_trace::event(
                        None,
                        "worker-create-session-start",
                        format_args!("workspace={}", workspace_root.display()),
                    );
                    match create_active_session(client, workspace_root) {
                        Ok(next) => {
                            session_switch_trace::event(
                                None,
                                "worker-session-created",
                                format_args!(
                                    "session_id={} thread_id={}",
                                    next.session_id, next.thread_id
                                ),
                            );
                            let previous_session_id = active.session_id.clone();
                            if let Err(error) =
                                client.unsubscribe_session(SessionUnsubscribeParams {
                                    session_id: previous_session_id,
                                })
                            {
                                send_event(
                                    event_proxy,
                                    AgentSessionEvent::Error(error.to_string()),
                                )?;
                            }
                            *active = next;
                            publish_subscription(
                                event_proxy,
                                &active.subscription,
                                &active.thread_id,
                                None,
                            )?;
                        }
                        Err(error) => {
                            send_event(event_proxy, AgentSessionEvent::Error(error.to_string()))?;
                        }
                    }
                }
                Ok(AgentSessionCommand::ActivateSession {
                    session_id,
                    switch_id,
                    response,
                }) => {
                    let _trace = session_switch_trace::Span::new(
                        Some(switch_id),
                        "app-server-activate-session",
                    );
                    session_switch_trace::event(
                        Some(switch_id),
                        "worker-activation-start",
                        format_args!("session_id={session_id}"),
                    );
                    match resolve_session_activation(client, session_id, workspace_root) {
                        Ok(SessionActivation::Current(next)) => {
                            let previous_session_id = active.session_id.clone();
                            if let Err(error) =
                                client.unsubscribe_session(SessionUnsubscribeParams {
                                    session_id: previous_session_id,
                                })
                            {
                                send_event(
                                    event_proxy,
                                    AgentSessionEvent::Error(error.to_string()),
                                )?;
                            }
                            *active = next;
                            publish_subscription(
                                event_proxy,
                                &active.subscription,
                                &active.thread_id,
                                Some(switch_id),
                            )?;
                            let _ = response.send(Ok(None));
                        }
                        Ok(SessionActivation::Reconnect {
                            root,
                            preferred_session_id,
                        }) => {
                            let projection = match prepare_workspace_reconnect(target, root.clone())
                            {
                                Ok(projection) => projection,
                                Err(error) => {
                                    let message = error.to_string();
                                    let _ = response.send(Err(message.clone()));
                                    send_event(event_proxy, AgentSessionEvent::Error(message))?;
                                    continue;
                                }
                            };
                            let _ = response.send(Ok(Some(projection)));
                            return Err(anyhow!(AgentSessionReconnect {
                                root,
                                preferred_session_id: Some(preferred_session_id),
                            }));
                        }
                        Err(error) => {
                            session_switch_trace::event(
                                Some(switch_id),
                                "worker-activation-error",
                                format_args!("error={error}"),
                            );
                            let _ = response.send(Err(error.to_string()));
                            send_event(event_proxy, AgentSessionEvent::Error(error.to_string()))?;
                        }
                    }
                }
                Ok(AgentSessionCommand::SubmitAgentMessage(text)) => {
                    submit_agent_message(client, active, text)?;
                }
                Ok(AgentSessionCommand::SubmitShellCommand(command)) => {
                    submit_shell_command(client, active, command)?;
                }
                Ok(AgentSessionCommand::SelectModel(model)) => {
                    select_model(client, active, model)?;
                }
                Ok(AgentSessionCommand::Refresh) => {
                    active.subscription =
                        subscribe_session(client, &active.session_id, active.session_sequence)?;
                    active.session_sequence = active.subscription.session.sequence;
                    active.sequence =
                        active_thread_projection(&active.subscription, &active.thread_id)?
                            .thread
                            .sequence;
                    publish_subscription(
                        event_proxy,
                        &active.subscription,
                        &active.thread_id,
                        None,
                    )?;
                }
                Ok(AgentSessionCommand::RefreshGit) => {
                    if let Err(error) = publish_git_projection(event_proxy, client) {
                        send_event(event_proxy, AgentSessionEvent::Error(error.to_string()))?;
                    }
                }
                Ok(AgentSessionCommand::ReadDirectory { path, response }) => {
                    let result = client
                        .read_directory(FsReadDirectoryParams {
                            workspace_folder_id: None,
                            path,
                        })
                        .map(|result| result.entries)
                        .map_err(|error| error.to_string());
                    let _ = response.send(result);
                }
                Ok(AgentSessionCommand::ReadFile { path, response }) => {
                    let result = read_file(client, path).map_err(|error| error.to_string());
                    let _ = response.send(result);
                }
                Ok(AgentSessionCommand::WriteFile { request, response }) => {
                    let result = write_file(client, request).map_err(|error| error.to_string());
                    let _ = response.send(result);
                }
                Ok(AgentSessionCommand::ListGitBranches(response)) => {
                    let result = client
                        .list_git_branches()
                        .map(|result| result.branches)
                        .map_err(|error| error.to_string());
                    let _ = response.send(result);
                }
                Ok(AgentSessionCommand::SwitchGitBranch { name, response }) => {
                    let result = switch_git_branch(client, name);
                    let _ = response.send(result.map_err(|error| error.to_string()));
                }
                Ok(AgentSessionCommand::SwitchWorkspace { root, response }) => {
                    let result = prepare_workspace_reconnect(target, root.clone());
                    match result {
                        Ok(projection) => {
                            let root = projection.root.clone();
                            let _ = response.send(Ok(projection));
                            return Err(anyhow!(AgentSessionReconnect {
                                root,
                                preferred_session_id: None,
                            }));
                        }
                        Err(error) => {
                            let _ = response.send(Err(error.to_string()));
                        }
                    }
                }
                Ok(AgentSessionCommand::ConfigureLanguageServer {
                    expected_revision,
                    server_id,
                    config,
                    response,
                }) => {
                    let result = client
                        .configure_language_server(LanguageServerConfigureParams {
                            command_id: next_command_id("language-server-configure"),
                            expected_revision,
                            server_id,
                            config,
                        })
                        .map_err(|error| error.to_string());
                    let _ = response.send(result);
                }
                Ok(AgentSessionCommand::RemoveLanguageServerConfiguration {
                    expected_revision,
                    server_id,
                    response,
                }) => {
                    let result = client
                        .remove_language_server_configuration(LanguageServerRemoveParams {
                            command_id: next_command_id("language-server-remove"),
                            expected_revision,
                            server_id,
                        })
                        .map_err(|error| error.to_string());
                    let _ = response.send(result);
                }
                Ok(AgentSessionCommand::Shutdown) => return Ok(()),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Ok(()),
            }
        }

        match events.recv_timeout(EVENT_POLL_INTERVAL) {
            Ok(AppServerEvent::Notification(ServerNotification::SessionUpdate(update))) => {
                if update.session_id == active.session_id {
                    active.session_sequence = active.session_sequence.max(update.durable_sequence);
                }
            }
            Ok(AppServerEvent::Notification(ServerNotification::SessionThreadUpdate(update))) => {
                if update.thread_id == active.thread_id {
                    active.sequence = active.sequence.max(update.durable_sequence);
                    send_event(event_proxy, AgentSessionEvent::Update(update))?;
                }
            }
            Ok(AppServerEvent::Notification(ServerNotification::GitStatusChanged(_))) => {
                publish_git_projection(event_proxy, client)?;
            }
            Ok(AppServerEvent::Notification(ServerNotification::FsChanged(changed))) => {
                send_event(event_proxy, AgentSessionEvent::FilesChanged(changed))?;
            }
            Ok(AppServerEvent::Notification(ServerNotification::ConfigChanged(_))) => {
                publish_configuration(event_proxy, client)?;
            }
            Ok(AppServerEvent::Notification(_)) => {}
            Ok(AppServerEvent::ConnectionClosed(reason)) => {
                return Err(AgentSessionConnectionLost(format!(
                    "App Server connection closed: {reason:?}"
                ))
                .into());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(AgentSessionConnectionLost(
                    "App Server event stream disconnected".into(),
                )
                .into());
            }
        }
    }
}

fn read_file(client: &mut AppServerRequestHandle, path: PathBuf) -> Result<TextFileSnapshot> {
    for _ in 0..FILE_SNAPSHOT_READ_ATTEMPTS {
        let before = client
            .get_file_metadata(FsGetMetadataParams {
                workspace_folder_id: None,
                path: path.clone(),
            })
            .map(disk_version)
            .map_err(client_error)?;
        let content = client
            .read_file(FsReadFileParams {
                workspace_folder_id: None,
                path: path.clone(),
            })
            .map_err(client_error)?
            .content;
        let after = client
            .get_file_metadata(FsGetMetadataParams {
                workspace_folder_id: None,
                path: path.clone(),
            })
            .map(disk_version)
            .map_err(client_error)?;
        if before == after {
            return Ok(TextFileSnapshot::new(path, content, after));
        }
    }
    Err(anyhow!(
        "{} kept changing while it was being read",
        path.display()
    ))
}

fn write_file(
    client: &mut AppServerRequestHandle,
    request: TextFileSaveRequest,
) -> Result<TextFileDiskVersion> {
    let (path, content, expected_version) = request.into_parts();
    let current = client
        .get_file_metadata(FsGetMetadataParams {
            workspace_folder_id: None,
            path: path.clone(),
        })
        .map_err(client_error)?;
    let current = disk_version(current);
    if current != expected_version {
        return Err(anyhow!(
            "{} changed on disk since it was opened",
            path.display()
        ));
    }
    if current.is_read_only() {
        return Err(anyhow!("{} is read-only", path.display()));
    }
    client
        .write_file(FsWriteFileParams {
            workspace_folder_id: None,
            path,
            content,
            expected_revision: None,
        })
        .map(|result| disk_version(result.metadata))
        .map_err(client_error)
}

fn disk_version(metadata: FsGetMetadataResult) -> TextFileDiskVersion {
    let access = if metadata.readonly {
        TextFileAccess::ReadOnly
    } else {
        TextFileAccess::Writable
    };
    TextFileDiskVersion::new(
        metadata.size_bytes,
        TextFileModifiedAt::from(metadata.modified_at_millis),
        access,
    )
}

fn client_error(error: ClientError) -> anyhow::Error {
    match error {
        ClientError::Transport(message) => AgentSessionConnectionLost(message).into(),
        error => anyhow!(error.to_string()),
    }
}

struct ActiveSession {
    session_id: SessionId,
    session_sequence: u64,
    thread_id: ThreadId,
    sequence: u64,
    subscription: SessionSubscribeResult,
}

fn ensure_active_session(
    client: &mut AppServerRequestHandle,
    workspace_root: &Path,
    preferred_session_id: Option<&SessionId>,
) -> Result<(Vec<Session>, ActiveSession)> {
    let mut sessions = client.list_sessions().map_err(client_error)?.sessions;
    sessions.sort_by(|left, right| {
        left.title
            .cmp(&right.title)
            .then_with(|| left.session_id.as_str().cmp(right.session_id.as_str()))
    });
    session_switch_trace::event(
        None,
        "session-catalog",
        format_args!(
            "total={} active={}",
            sessions.len(),
            sessions
                .iter()
                .filter(|session| session.status == SessionStatus::Active)
                .count()
        ),
    );
    let session = match preferred_session_id
        .and_then(|preferred| {
            sessions.iter().find(|session| {
                &session.session_id == preferred
                    && session.status == SessionStatus::Active
                    && session_is_current_workspace(session, workspace_root)
            })
        })
        .or_else(|| {
            sessions.iter().find(|session| {
                session.status == SessionStatus::Active
                    && session_is_current_workspace(session, workspace_root)
            })
        })
        .cloned()
    {
        Some(session) => session,
        None => {
            let session = create_session(client, workspace_root)?;
            sessions.push(session.clone());
            session
        }
    };
    let active = initialize_session(client, session, workspace_root)?;
    sessions.retain(|session| session.status == SessionStatus::Active);
    Ok((sessions, active))
}

fn create_active_session(
    client: &mut AppServerRequestHandle,
    workspace_root: &Path,
) -> Result<ActiveSession> {
    let session = create_session(client, workspace_root)?;
    initialize_session(client, session, workspace_root)
}

enum SessionActivation {
    Current(ActiveSession),
    Reconnect {
        root: PathBuf,
        preferred_session_id: SessionId,
    },
}

fn resolve_session_activation(
    client: &mut AppServerRequestHandle,
    session_id: SessionId,
    workspace_root: &Path,
) -> Result<SessionActivation> {
    let session = client
        .read_session(SessionReadParams { session_id })
        .map_err(client_error)?
        .session;
    if session.status != SessionStatus::Active {
        return Err(anyhow!("cannot activate a non-active session"));
    }
    match route_session_for_target(&session, workspace_root)? {
        SessionWorkspaceRoute::Current => {
            initialize_session(client, session, workspace_root).map(SessionActivation::Current)
        }
        SessionWorkspaceRoute::Reconnect(binding) => Ok(SessionActivation::Reconnect {
            root: binding.root,
            preferred_session_id: session.session_id,
        }),
        SessionWorkspaceRoute::LegacyUnbound => Err(anyhow!(
            "cannot activate legacy Session {} because it has no Workspace binding",
            session.session_id,
        )),
    }
}

fn session_is_current_workspace(session: &Session, workspace_root: &Path) -> bool {
    matches!(
        route_session_for_target(session, workspace_root),
        Ok(SessionWorkspaceRoute::Current)
    )
}

fn route_session_for_target(
    session: &Session,
    workspace_root: &Path,
) -> Result<SessionWorkspaceRoute> {
    if session
        .workspace
        .as_ref()
        .is_some_and(|binding| binding.root() == workspace_root)
    {
        return Ok(SessionWorkspaceRoute::Current);
    }
    if workspace_root.exists() {
        return route_session_workspace(session, workspace_root).map_err(anyhow::Error::from);
    }
    Ok(match session.workspace.as_ref() {
        Some(binding) => SessionWorkspaceRoute::Reconnect(binding.clone()),
        None => SessionWorkspaceRoute::LegacyUnbound,
    })
}

fn initialize_session(
    client: &mut AppServerRequestHandle,
    session: Session,
    workspace_root: &Path,
) -> Result<ActiveSession> {
    let (session, thread_id) = ensure_session_thread(client, session, workspace_root)?;
    let subscription = subscribe_session(client, &session.session_id, 0)?;
    let thread = active_thread_projection(&subscription, &thread_id)?;
    Ok(ActiveSession {
        session_id: subscription.session.session_id.clone(),
        session_sequence: subscription.session.sequence,
        thread_id,
        sequence: thread.thread.sequence,
        subscription,
    })
}

fn create_session(client: &mut AppServerRequestHandle, workspace_root: &Path) -> Result<Session> {
    client
        .create_session(SessionCreateParams {
            command_id: next_command_id("session"),
            title: workspace_title(workspace_root),
        })
        .map(|result| result.session)
        .map_err(client_error)
}

fn ensure_session_thread(
    client: &mut AppServerRequestHandle,
    session: Session,
    workspace_root: &Path,
) -> Result<(Session, ThreadId)> {
    if let Some(thread) = session
        .threads
        .iter()
        .find(|thread| thread.status == SessionThreadStatus::Active)
    {
        return Ok((session.clone(), thread.thread_id.clone()));
    }
    let result = client
        .request_session(SessionRequestParams {
            command_id: next_command_id("thread"),
            session_id: session.session_id,
            expected_sequence: session.sequence,
            request: SessionRequest::CreateThread {
                title: workspace_title(workspace_root),
            },
        })
        .map_err(client_error)?;
    let SessionRequestResult::Thread(result) = result else {
        return Err(anyhow!(
            "Session request returned an unexpected Thread result"
        ));
    };
    Ok((result.session, result.thread_id))
}

fn subscribe_session(
    client: &mut AppServerRequestHandle,
    session_id: &SessionId,
    after_sequence: u64,
) -> Result<SessionSubscribeResult> {
    client
        .subscribe_session(SessionSubscribeParams {
            session_id: session_id.clone(),
            after_sequence,
        })
        .map_err(client_error)
}

fn active_thread_projection<'a>(
    subscription: &'a SessionSubscribeResult,
    thread_id: &ThreadId,
) -> Result<&'a SessionThreadProjection> {
    subscription
        .thread_projections
        .iter()
        .find(|projection| &projection.thread.thread_id == thread_id)
        .ok_or_else(|| anyhow!("Session subscription did not include active Thread"))
}

fn publish_subscription(
    event_proxy: &AppProxy<NativeEvent>,
    subscription: &SessionSubscribeResult,
    thread_id: &ThreadId,
    switch_id: Option<SwitchId>,
) -> Result<()> {
    send_event(
        event_proxy,
        snapshot_event_from_subscription(subscription, thread_id, switch_id)?,
    )
}

fn snapshot_event_from_subscription(
    subscription: &SessionSubscribeResult,
    thread_id: &ThreadId,
    switch_id: Option<SwitchId>,
) -> Result<AgentSessionEvent> {
    let thread = active_thread_projection(subscription, thread_id)?;
    session_switch_trace::event(
        switch_id,
        "subscription-ready",
        format_args!(
            "session_id={} thread_id={} updates={} thread_sequence={}",
            subscription.session.session_id,
            thread_id,
            thread.updates.len(),
            thread.thread.sequence
        ),
    );
    // `thread.thread` is already the authoritative snapshot at the latest durable sequence.
    // `thread.updates` is replay history from sequence zero, not new streaming state. Forwarding
    // it after the snapshot would make the final committed update request another refresh, which
    // would publish the same snapshot and replay again during Session Tab activation.
    Ok(AgentSessionEvent::Snapshot {
        session: subscription.session.clone(),
        thread: thread.thread.clone(),
        switch_id,
    })
}

fn submit_agent_message(
    client: &mut AppServerRequestHandle,
    active: &mut ActiveSession,
    text: String,
) -> Result<()> {
    if text.trim().is_empty() {
        return Ok(());
    }
    let result = client
        .request_session(SessionRequestParams {
            command_id: next_command_id("turn"),
            session_id: active.session_id.clone(),
            expected_sequence: active.sequence,
            request: SessionRequest::StartTurn {
                thread_id: active.thread_id.clone(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                resource_budget: None,
                input: vec![InputItem::Text { text }],
            },
        })
        .map_err(client_error)?;
    let SessionRequestResult::Turn(result) = result else {
        return Err(anyhow!(
            "Session request returned an unexpected Turn result"
        ));
    };
    active.sequence = result.sequence;
    Ok(())
}

fn submit_shell_command(
    client: &mut AppServerRequestHandle,
    active: &mut ActiveSession,
    command: String,
) -> Result<()> {
    if command.trim().is_empty() {
        return Ok(());
    }
    let result = client
        .request_session(SessionRequestParams {
            command_id: next_command_id("shell-turn"),
            session_id: active.session_id.clone(),
            expected_sequence: active.sequence,
            request: SessionRequest::StartShellTurn {
                thread_id: active.thread_id.clone(),
                approval_mode: zeta_protocol::ApprovalMode::AskPermissions,
                command,
                working_directory: ".".into(),
            },
        })
        .map_err(client_error)?;
    let SessionRequestResult::Turn(result) = result else {
        return Err(anyhow!(
            "Session request returned an unexpected Turn result"
        ));
    };
    active.sequence = result.sequence;
    Ok(())
}

fn select_model(
    client: &mut AppServerRequestHandle,
    active: &mut ActiveSession,
    model: ModelRef,
) -> Result<()> {
    let result = client
        .request_session(SessionRequestParams {
            command_id: next_command_id("model"),
            session_id: active.session_id.clone(),
            expected_sequence: active.session_sequence,
            request: SessionRequest::SetModel { model },
        })
        .map_err(client_error)?;
    let SessionRequestResult::Session(result) = result else {
        return Err(anyhow!(
            "Session request returned an unexpected Session result"
        ));
    };
    active.session_sequence = result.session.sequence;
    Ok(())
}

fn switch_git_branch(
    client: &mut AppServerRequestHandle,
    name: String,
) -> Result<GitTextDiffResult> {
    client
        .switch_git_branch(GitBranchSwitchParams {
            repository_id: None,
            name,
        })
        .map_err(client_error)?;
    read_git_projection(client)?.ok_or_else(|| anyhow!("Git repository became unavailable"))
}

fn prepare_workspace_reconnect(
    target: &AgentSessionTarget,
    root: PathBuf,
) -> Result<WorkspaceSwitchProjection> {
    let session = target.with_workspace_root(&root)?.start()?;
    let mut client = session.client();
    let git = read_git_projection(&mut client);
    let shutdown = session
        .shutdown()
        .map_err(|error| anyhow!(error.to_string()));
    let projection = WorkspaceSwitchProjection { root, git: git? };
    shutdown?;
    Ok(projection)
}

fn publish_git_projection(
    event_proxy: &AppProxy<NativeEvent>,
    client: &mut AppServerRequestHandle,
) -> Result<()> {
    send_event(
        event_proxy,
        AgentSessionEvent::GitProjection(read_git_projection(client)?),
    )
}

fn publish_configuration(
    event_proxy: &AppProxy<NativeEvent>,
    client: &mut AppServerRequestHandle,
) -> Result<()> {
    let configuration = client.read_config().map_err(client_error)?;
    send_event(event_proxy, AgentSessionEvent::Configuration(configuration))
}

fn read_git_projection(client: &mut AppServerRequestHandle) -> Result<Option<GitTextDiffResult>> {
    match client.git_text_diff() {
        Ok(projection) => Ok(Some(projection)),
        Err(error) if git_is_unavailable(&error) => Ok(None),
        Err(error) => Err(client_error(error)),
    }
}

fn git_is_unavailable(error: &ClientError) -> bool {
    matches!(
        error,
        ClientError::Server {
            code: -32062 | -32060,
            ..
        }
    )
}

fn send_event(event_proxy: &AppProxy<NativeEvent>, event: AgentSessionEvent) -> Result<()> {
    event_proxy
        .send_event(event.into())
        .map_err(|_| anyhow!("native event loop is unavailable"))
}

fn next_command_id(prefix: &str) -> CommandId {
    static NEXT_COMMAND: AtomicU64 = AtomicU64::new(1);
    let sequence = NEXT_COMMAND.fetch_add(1, Ordering::Relaxed);
    CommandId::new(format!("native-{prefix}-{}-{sequence}", std::process::id()))
        .expect("generated native command ID is non-empty")
}

fn workspace_title(workspace_root: &Path) -> String {
    workspace_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Agent Session")
        .to_owned()
}

impl NativeApp {
    pub(crate) fn add_session(&mut self) {
        let Some(session) = self.agent_session.as_ref() else {
            eprintln!("could not create session: App Server session is unavailable");
            return;
        };
        session_switch_trace::event(
            None,
            "session-create-request",
            format_args!("source=add-session"),
        );
        if let Err(error) = session.create_session() {
            eprintln!("could not create session: {error}");
        }
    }

    pub(crate) fn activate_session_tab(&mut self, index: usize) {
        let switch_id = session_switch_trace::SwitchId::next();
        let Some(tab) = self.tab_inputs.session_input_at(index) else {
            session_switch_trace::event(
                Some(switch_id),
                "activation-rejected",
                format_args!("reason=missing-tab index={index}"),
            );
            return;
        };
        let Some(session_id) = tab.session_id().cloned() else {
            session_switch_trace::event(
                Some(switch_id),
                "activation-rejected",
                format_args!("reason=non-session-tab index={index}"),
            );
            return;
        };
        let target_workspace_root = tab.workspace_root().map(Path::to_path_buf);
        session_switch_trace::event(
            Some(switch_id),
            "activation-request",
            format_args!("index={index} session_id={session_id}"),
        );
        if self.tab_inputs.selected_session() == Some(&session_id) {
            if self.tab_inputs.is_settings() {
                self.activate_session_workbench_tab();
                self.rebuild_presentation_on_next_redraw();
            } else {
                session_switch_trace::event(
                    Some(switch_id),
                    "activation-rejected",
                    format_args!("reason=already-selected"),
                );
            }
            return;
        }
        let switches_workspace = target_workspace_root
            .as_deref()
            .is_some_and(|target| target != self.workspace_context.working_directory());
        if switches_workspace
            && self.file_editor_host.request_workspace_replace()
                == crate::file_editor_host::FileEditorCloseRequest::NeedsConfirmation
        {
            session_switch_trace::event(
                Some(switch_id),
                "activation-rejected",
                format_args!("reason=unsaved-workspace-file"),
            );
            eprintln!("could not open Session Workspace while the active file has unsaved changes");
            return;
        }
        let ensured = {
            let _trace = session_switch_trace::Span::new(Some(switch_id), "ensure-terminal");
            self.ensure_terminal_for_session(&session_id)
        };
        if !ensured {
            session_switch_trace::event(
                Some(switch_id),
                "activation-rejected",
                format_args!("reason=terminal-ensure-failed"),
            );
            return;
        }
        let Some(session) = self.agent_session.as_ref() else {
            session_switch_trace::event(
                Some(switch_id),
                "activation-rejected",
                format_args!("reason=agent-session-unavailable"),
            );
            return;
        };
        let workspace_switch = match session.activate_session(session_id.clone(), switch_id) {
            Ok(workspace_switch) => workspace_switch,
            Err(error) => {
                session_switch_trace::event(
                    Some(switch_id),
                    "activation-rejected",
                    format_args!("reason=agent-command-queue error={error}"),
                );
                eprintln!("could not activate session: {error}");
                return;
            }
        };
        if let Some(projection) = workspace_switch
            && !self.apply_workspace_switch_projection(projection)
        {
            return;
        }
        self.tab_inputs.activate_session(&session_id);
        self.activate_session_workbench_tab();
        let terminal_activated = self.activate_terminal_for_session(&session_id);
        session_switch_trace::event(
            Some(switch_id),
            "local-terminal-activation",
            format_args!("success={terminal_activated}"),
        );
        {
            let _trace = session_switch_trace::Span::new(Some(switch_id), "local-ui-invalidation");
            self.rebuild_presentation_on_next_redraw();
        }
        session_switch_trace::event(
            Some(switch_id),
            "local-activation-visible",
            format_args!("selected_session={session_id}"),
        );
    }

    fn upsert_session_tab(&mut self, session: &Session) {
        let workspace = self.workspace_context.working_directory_label().to_owned();
        let result = self.tab_inputs.upsert_session(session, &workspace);
        let (label, input_key) = match result {
            TabInputChange::Added(input_key) => ("session-tab-added", input_key),
            TabInputChange::Updated(input_key) => ("session-tab-updated", input_key),
        };
        session_switch_trace::event(
            None,
            label,
            format_args!(
                "session_id={} input={input_key:?} tab_count={}",
                session.session_id,
                self.tab_inputs.session_count()
            ),
        );
    }

    fn upsert_session_catalog(&mut self, sessions: &[Session]) {
        let workspace = self.workspace_context.working_directory_label().to_owned();
        for session in sessions {
            self.tab_inputs.upsert_catalog_session(session, &workspace);
        }
    }

    pub(crate) fn handle_agent_session_event(&mut self, event: AgentSessionEvent) {
        let previous_line_count = crate::thread_timeline::line_count(&self.thread_projection);
        let workspace_may_have_changed = matches!(
            &event,
            AgentSessionEvent::Update(update)
                if matches!(
                    &update.update,
                    zeta_protocol::ThreadUpdate::Committed {
                        event: zeta_protocol::ThreadEvent::ItemCompleted {
                            item: zeta_protocol::ThreadItem::ToolResult { .. },
                            ..
                        }
                    }
                )
        );
        match event {
            AgentSessionEvent::Catalog {
                slash_commands,
                models,
            } => {
                if let Err(error) = self
                    .composer
                    .interaction_mut()
                    .set_catalog(slash_commands, composer_model_options(models))
                {
                    eprintln!("could not install Slash Commands catalog: {error}");
                }
            }
            AgentSessionEvent::Configuration(configuration) => {
                self.language_server_settings.synchronize(&configuration);
                self.language_service
                    .apply_configuration(&configuration, &self.file_editor_host);
            }
            AgentSessionEvent::SessionCatalog(sessions) => {
                self.upsert_session_catalog(&sessions);
            }
            AgentSessionEvent::Snapshot {
                session,
                thread,
                switch_id,
            } => {
                session_switch_trace::event(
                    switch_id,
                    "snapshot-received",
                    format_args!(
                        "session_id={} thread_id={}",
                        session.session_id, thread.thread_id
                    ),
                );
                self.upsert_session_tab(&session);
                self.ensure_terminal_for_session(&session.session_id);
                self.activate_terminal_for_session(&session.session_id);
                synchronize_composer_classifier(&mut self.composer, &thread);
                self.thread_projection.replace_snapshot(thread);
            }
            AgentSessionEvent::Update(update) => {
                update_composer_classifier(&mut self.composer, &update.update);
                if self.thread_projection.apply_update(*update)
                    == ThreadProjectionUpdate::ResubscribeRequired
                    && let Some(session) = self.agent_session.as_ref()
                    && let Err(error) = session.refresh()
                {
                    eprintln!("could not refresh Agent Thread projection: {error}");
                }
            }
            AgentSessionEvent::GitProjection(projection) => {
                self.workspace_context
                    .apply_git_projection(projection.as_ref());
                self.sync_sidebar_pane_repository();
                self.refresh_files_from_app_server();
            }
            AgentSessionEvent::FilesChanged(changed) => {
                if shell_completion_sources_changed(&changed) {
                    self.composer.refresh_shell_workspace();
                }
                self.refresh_files_from_app_server();
                self.refresh_open_files_from_app_server(&changed);
            }
            AgentSessionEvent::Error(error) => {
                eprintln!("Agent session failed: {error}");
            }
            AgentSessionEvent::Closed => {}
        }
        if workspace_may_have_changed {
            if let Some(session) = self.agent_session.as_ref()
                && let Err(error) = session.refresh_git()
            {
                eprintln!("could not refresh Git projection: {error}");
            }
        }
        let line_count = crate::thread_timeline::line_count(&self.thread_projection);
        let limit = self.thread_timeline_scroll_limit();
        self.thread_timeline_scroll
            .preserve_view_after_growth(line_count.saturating_sub(previous_line_count), limit);
        self.thread_timeline_scroll.clamp(limit);
        self.rebuild_presentation_on_next_redraw();
    }
}

fn shell_completion_sources_changed(changed: &FsChanged) -> bool {
    match changed {
        FsChanged::RescanRequired { .. } => true,
        FsChanged::PathsChanged { paths, .. } => paths.iter().any(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    matches!(
                        name,
                        "package.json"
                            | "Justfile"
                            | "justfile"
                            | ".justfile"
                            | "Makefile"
                            | "makefile"
                            | "GNUmakefile"
                    )
                })
        }),
    }
}

impl NativeApp {
    pub(crate) fn replace_sidebar_pane_workspace(&mut self) {
        let sidebar_kind = self.pane_host.kind(&(
            crate::pane_host::PaneHostScope::Sidebar,
            self.sidebar_pane_group.root_pane(),
        ));
        let removed = self
            .sidebar_pane_workspace
            .replace_workspace(&self.workspace_context);
        let view = match sidebar_kind {
            Some(crate::pane_input::PaneInputKind::Diff) => AgentSidebarView::Changes,
            _ => AgentSidebarView::Files,
        };
        self.select_sidebar_pane_view(view);
        self.remove_scm_animation_tracks(removed);
    }

    fn sync_sidebar_pane_repository(&mut self) {
        let removed = self
            .sidebar_pane_workspace
            .sync_repository(&self.workspace_context);
        self.remove_scm_animation_tracks(removed);
    }

    fn remove_scm_animation_tracks(
        &mut self,
        removed: Vec<zeta_editor::MultiDiffEditorItemIdentity>,
    ) {
        for identity in removed {
            self.retained_runtime
                .animation_registry_mut()
                .remove_element(identity.section_id());
        }
    }

    pub(crate) fn refresh_files_from_app_server(&mut self) {
        let Some(session) = self.agent_session.as_ref() else {
            return;
        };
        match session.read_directory(PathBuf::from(".")) {
            Ok(entries) => self.sidebar_pane_workspace.refresh_files(entries),
            Err(error) => eprintln!("could not read App Server workspace directory: {error}"),
        }
    }

    pub(crate) fn load_file_tree_directory(&mut self, element: zui::ui::ElementId, path: PathBuf) {
        let Some(session) = self.agent_session.as_ref() else {
            return;
        };
        match session.read_directory(path) {
            Ok(entries) => {
                self.sidebar_pane_workspace
                    .complete_file_tree_directory_load(element, entries);
            }
            Err(error) => eprintln!("could not read App Server workspace directory: {error}"),
        }
    }

    pub(crate) fn open_workspace_file(&mut self, path: PathBuf) {
        let Some(session) = self.agent_session.as_ref() else {
            return;
        };
        match session.read_file(path) {
            Ok(snapshot) => {
                self.file_editor_host.open(snapshot);
                self.language_service
                    .synchronize_active(&self.file_editor_host);
                self.file_editor_input.reset_for_document_change();
                self.sidebar_part.expand();
                self.workspace_surface.show_editor();
                self.pending_focus = Some(crate::shell_interaction::FILE_EDITOR_DOCUMENT);
                self.rebuild_presentation();
                self.request_redraw();
            }
            Err(error) => eprintln!("could not open App Server workspace file: {error}"),
        }
    }

    pub(crate) fn open_language_definition(
        &mut self,
        target: zeta_language_service::LanguageLocationTarget,
    ) {
        let Some(session) = self.agent_session.as_ref() else {
            return;
        };
        match session.read_file(target.path) {
            Ok(snapshot) => {
                self.file_editor_host.open(snapshot);
                if let Some(position) = definition_editor_position(
                    self.file_editor_host
                        .active()
                        .map(|tab| tab.document().text())
                        .unwrap_or_default(),
                    target.selection_range.start.row,
                    target.selection_range.start.character,
                    target.encoding,
                ) {
                    self.file_editor_host
                        .move_active_caret(position, zeta_editor::CodeEditorSelectionMode::Move);
                }
                self.language_service
                    .synchronize_active(&self.file_editor_host);
                self.file_editor_input.reset_for_document_change();
                self.sidebar_part.expand();
                self.workspace_surface.show_editor();
                self.pending_focus = Some(crate::shell_interaction::FILE_EDITOR_DOCUMENT);
                self.rebuild_presentation();
                self.request_redraw();
            }
            Err(error) => eprintln!("could not open language definition: {error}"),
        }
    }

    pub(crate) fn save_active_workspace_file(&mut self) {
        let Some(request) = self.file_editor_host.save_request() else {
            return;
        };
        let _ = self.write_active_workspace_file(request);
    }

    pub(crate) fn try_save_active_workspace_file(&mut self) -> bool {
        let Some(request) = self.file_editor_host.save_request() else {
            return false;
        };
        self.write_active_workspace_file(request)
    }

    pub(crate) fn overwrite_active_workspace_file(&mut self) -> bool {
        let Some(request) = self.file_editor_host.overwrite_request() else {
            return false;
        };
        self.write_active_workspace_file(request)
    }

    fn write_active_workspace_file(&mut self, request: TextFileSaveRequest) -> bool {
        let path = request.path().to_owned();
        let Some(session) = self.agent_session.as_ref() else {
            return false;
        };
        let saved = match session.write_file(request) {
            Ok(version) => self.file_editor_host.mark_active_saved(version),
            Err(error) => {
                eprintln!("could not save App Server workspace file: {error}");
                if let Ok(snapshot) = session.read_file(path.clone()) {
                    self.file_editor_host.observe_external(snapshot);
                }
                false
            }
        };
        if saved {
            self.language_service.save(&path);
        }
        self.rebuild_presentation();
        self.request_redraw();
        saved
    }

    fn refresh_open_files_from_app_server(&mut self, changed: &FsChanged) {
        let paths = match changed {
            FsChanged::PathsChanged { paths, .. } => self
                .file_editor_host
                .tabs()
                .iter()
                .filter(|tab| paths.iter().any(|path| path == tab.path()))
                .map(|tab| tab.path().to_path_buf())
                .collect::<Vec<_>>(),
            FsChanged::RescanRequired { .. } => self
                .file_editor_host
                .tabs()
                .iter()
                .map(|tab| tab.path().to_path_buf())
                .collect(),
        };
        let Some(session) = self.agent_session.as_ref() else {
            return;
        };
        for path in paths {
            match session.read_file(path) {
                Ok(snapshot) => {
                    self.file_editor_host.observe_external(snapshot);
                }
                Err(error) => eprintln!("could not refresh open workspace file: {error}"),
            }
        }
    }
}

fn definition_editor_position(
    text: &str,
    row: u32,
    character: u32,
    encoding: zeta_language_service::LanguagePositionEncoding,
) -> Option<zeta_editor::CodeEditorPosition> {
    let row_index = usize::try_from(row).ok()?;
    let line = text.split('\n').nth(row_index)?;
    let line = line.strip_suffix('\r').unwrap_or(line);
    let requested = usize::try_from(character).ok()?;
    let byte_offset = match encoding {
        zeta_language_service::LanguagePositionEncoding::Utf8 => {
            (requested <= line.len() && line.is_char_boundary(requested)).then_some(requested)?
        }
        zeta_language_service::LanguagePositionEncoding::Utf16 => {
            let mut units = 0;
            let mut resolved = None;
            for (offset, scalar) in line.char_indices() {
                if units == requested {
                    resolved = Some(offset);
                    break;
                }
                units += scalar.len_utf16();
                if units > requested {
                    return None;
                }
            }
            resolved.or_else(|| (units == requested).then_some(line.len()))?
        }
    };
    Some(zeta_editor::CodeEditorPosition {
        row_index,
        byte_offset,
    })
}

#[cfg(test)]
#[path = "agent_session_tests.rs"]
mod tests;
