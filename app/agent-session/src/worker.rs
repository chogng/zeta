use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;
use std::time::Instant;

use anyhow::Result;
use anyhow::anyhow;
use zeta_app_server_client::AppServerEvent;
use zeta_app_server_client::AppServerEvents;
use zeta_app_server_client::AppServerRequestHandle;
use zeta_app_server_client::ServerNotification;
use zeta_app_server_protocol::protocol::config::LanguageServerConfigureParams;
use zeta_app_server_protocol::protocol::config::LanguageServerRemoveParams;
use zeta_app_server_protocol::protocol::fs::FsReadDirectoryParams;
use zeta_protocol::SessionId;

use crate::AGENT_UNAVAILABLE_COMMAND_ERROR;
use crate::AgentSessionCommand;
use crate::AgentSessionEvent;
use crate::AgentSessionEventSink;
use crate::AgentSessionTarget;
use crate::RECONNECT_WINDOW;
use crate::reconnect_delay_within_window;
use crate::reject_disconnected_command;

const EVENT_POLL_INTERVAL: Duration = Duration::from_millis(25);
mod operations;

use operations::*;

pub(crate) fn run_agent_session(
    event_sink: AgentSessionEventSink,
    commands: Receiver<AgentSessionCommand>,
    target: Box<dyn AgentSessionTarget>,
    available: Arc<AtomicBool>,
) {
    let result = run_with_recovery(&event_sink, &commands, target, &available);
    if let Err(error) = result {
        let _ = send_event(&event_sink, AgentSessionEvent::Error(error.to_string()));
    }
    let _ = send_event(&event_sink, AgentSessionEvent::Closed);
}

struct AgentSessionFailure {
    error: anyhow::Error,
    retryable: bool,
    connection_was_ready: bool,
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
pub(super) struct ConnectionLost(String);

impl std::fmt::Display for ConnectionLost {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ConnectionLost {}

#[derive(Debug)]
struct ReconnectWorkspace {
    root: PathBuf,
    preferred_session_id: Option<SessionId>,
}

impl std::fmt::Display for ReconnectWorkspace {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "reconnect Workspace authority at {}",
            self.root.display()
        )
    }
}

impl std::error::Error for ReconnectWorkspace {}

fn run_with_recovery(
    event_sink: &AgentSessionEventSink,
    commands: &Receiver<AgentSessionCommand>,
    mut target: Box<dyn AgentSessionTarget>,
    available: &AtomicBool,
) -> Result<()> {
    let mut preferred_session_id = None;
    let mut attempts = 0;
    let mut recovery_started = None;
    loop {
        match run_connection(
            event_sink,
            commands,
            target.as_ref(),
            preferred_session_id.as_ref(),
            available,
        ) {
            Ok(()) => return Ok(()),
            Err(failure) if failure.error.downcast_ref::<ReconnectWorkspace>().is_some() => {
                let reconnect = failure
                    .error
                    .downcast_ref::<ReconnectWorkspace>()
                    .expect("guard verified reconnect marker");
                target = target
                    .retarget(&reconnect.root)
                    .map_err(anyhow::Error::msg)?;
                preferred_session_id = reconnect.preferred_session_id.clone();
                attempts = 0;
                recovery_started = None;
            }
            Err(failure) if !target.is_remote() || !failure.retryable => {
                return Err(failure.error);
            }
            Err(failure) => {
                if failure.connection_was_ready || recovery_started.is_none() {
                    attempts = 0;
                    recovery_started = Some(Instant::now());
                }
                let started = recovery_started.expect("retryable failure starts recovery");
                let Some(delay) = reconnect_delay_within_window(started.elapsed(), attempts) else {
                    return Err(anyhow!(
                        "Remote App Server did not recover within {} seconds after {attempts} attempts: {}",
                        RECONNECT_WINDOW.as_secs(),
                        failure.error
                    ));
                };
                attempts += 1;
                send_event(
                    event_sink,
                    AgentSessionEvent::Error(format!(
                        "Remote App Server disconnected; reconnecting (attempt {attempts}, {} second recovery window)",
                        RECONNECT_WINDOW.as_secs()
                    )),
                )?;
                if !wait_for_reconnect(event_sink, commands, delay)? {
                    return Ok(());
                }
            }
        }
    }
}

