use std::sync::atomic::AtomicBool;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::RecvTimeoutError;
use std::time::Duration;
use std::time::Instant;

use zui::app::AppProxy;

use super::AgentSessionCommand;
use super::AgentSessionEvent;
use super::NativeEvent;
use crate::app_server::AppServerHost;
use zeta_agent_session::{
    AGENT_UNAVAILABLE_COMMAND_ERROR, RECONNECT_WINDOW, reconnect_delay_within_window,
    reject_disconnected_command,
};

/// Runs one Remote Agent connection at a time and rebuilds its projection after a transport loss.
///
/// The remote App Server owns durable Session and Thread state, so a reconnect starts a new
/// protocol session and asks the normal startup path for the current snapshot. Requests that were
/// in flight when SSH failed and commands submitted while disconnected are never replayed. A
/// connection that reached the normal event loop starts a fresh recovery window on a later loss.
/// Terminal runtimes use their separate PTY reconnect lease.
pub(super) fn run_with_recovery(
    event_proxy: &AppProxy<NativeEvent>,
    commands: &Receiver<AgentSessionCommand>,
    target: &AppServerHost,
    available: &AtomicBool,
) -> anyhow::Result<()> {
    let mut target = target.clone();
    let mut preferred_session_id = None;
    let mut attempts = 0;
    let mut recovery_started = None;
    loop {
        match super::run_agent_session_connection(
            event_proxy,
            commands,
            &target,
            preferred_session_id.as_ref(),
            available,
        ) {
            Ok(()) => return Ok(()),
            Err(failure)
                if failure
                    .error
                    .downcast_ref::<super::AgentSessionReconnect>()
                    .is_some() =>
            {
                let reconnect = failure
                    .error
                    .downcast_ref::<super::AgentSessionReconnect>()
                    .expect("guard verified reconnect marker");
                target = target.with_workspace_root(&reconnect.root)?;
                preferred_session_id = reconnect.preferred_session_id.clone();
                attempts = 0;
                recovery_started = None;
            }
            Err(failure) if !failure.retryable => {
                return Err(anyhow::anyhow!(
                    "Remote Agent session stopped: {}",
                    failure.error
                ));
            }
            Err(failure) => {
                if failure.connection_was_ready || recovery_started.is_none() {
                    attempts = 0;
                    recovery_started = Some(Instant::now());
                }
                let started = recovery_started.expect("retryable failure starts recovery");
                let Some(delay) = reconnect_delay_within_window(started.elapsed(), attempts) else {
                    return Err(anyhow::anyhow!(
                        "Remote App Server did not recover within {} seconds after {attempts} attempts: {}",
                        RECONNECT_WINDOW.as_secs(),
                        failure.error
                    ));
                };
                attempts += 1;
                super::send_event(
                    event_proxy,
                    AgentSessionEvent::Error(format!(
                        "Remote App Server disconnected; reconnecting (attempt {attempts}, {} second recovery window)",
                        RECONNECT_WINDOW.as_secs()
                    )),
                )?;
                if !wait_for_reconnect(event_proxy, commands, delay)? {
                    return Ok(());
                }
            }
        }
    }
}

fn wait_for_reconnect(
    event_proxy: &AppProxy<NativeEvent>,
    commands: &Receiver<AgentSessionCommand>,
    delay: Duration,
) -> anyhow::Result<bool> {
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
                super::send_event(
                    event_proxy,
                    AgentSessionEvent::Error(AGENT_UNAVAILABLE_COMMAND_ERROR.to_owned()),
                )?;
            }
            Err(RecvTimeoutError::Timeout) => return Ok(true),
            Err(RecvTimeoutError::Disconnected) => return Ok(false),
        }
    }
}
