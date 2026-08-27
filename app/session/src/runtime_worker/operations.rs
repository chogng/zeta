use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use anyhow::Result;
use anyhow::anyhow;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::SessionWorkspaceRoute;
use zeta_app_server_client::route_session_workspace;
use zeta_app_server_protocol::protocol::common::CommandId;
use zeta_app_server_protocol::protocol::session::SessionCreateParams;
use zeta_app_server_protocol::protocol::session::SessionReadParams;
use zeta_app_server_protocol::protocol::session::SessionRequest;
use zeta_app_server_protocol::protocol::session::SessionRequestParams;
use zeta_app_server_protocol::protocol::session::SessionRequestResult;
use zeta_app_server_protocol::protocol::session::SessionSubscribeParams;
use zeta_app_server_protocol::protocol::session::SessionSubscribeResult;
use zeta_app_server_protocol::protocol::session::SessionThreadProjection;
use zeta_app_server_protocol::protocol::session::SessionUnsubscribeParams;
use zeta_app_server_protocol::protocol::turn::InputItem;
use zeta_protocol::ModelRef;
use zeta_protocol::Session;
use zeta_protocol::SessionId;
use zeta_protocol::SessionStatus;
use zeta_protocol::SessionThreadStatus;
use zeta_protocol::ThreadId;

use crate::SessionRuntimeEvent;
use crate::SessionRuntimeEventSink;
use crate::SessionRuntimeTarget;
use crate::WorkspaceSwitchResult;

use super::ConnectionLost;

pub(super) fn unsubscribe_active(
    event_sink: &SessionRuntimeEventSink,
    client: &mut AppServerRequestHandle,
    active: &ActiveSession,
) -> Result<()> {
    if let Err(error) = client.unsubscribe_session(SessionUnsubscribeParams {
        session_id: active.session_id.clone(),
    }) {
        send_event(event_sink, SessionRuntimeEvent::Error(error.to_string()))?;
    }
    Ok(())
}

fn client_error(error: ClientError) -> anyhow::Error {
    match error {
        ClientError::Transport(message) => ConnectionLost(message).into(),
        error => anyhow!(error.to_string()),
    }
}

pub(super) struct ActiveSession {
    pub(super) session_id: SessionId,
    pub(super) session_sequence: u64,
    pub(super) thread_id: ThreadId,
    pub(super) sequence: u64,
    pub(super) subscription: SessionSubscribeResult,
}

pub(super) fn ensure_active_session(
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
    let session = preferred_session_id
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
        .cloned();
    let session = match session {
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

pub(super) fn create_active_session(
    client: &mut AppServerRequestHandle,
    workspace_root: &Path,
) -> Result<ActiveSession> {
    let session = create_session(client, workspace_root)?;
    initialize_session(client, session, workspace_root)
}

pub(super) enum SessionActivation {
    Current(ActiveSession),
    Reconnect {
        root: PathBuf,
        preferred_session_id: SessionId,
    },
}

pub(super) fn resolve_session_activation(
    client: &mut AppServerRequestHandle,
    session_id: SessionId,
    workspace_root: &Path,
) -> Result<SessionActivation> {
    let session = client
        .read_session(SessionReadParams { session_id })
        .map_err(client_error)?
        .session;
    if session.status != SessionStatus::Active {
        return Err(anyhow!("cannot activate a non-active Session"));
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
            session.session_id
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
    let thread = active_thread_entry(&subscription, &thread_id)?;
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

pub(super) fn stop_session(
    client: &mut AppServerRequestHandle,
    session_id: &SessionId,
) -> Result<()> {
    let session = client
        .read_session(SessionReadParams {
            session_id: session_id.clone(),
        })
        .map_err(client_error)?
        .session;
    if session.status != SessionStatus::Active {
        return Err(anyhow!("Session is not active: {session_id}"));
    }
    let result = client
        .request_session(SessionRequestParams {
            command_id: next_command_id("stop-session"),
            session_id: session.session_id,
            expected_sequence: session.sequence,
            request: SessionRequest::Stop,
        })
        .map_err(client_error)?;
    if !matches!(result, SessionRequestResult::Session(_)) {
        return Err(anyhow!(
            "Session stop request returned an unexpected result"
        ));
    }
    Ok(())
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

pub(super) fn subscribe_session(
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

pub(super) fn active_thread_entry<'a>(
    subscription: &'a SessionSubscribeResult,
    thread_id: &ThreadId,
) -> Result<&'a SessionThreadProjection> {
    subscription
        .thread_projections
        .iter()
        .find(|entry| &entry.thread.thread_id == thread_id)
        .ok_or_else(|| anyhow!("Session subscription did not include active Thread"))
}

pub(super) fn publish_subscription(
    event_sink: &SessionRuntimeEventSink,
    subscription: &SessionSubscribeResult,
    thread_id: &ThreadId,
) -> Result<()> {
    let thread = active_thread_entry(subscription, thread_id)?;
    send_event(
        event_sink,
        SessionRuntimeEvent::Snapshot {
            session: subscription.session.clone(),
            thread: thread.thread.clone(),
            transcript: thread.transcript.clone(),
        },
    )
}

pub(super) fn submit_agent_message(
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
                tool_mode: None,
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

pub(super) fn submit_shell_command(
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

pub(super) fn select_model(
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

pub(super) fn prepare_workspace_reconnect(
    target: &dyn SessionRuntimeTarget,
    root: PathBuf,
) -> Result<WorkspaceSwitchResult> {
    let target = target.retarget(&root).map_err(anyhow::Error::msg)?;
    let session = target.start().map_err(anyhow::Error::msg)?;
    session
        .shutdown()
        .map_err(|error| anyhow!(error.to_string()))?;
    Ok(WorkspaceSwitchResult { root })
}

pub(super) fn send_event(
    event_sink: &SessionRuntimeEventSink,
    event: SessionRuntimeEvent,
) -> Result<()> {
    event_sink(event).map_err(anyhow::Error::msg)
}

pub(super) fn next_command_id(prefix: &str) -> CommandId {
    static NEXT_COMMAND: AtomicU64 = AtomicU64::new(1);
    let sequence = NEXT_COMMAND.fetch_add(1, Ordering::Relaxed);
    CommandId::new(format!(
        "session-runtime-{prefix}-{}-{sequence}",
        std::process::id()
    ))
    .expect("generated Session runtime command ID is non-empty")
}

fn workspace_title(workspace_root: &Path) -> String {
    workspace_root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("Session")
        .to_owned()
}

#[cfg(test)]
#[path = "operations_tests.rs"]
mod tests;