fn wait_for_reconnect(
    event_sink: &AgentSessionEventSink,
    commands: &Receiver<AgentSessionCommand>,
    delay: Duration,
) -> Result<bool> {
    let started = Instant::now();
    loop {
        let Some(remaining) = delay.checked_sub(started.elapsed()) else {
            return Ok(true);
        };
        match commands.recv_timeout(remaining) {
            Ok(command) => {
                if reject_disconnected_command(command) {
                    return Ok(false);
                }
                send_event(
                    event_sink,
                    AgentSessionEvent::Error(AGENT_UNAVAILABLE_COMMAND_ERROR.to_owned()),
                )?;
            }
            Err(mpsc::RecvTimeoutError::Timeout) => return Ok(true),
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(false),
        }
    }
}

fn run_connection(
    event_sink: &AgentSessionEventSink,
    commands: &Receiver<AgentSessionCommand>,
    target: &dyn AgentSessionTarget,
    preferred_session_id: Option<&SessionId>,
    available: &AtomicBool,
) -> std::result::Result<(), AgentSessionFailure> {
    available.store(false, Ordering::Release);
    let workspace_root = target.workspace_root();
    let mut session = target
        .start()
        .map_err(anyhow::Error::msg)
        .map_err(AgentSessionFailure::connection)?;
    let mut client = session.client();
    let initialization = client
        .initialization()
        .map_err(|error| AgentSessionFailure::connection(anyhow!(error.to_string())))?;
    let slash_commands = initialization.slash_commands.clone();
    let models = client
        .list_models()
        .map_err(|error| AgentSessionFailure::connection(anyhow!(error.to_string())))?
        .models;
    send_event(
        event_sink,
        AgentSessionEvent::Catalog {
            slash_commands,
            models,
        },
    )
    .map_err(AgentSessionFailure::fatal)?;
    publish_configuration(event_sink, &mut client).map_err(AgentSessionFailure::connection)?;
    publish_git_snapshot(event_sink, &mut client).map_err(AgentSessionFailure::connection)?;
    let events = session
        .take_events()
        .map_err(|error| AgentSessionFailure::connection(anyhow!(error.to_string())))?;
    let (sessions, mut active) =
        ensure_active_session(&mut client, workspace_root, preferred_session_id)
            .map_err(AgentSessionFailure::connection)?;
    send_event(event_sink, AgentSessionEvent::SessionCatalog(sessions))
        .map_err(AgentSessionFailure::fatal)?;
    publish_subscription(event_sink, &active.subscription, &active.thread_id, None)
        .map_err(AgentSessionFailure::fatal)?;
    available.store(true, Ordering::Release);

    let loop_result = drive(
        event_sink,
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
            if error.downcast_ref::<ConnectionLost>().is_some() {
                Err(AgentSessionFailure::disconnected(error))
            } else {
                Err(AgentSessionFailure::fatal(error))
            }
        }
    }
}

