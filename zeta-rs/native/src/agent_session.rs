use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use zeta_app_server_client::{
    AppServerEvent, AppServerEvents, AppServerRequestHandle, AppServerSession, ClientError,
    InProcessClientOptions, ServerNotification, local_profile_root,
};
use zeta_app_server_protocol::protocol::common::{ClientInfo, CommandId};
use zeta_app_server_protocol::protocol::fs::{FsReadDirectoryEntry, FsReadDirectoryParams};
use zeta_app_server_protocol::protocol::git::{
    GitBranchDto, GitBranchSwitchParams, GitTextDiffResult,
};
use zeta_app_server_protocol::protocol::model::ModelCatalogEntry;
use zeta_app_server_protocol::protocol::session::{
    SessionCreateParams, SessionModelSetParams, SessionThreadCreateParams,
};
use zeta_app_server_protocol::protocol::slash_commands::SlashCommandDefinition;
use zeta_app_server_protocol::protocol::thread::{ThreadSubscribeParams, ThreadSubscribeResult};
use zeta_app_server_protocol::protocol::turn::{InputItem, ShellTurnStartParams, TurnStartParams};
use zeta_app_server_protocol::protocol::workspace::WorkspaceSwitchParams;
use zeta_protocol::{
    ModelRef, Session, SessionId, SessionStatus, SessionThreadStatus, Thread, ThreadId,
    ThreadUpdateEnvelope,
};
use zeta_winit::EventLoopProxy;

use crate::NativeApp;
use crate::native_event::NativeEvent;
use crate::thread_projection::ThreadProjectionUpdate;

const COMMAND_QUEUE_CAPACITY: usize = 32;
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug)]
pub(crate) enum AgentSessionEvent {
    Catalog {
        slash_commands: Vec<SlashCommandDefinition>,
        models: Vec<ModelCatalogEntry>,
    },
    Snapshot(Thread),
    Update(Box<ThreadUpdateEnvelope>),
    GitProjection(Option<GitTextDiffResult>),
    FilesChanged,
    Error(String),
    Closed,
}

enum AgentSessionCommand {
    SubmitAgentMessage(String),
    SubmitShellCommand(String),
    SelectModel(ModelRef),
    Refresh,
    RefreshGit,
    ReadDirectory {
        path: PathBuf,
        response: SyncSender<std::result::Result<Vec<FsReadDirectoryEntry>, String>>,
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
    Shutdown,
}

pub(crate) struct WorkspaceSwitchProjection {
    pub(crate) root: PathBuf,
    pub(crate) git: Option<GitTextDiffResult>,
}

pub(crate) struct AgentSession {
    commands: SyncSender<AgentSessionCommand>,
    worker: Option<JoinHandle<()>>,
}

impl AgentSession {
    pub(crate) fn spawn(
        event_proxy: EventLoopProxy<NativeEvent>,
        workspace_root: PathBuf,
    ) -> Result<Self> {
        let (commands, command_receiver) = mpsc::sync_channel(COMMAND_QUEUE_CAPACITY);
        let worker = thread::Builder::new()
            .name("zeta-native-agent-session".into())
            .spawn(move || run_agent_session(event_proxy, command_receiver, workspace_root))
            .context("could not start native Agent session worker")?;
        Ok(Self {
            commands,
            worker: Some(worker),
        })
    }

    pub(crate) fn submit_agent_message(&self, text: String) -> Result<()> {
        self.commands
            .try_send(AgentSessionCommand::SubmitAgentMessage(text))
            .context("Agent submission queue is unavailable")
    }

    pub(crate) fn submit_shell_command(&self, command: String) -> Result<()> {
        self.commands
            .try_send(AgentSessionCommand::SubmitShellCommand(command))
            .context("Shell submission queue is unavailable")
    }

    pub(crate) fn select_model(&self, model: ModelRef) -> Result<()> {
        self.commands
            .try_send(AgentSessionCommand::SelectModel(model))
            .context("Agent model selection queue is unavailable")
    }

    pub(crate) fn refresh(&self) -> Result<()> {
        self.commands
            .try_send(AgentSessionCommand::Refresh)
            .context("Agent refresh queue is unavailable")
    }

