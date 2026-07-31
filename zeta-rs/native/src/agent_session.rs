use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender, TryRecvError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use anyhow::{Context, Result, anyhow};
use zeta_app_server_client::{
    AppServerEvent, AppServerEvents, AppServerRequestHandle, AppServerSession,
    InProcessClientOptions, ServerNotification, local_profile_root,
};
use zeta_app_server_protocol::protocol::common::{ClientInfo, CommandId};
use zeta_app_server_protocol::protocol::session::{SessionCreateParams, SessionThreadCreateParams};
use zeta_app_server_protocol::protocol::thread::{ThreadSubscribeParams, ThreadSubscribeResult};
use zeta_app_server_protocol::protocol::turn::{InputItem, ShellTurnStartParams, TurnStartParams};
use zeta_protocol::{
    Session, SessionId, SessionStatus, SessionThreadStatus, Thread, ThreadId, ThreadUpdateEnvelope,
};
use zeta_winit::EventLoopProxy;

use crate::NativeApp;
use crate::native_event::NativeEvent;
use crate::thread_projection::ThreadProjectionUpdate;

const COMMAND_QUEUE_CAPACITY: usize = 32;
const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug)]
pub(crate) enum AgentSessionEvent {
    Snapshot(Thread),
    Update(Box<ThreadUpdateEnvelope>),
    Error(String),
    Closed,
}

enum AgentSessionCommand {
    SubmitAgentMessage(String),
    SubmitShellCommand(String),
    Refresh,
    Shutdown,
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

    pub(crate) fn refresh(&self) -> Result<()> {
        self.commands
            .try_send(AgentSessionCommand::Refresh)
            .context("Agent refresh queue is unavailable")
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
                Ok(AgentSessionCommand::Refresh) => {
                    active.subscription = subscribe_thread(client, &active.thread_id)?;
                    publish_subscription(event_proxy, &active.subscription)?;
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
            AgentSessionEvent::Error(error) => {
                eprintln!("Agent session failed: {error}");
            }
            AgentSessionEvent::Closed => {}
        }
        if workspace_may_have_changed {
            self.workspace_context.refresh_repository();
            self.agent_sidebar_workspace
                .sync_repository(&self.workspace_context);
            self.agent_sidebar_workspace.refresh_files();
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

#[cfg(test)]
#[path = "agent_session_tests.rs"]
mod tests;
