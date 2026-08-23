use std::path::PathBuf;
use std::thread;
use std::time::Duration;
use std::time::Instant;

use zeta_remote::RemoteProfile;
use zeta_remote::RemoteWorkspacePath;
use zeta_remote::SshTarget;
use zeta_remote_connections::RemoteConnectionFailureKind;

use super::runtime;
use super::runtime::ReadyRemoteRuntime;

const RECONNECT_WINDOW: Duration = Duration::from_secs(30);
const INITIAL_RECONNECT_DELAY: Duration = Duration::from_millis(250);
const MAX_RECONNECT_DELAY: Duration = Duration::from_secs(2);

pub(super) fn run(
    ready: ReadyRemoteRuntime,
    ssh_executable: Option<PathBuf>,
) -> Result<(), String> {
    let mut profile = ready.profile;
    let mut session = ready.session;
    let mut recovery = None;
    loop {
        let mut options =
            zeta_tui::TuiOptions::new(format!("Remote SSH: {}", profile.target().host().as_str()))
                .with_remote_workspace(PathBuf::from(profile.target().workspace().as_str()))
                .with_profile_root(zeta_app_server_client::local_profile_root());
        if let Some(state) = recovery.take() {
            options = options.with_recovery(state);
        }
        match zeta_tui::run(session, options).map_err(|error| error.to_string())? {
            zeta_tui::TuiExit::UserRequested | zeta_tui::TuiExit::TerminationRequested => {
                return Ok(());
            }
            zeta_tui::TuiExit::ConnectionLost {
                kind: zeta_tui::TuiConnectionLossKind::Transport,
                recovery: next_recovery,
                reason,
            } => {
                eprintln!("Remote App Server disconnected: {reason}");
                recovery = Some(next_recovery);
                session = reconnect(&profile, ssh_executable.as_deref(), &reason)?.session;
            }
            zeta_tui::TuiExit::ConnectionLost { kind, reason, .. } => {
                return Err(format!(
                    "Remote App Server recovery stopped after {kind:?}: {reason}"
                ));
            }
            zeta_tui::TuiExit::WorkspaceReconnectRequested(request) => {
                let (workspace_root, next_recovery) = request.into_parts();
                let workspace = workspace_root.to_str().ok_or_else(|| {
                    "Remote Session Workspace path is not valid UTF-8".to_string()
                })?;
                let target = SshTarget::new(
                    profile.target().host().clone(),
                    RemoteWorkspacePath::parse(workspace).map_err(|error| error.to_string())?,
                );
                profile = RemoteProfile::new(target, profile.runtime().clone());
                recovery = Some(next_recovery);
                session = reconnect(
                    &profile,
                    ssh_executable.as_deref(),
                    "Session belongs to another Remote Workspace",
                )?
                .session;
            }
        }
    }
}

fn reconnect(
    profile: &RemoteProfile,
    ssh_executable: Option<&std::path::Path>,
    initial_reason: &str,
) -> Result<ReadyRemoteRuntime, String> {
    let started = Instant::now();
    retry(
        initial_reason,
        || match runtime::reconnect_exact(profile, ssh_executable) {
            Ok(ready) => Ok(ready),
            Err(error) if error.kind() == RemoteConnectionFailureKind::Transport => {
                Err(ReconnectFailure::Retryable(error.to_string()))
            }
            Err(error) => Err(ReconnectFailure::Terminal(format!(
                "Remote App Server reconnect stopped because the verified runtime changed or rejected the connection: {error}"
            ))),
        },
        thread::sleep,
        || started.elapsed(),
        |attempt, delay| {
            eprintln!(
                "Reconnecting to Remote App Server (attempt {attempt}, retrying in {} ms)...",
                delay.as_millis()
            )
        },
    )
}

enum ReconnectFailure {
    Retryable(String),
    Terminal(String),
}

fn retry<T>(
    initial_reason: &str,
    mut reconnect: impl FnMut() -> Result<T, ReconnectFailure>,
    mut wait: impl FnMut(Duration),
    mut elapsed: impl FnMut() -> Duration,
    mut report: impl FnMut(usize, Duration),
) -> Result<T, String> {
    let mut attempts = 0;
    let mut last_reason = initial_reason.to_owned();
    loop {
        let Some(delay) = reconnect_delay_within_window(elapsed(), attempts) else {
            return Err(format!(
                "Remote App Server did not recover within {} seconds after {attempts} attempts: {last_reason}",
                RECONNECT_WINDOW.as_secs()
            ));
        };
        attempts += 1;
        report(attempts, delay);
        wait(delay);
        match reconnect() {
            Ok(ready) => return Ok(ready),
            Err(ReconnectFailure::Retryable(error)) => last_reason = error,
            Err(ReconnectFailure::Terminal(error)) => return Err(error),
        }
    }
}

fn reconnect_delay_within_window(elapsed: Duration, attempt: usize) -> Option<Duration> {
    let remaining = RECONNECT_WINDOW.checked_sub(elapsed)?;
    let delay = reconnect_delay(attempt);
    (delay <= remaining).then_some(delay)
}

fn reconnect_delay(attempt: usize) -> Duration {
    let multiplier = 1_u32 << (attempt.min(31) as u32);
    (INITIAL_RECONNECT_DELAY * multiplier).min(MAX_RECONNECT_DELAY)
}

#[cfg(test)]
#[path = "remote_connect_tui_tests.rs"]
mod tests;