    pub(crate) fn refresh_git(&self) -> Result<()> {
        self.commands
            .try_send(AgentSessionCommand::RefreshGit)
            .context("Git refresh queue is unavailable")
    }

    pub(crate) fn read_directory(&self, path: PathBuf) -> Result<Vec<FsReadDirectoryEntry>> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .try_send(AgentSessionCommand::ReadDirectory { path, response })
            .context("Workspace directory query queue is unavailable")?;
        result
            .recv()
            .context("Workspace directory query worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    pub(crate) fn local_branches(&self) -> Result<Vec<GitBranchDto>> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .try_send(AgentSessionCommand::ListGitBranches(response))
            .context("Git branch query queue is unavailable")?;
        result
            .recv()
            .context("Git branch query worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    pub(crate) fn switch_git_branch(&self, name: String) -> Result<GitTextDiffResult> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .try_send(AgentSessionCommand::SwitchGitBranch { name, response })
            .context("Git branch mutation queue is unavailable")?;
        result
            .recv()
            .context("Git branch mutation worker stopped")?
            .map_err(anyhow::Error::msg)
    }

    pub(crate) fn switch_workspace(&self, root: PathBuf) -> Result<WorkspaceSwitchProjection> {
        let (response, result) = mpsc::sync_channel(1);
        self.commands
            .try_send(AgentSessionCommand::SwitchWorkspace { root, response })
            .context("Workspace switch queue is unavailable")?;
        result
            .recv()
            .context("Workspace switch worker stopped")?
            .map_err(anyhow::Error::msg)
    }
}

impl Drop for AgentSession {
    fn drop(&mut self) {
        let _ = self.commands.send(AgentSessionCommand::Shutdown);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn run_agent_session(
    event_proxy: EventLoopProxy<NativeEvent>,
    commands: Receiver<AgentSessionCommand>,
    workspace_root: PathBuf,
) {
    let result = run_agent_session_inner(&event_proxy, &commands, &workspace_root);
    if let Err(error) = result {
        let _ = send_event(&event_proxy, AgentSessionEvent::Error(error.to_string()));
    }
    let _ = send_event(&event_proxy, AgentSessionEvent::Closed);
}

fn run_agent_session_inner(
    event_proxy: &EventLoopProxy<NativeEvent>,
    commands: &Receiver<AgentSessionCommand>,
    workspace_root: &Path,
) -> Result<()> {
    let mut session = AppServerSession::start_embedded(
        InProcessClientOptions::new(
            local_profile_root(),
            ClientInfo {
                name: "zeta-native".into(),
                version: env!("CARGO_PKG_VERSION").into(),
            },
        )
        .with_workspace_root(workspace_root),
    )
    .map_err(|error| anyhow!(error.to_string()))?;
    let mut client = session.client();
    let slash_commands = client
        .initialization()
        .map_err(|error| anyhow!(error.to_string()))?
        .slash_commands
        .clone();
    let models = client
        .list_models()
        .map_err(|error| anyhow!(error.to_string()))?
        .models;
    send_event(
        event_proxy,
        AgentSessionEvent::Catalog {
            slash_commands,
            models,
        },
    )?;
    publish_git_projection(event_proxy, &mut client)?;
    let events = session
        .take_events()
        .map_err(|error| anyhow!(error.to_string()))?;
    let mut active = ensure_active_thread(&mut client, workspace_root)?;
    publish_subscription(event_proxy, &active.subscription)?;

    let loop_result = drive_agent_session(event_proxy, commands, &events, &mut client, &mut active);
    session
        .shutdown()
        .map_err(|error| anyhow!(error.to_string()))?;
    loop_result
}

fn drive_agent_session(
    event_proxy: &EventLoopProxy<NativeEvent>,
    commands: &Receiver<AgentSessionCommand>,
    events: &AppServerEvents,
    client: &mut AppServerRequestHandle,
    active: &mut ActiveThread,
) -> Result<()> {
    loop {
        loop {
            match commands.try_recv() {
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
                    active.subscription = subscribe_thread(client, &active.thread_id)?;
                    publish_subscription(event_proxy, &active.subscription)?;
                }
                Ok(AgentSessionCommand::RefreshGit) => {
                    if let Err(error) = publish_git_projection(event_proxy, client) {
                        send_event(event_proxy, AgentSessionEvent::Error(error.to_string()))?;
                    }
                }
                Ok(AgentSessionCommand::ReadDirectory { path, response }) => {
                    let result = client
                        .read_directory(FsReadDirectoryParams { path })
                        .map(|result| result.entries)
                        .map_err(|error| error.to_string());
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
                    let result = switch_workspace(client, root);
                    let _ = response.send(result.map_err(|error| error.to_string()));
                }
                Ok(AgentSessionCommand::Shutdown) => return Ok(()),
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return Ok(()),
            }
        }

        match events.recv_timeout(EVENT_POLL_INTERVAL) {
            Ok(AppServerEvent::Notification(ServerNotification::ThreadUpdate(update))) => {
                if update.thread_id == active.thread_id {
                    active.sequence = active.sequence.max(update.durable_sequence);
                    send_event(event_proxy, AgentSessionEvent::Update(update))?;
                }
            }
            Ok(AppServerEvent::Notification(ServerNotification::GitStatusChanged(_))) => {
                publish_git_projection(event_proxy, client)?;
            }
            Ok(AppServerEvent::Notification(ServerNotification::FsChanged(_))) => {
                send_event(event_proxy, AgentSessionEvent::FilesChanged)?;
            }
            Ok(AppServerEvent::Notification(_)) => {}
            Ok(AppServerEvent::ConnectionClosed(reason)) => {
                return Err(anyhow!("App Server connection closed: {reason:?}"));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(anyhow!("App Server event stream disconnected"));
            }
        }
    }
}

struct ActiveThread {
    session_id: SessionId,
    session_sequence: u64,
    thread_id: ThreadId,
    sequence: u64,
    subscription: ThreadSubscribeResult,
}

fn ensure_active_thread(
    client: &mut AppServerRequestHandle,
    workspace_root: &Path,
) -> Result<ActiveThread> {
    let sessions = client
        .list_sessions()
        .map_err(|error| anyhow!(error.to_string()))?
        .sessions;
    let session = match sessions
        .into_iter()
        .find(|session| session.status == SessionStatus::Active)
    {
        Some(session) => session,
        None => create_session(client, workspace_root)?,
    };
    let (session, thread_id) = ensure_session_thread(client, session, workspace_root)?;
    let subscription = subscribe_thread(client, &thread_id)?;
    Ok(ActiveThread {
        session_id: session.session_id,
        session_sequence: session.sequence,
        thread_id,
        sequence: subscription.thread.sequence,
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
        .map_err(|error| anyhow!(error.to_string()))
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
    client
        .create_session_thread(SessionThreadCreateParams {
            command_id: next_command_id("thread"),
            session_id: session.session_id,
            expected_sequence: session.sequence,
            title: workspace_title(workspace_root),
        })
        .map(|result| (result.session, result.thread_id))
        .map_err(|error| anyhow!(error.to_string()))
}

fn subscribe_thread(
    client: &mut AppServerRequestHandle,
    thread_id: &ThreadId,
) -> Result<ThreadSubscribeResult> {
    client
        .subscribe_thread(ThreadSubscribeParams {
            thread_id: thread_id.clone(),
            after_sequence: 0,
        })
        .map_err(|error| anyhow!(error.to_string()))
}

fn publish_subscription(
    event_proxy: &EventLoopProxy<NativeEvent>,
    subscription: &ThreadSubscribeResult,
) -> Result<()> {
    send_event(
        event_proxy,
        AgentSessionEvent::Snapshot(subscription.thread.clone()),
    )?;
    for update in &subscription.updates {
        send_event(
            event_proxy,
            AgentSessionEvent::Update(Box::new(update.clone())),
        )?;
    }
    Ok(())
}

fn submit_agent_message(
    client: &mut AppServerRequestHandle,
    active: &mut ActiveThread,
    text: String,
) -> Result<()> {
    if text.trim().is_empty() {
        return Ok(());
    }
    let result = client
        .start_turn(TurnStartParams {
            command_id: next_command_id("turn"),
            session_id: active.session_id.clone(),
            thread_id: active.thread_id.clone(),
            expected_sequence: active.sequence,
            input: vec![InputItem::Text { text }],
        })
        .map_err(|error| anyhow!(error.to_string()))?;
    active.sequence = result.sequence;
    Ok(())
}

fn submit_shell_command(
    client: &mut AppServerRequestHandle,
    active: &mut ActiveThread,
    command: String,
) -> Result<()> {
    if command.trim().is_empty() {
        return Ok(());
    }
    let result = client
        .start_shell_turn(ShellTurnStartParams {
            command_id: next_command_id("shell-turn"),
            session_id: active.session_id.clone(),
            thread_id: active.thread_id.clone(),
            expected_sequence: active.sequence,
            command,
            working_directory: ".".into(),
        })
        .map_err(|error| anyhow!(error.to_string()))?;
    active.sequence = result.sequence;
    Ok(())
}

fn select_model(
    client: &mut AppServerRequestHandle,
    active: &mut ActiveThread,
    model: ModelRef,
) -> Result<()> {
    let result = client
        .set_session_model(SessionModelSetParams {
            command_id: next_command_id("model"),
            session_id: active.session_id.clone(),
            expected_sequence: active.session_sequence,
            model,
        })
        .map_err(|error| anyhow!(error.to_string()))?;
    active.session_sequence = result.session.sequence;
    Ok(())
}

fn switch_git_branch(
    client: &mut AppServerRequestHandle,
    name: String,
) -> Result<GitTextDiffResult> {
    client
        .switch_git_branch(GitBranchSwitchParams { name })
        .map_err(|error| anyhow!(error.to_string()))?;
    read_git_projection(client)?.ok_or_else(|| anyhow!("Git repository became unavailable"))
}

fn switch_workspace(
    client: &mut AppServerRequestHandle,
    root: PathBuf,
) -> Result<WorkspaceSwitchProjection> {
    let switched = client
        .switch_workspace(WorkspaceSwitchParams { root })
        .map_err(|error| anyhow!(error.to_string()))?;
    Ok(WorkspaceSwitchProjection {
        root: switched.root,
        git: read_git_projection(client)?,
    })
}

fn publish_git_projection(
    event_proxy: &EventLoopProxy<NativeEvent>,
    client: &mut AppServerRequestHandle,
) -> Result<()> {
    send_event(
        event_proxy,
        AgentSessionEvent::GitProjection(read_git_projection(client)?),
    )
}

fn read_git_projection(client: &mut AppServerRequestHandle) -> Result<Option<GitTextDiffResult>> {
    match client.git_text_diff() {
        Ok(projection) => Ok(Some(projection)),
        Err(error) if git_is_unavailable(&error) => Ok(None),
        Err(error) => Err(anyhow!(error.to_string())),
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

fn send_event(event_proxy: &EventLoopProxy<NativeEvent>, event: AgentSessionEvent) -> Result<()> {
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
                    .composer_interaction
                    .set_catalog(slash_commands, models)
                {
                    eprintln!("could not install Slash Commands catalog: {error}");
                }
            }
            AgentSessionEvent::Snapshot(thread) => {
                self.thread_projection.replace_snapshot(thread);
            }
            AgentSessionEvent::Update(update) => {
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
                self.agent_sidebar_workspace
                    .sync_repository(&self.workspace_context);
                self.refresh_files_from_app_server();
            }
            AgentSessionEvent::FilesChanged => self.refresh_files_from_app_server(),
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
        self.rebuild_presentation();
        self.request_redraw();
    }
}

impl NativeApp {
    pub(crate) fn refresh_files_from_app_server(&mut self) {
        let Some(session) = self.agent_session.as_ref() else {
            return;
        };
        match session.read_directory(PathBuf::from(".")) {
            Ok(entries) => self.agent_sidebar_workspace.refresh_files(entries),
            Err(error) => eprintln!("could not read App Server workspace directory: {error}"),
        }
    }

    pub(crate) fn load_file_tree_directory(
        &mut self,
        element: zeta_ui_dispatch::ElementId,
        path: PathBuf,
    ) {
        let Some(session) = self.agent_session.as_ref() else {
            return;
        };
        match session.read_directory(path) {
            Ok(entries) => {
                self.agent_sidebar_workspace
                    .complete_file_tree_directory_load(element, entries);
            }
            Err(error) => eprintln!("could not read App Server workspace directory: {error}"),
        }
    }
}

#[cfg(test)]
#[path = "agent_session_tests.rs"]
mod tests;
