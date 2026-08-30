use std::path::PathBuf;
use std::thread;
use std::time::Instant;

use zeta_remote::RemoteProfile;
use zeta_remote_connections::RemoteConnectionFailureKind;

use super::runtime;
use super::runtime::ReadyRemoteRuntime;
use crate::reconnect;
use crate::reconnect::Failure;

pub(super) fn run(
    ready: ReadyRemoteRuntime,
    ssh_executable: Option<PathBuf>,
) -> Result<(), String> {
    let profile = ready.profile;
    let mut session = ready.session;
    let mut recovery = None;
    loop {
        let mut options =
            zeta_tui::TuiOptions::new(format!("Remote SSH: {}", profile.target().host().as_str()))
                .with_remote_dir(PathBuf::from(profile.target().dir().as_str()))
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
        }
    }
}

fn reconnect(
    profile: &RemoteProfile,
    ssh_executable: Option<&std::path::Path>,
    initial_reason: &str,
) -> Result<ReadyRemoteRuntime, String> {
    let started = Instant::now();
    reconnect::retry(
        "Remote App Server",
        initial_reason,
        || match runtime::reconnect_exact(profile, ssh_executable) {
            Ok(ready) => Ok(ready),
            Err(error) if error.kind() == RemoteConnectionFailureKind::Transport => {
                Err(Failure::Retryable(error.to_string()))
            }
            Err(error) => Err(Failure::Terminal(format!(
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