fn drive(
    event_sink: &AgentSessionEventSink,
    commands: &Receiver<AgentSessionCommand>,
    events: &AppServerEvents,
    client: &mut AppServerRequestHandle,
    active: &mut ActiveSession,
    workspace_root: &Path,
    target: &dyn AgentSessionTarget,
) -> Result<()> {
    loop {
        loop {
            match commands.try_recv() {
                Ok(AgentSessionCommand::CreateSession) => {
                    match create_active_session(client, workspace_root) {
                        Ok(next) => {
                            unsubscribe_active(event_sink, client, active)?;
                            *active = next;
                            publish_subscription(
                                event_sink,
                                &active.subscription,
                                &active.thread_id,
                                None,
                            )?;
                        }
                        Err(error) => {
                            send_event(event_sink, AgentSessionEvent::Error(error.to_string()))?;
                        }
                    }
                }
                Ok(AgentSessionCommand::StopSession {
                    session_id,
                    response,
                }) => {
                    let result =
                        stop_session(client, &session_id).map_err(|error| error.to_string());
                    let _ = response.send(result);
                }
                Ok(AgentSessionCommand::ActivateSession {
                    session_id,
                    switch_id,
                    response,
                }) => match resolve_session_activation(client, session_id, workspace_root) {
                    Ok(SessionActivation::Current(next)) => {
                        unsubscribe_active(event_sink, client, active)?;
                        *active = next;
                        publish_subscription(
                            event_sink,
                            &active.subscription,
                            &active.thread_id,
                            Some(switch_id),
                        )?;
                        let _ = response.send(Ok(None));
                    }
                    Ok(SessionActivation::Reconnect {
                        root,
                        preferred_session_id,
                    }) => match prepare_workspace_reconnect(target, root.clone()) {
                        Ok(prepared) => {
                            let _ = response.send(Ok(Some(prepared)));
                            return Err(anyhow!(ReconnectWorkspace {
                                root,
                                preferred_session_id: Some(preferred_session_id),
                            }));
                        }
                        Err(error) => {
                            let message = error.to_string();
                            let _ = response.send(Err(message.clone()));
                            send_event(event_sink, AgentSessionEvent::Error(message))?;
                        }
                    },
                    Err(error) => {
                        let message = error.to_string();
                        let _ = response.send(Err(message.clone()));
                        send_event(event_sink, AgentSessionEvent::Error(message))?;
                    }
                },
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
                        event_sink,
                        &active.subscription,
                        &active.thread_id,
                        None,
                    )?;
                }
                Ok(AgentSessionCommand::RefreshGit) => {
                    if let Err(error) = publish_git_snapshot(event_sink, client) {
                        send_event(event_sink, AgentSessionEvent::Error(error.to_string()))?;
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
                    let _ =
                        response.send(read_file(client, path).map_err(|error| error.to_string()));
                }
                Ok(AgentSessionCommand::WriteFile { request, response }) => {
                    let _ = response
                        .send(write_file(client, request).map_err(|error| error.to_string()));
                }
                Ok(AgentSessionCommand::ListGitBranches(response)) => {
                    let result = client
                        .list_git_branches()
                        .map(|result| result.branches)
                        .map_err(|error| error.to_string());
                    let _ = response.send(result);
                }
                Ok(AgentSessionCommand::SwitchGitBranch { name, response }) => {
                    let _ = response
                        .send(switch_git_branch(client, name).map_err(|error| error.to_string()));
                }
                Ok(AgentSessionCommand::SwitchWorkspace { root, response }) => {
                    match prepare_workspace_reconnect(target, root.clone()) {
                        Ok(prepared) => {
                            let root = prepared.root.clone();
                            let _ = response.send(Ok(prepared));
                            return Err(anyhow!(ReconnectWorkspace {
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
                    send_event(event_sink, AgentSessionEvent::Update(update))?;
                }
            }
            Ok(AppServerEvent::Notification(ServerNotification::GitStatusChanged(_))) => {
                publish_git_snapshot(event_sink, client)?;
            }
            Ok(AppServerEvent::Notification(ServerNotification::FsChanged(changed))) => {
                send_event(event_sink, AgentSessionEvent::FilesChanged(changed))?;
            }
            Ok(AppServerEvent::Notification(ServerNotification::ConfigChanged(_))) => {
                publish_configuration(event_sink, client)?;
            }
            Ok(AppServerEvent::Notification(_)) => {}
            Ok(AppServerEvent::ConnectionClosed(reason)) => {
                return Err(
                    ConnectionLost(format!("App Server connection closed: {reason:?}")).into(),
                );
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(ConnectionLost("App Server event stream disconnected".into()).into());
            }
        }
    }
}
