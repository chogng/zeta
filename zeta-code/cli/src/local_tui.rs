use std::env;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::thread;
use std::time::Instant;

use zeta_app_server_client::AppServerSession;
use zeta_app_server_client::ClientError;
use zeta_app_server_client::StdioAppServerCommand;
use zeta_app_server_protocol::protocol::common::ClientInfo;

use crate::reconnect;
use crate::reconnect::Failure;

pub(super) fn run(dir_root: PathBuf, profile_root: PathBuf) -> Result<(), String> {
    run_entry(dir_root, profile_root, Entry::New)
}

pub(super) fn resume(
    dir_root: PathBuf,
    profile_root: PathBuf,
    recovery: zeta_tui::TuiRecoveryState,
) -> Result<(), String> {
    run_entry(dir_root, profile_root, Entry::Resume(recovery))
}

enum Entry {
    New,
    Resume(zeta_tui::TuiRecoveryState),
}

fn run_entry(dir_root: PathBuf, profile_root: PathBuf, entry: Entry) -> Result<(), String> {
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err(
            "interactive mode requires a TTY; use `zeta ask` or `zeta exec` instead".into(),
        );
    }
    let executable = env::current_exe()
        .map_err(|error| format!("could not resolve zeta executable: {error}"))?;
    let mut session =
        connect(&executable, &dir_root, &profile_root).map_err(|error| error.to_string())?;
    let mut recovery = match entry {
        Entry::New => None,
        Entry::Resume(recovery) => Some(recovery),
    };
    loop {
        let mut options = zeta_tui::TuiOptions::new("TUI conversation")
            .with_dir_root(&dir_root)
            .with_profile_root(&profile_root);
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
                eprintln!("Local App Server disconnected: {reason}");
                session =
                    reconnect(&executable, &dir_root, &profile_root, &reason).map_err(|error| {
                        reconnect::recovery_error(error, &recovery_command(&next_recovery))
                    })?;
                recovery = Some(next_recovery);
            }
            zeta_tui::TuiExit::ConnectionLost {
                kind,
                recovery,
                reason,
            } => {
                return Err(reconnect::recovery_error(
                    format!("Local App Server recovery stopped after {kind:?}: {reason}"),
                    &recovery_command(&recovery),
                ));
            }
        }
    }
}

fn recovery_command(recovery: &zeta_tui::TuiRecoveryState) -> Vec<String> {
    vec![
        "zeta".into(),
        "resume".into(),
        recovery.session_id().to_string(),
        recovery.thread_id().to_string(),
    ]
}

fn connect(
    executable: &std::path::Path,
    dir_root: &std::path::Path,
    profile_root: &std::path::Path,
) -> Result<AppServerSession, ClientError> {
    let command = StdioAppServerCommand::new(executable)
        .with_argument("app-server")
        .with_argument("connect")
        .with_environment_variable("ZETA_PROFILE_ROOT", profile_root.as_os_str())
        .with_environment_variable("ZETA_WORKSPACE_ROOT", dir_root.as_os_str());
    AppServerSession::start_stdio(
        command,
        ClientInfo {
            name: "zeta-cli".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        zeta_tui::client_capabilities(),
    )
}

fn reconnect(
    executable: &std::path::Path,
    dir_root: &std::path::Path,
    profile_root: &std::path::Path,
    initial_reason: &str,
) -> Result<AppServerSession, String> {
    let started = Instant::now();
    reconnect::retry(
        "Local App Server",
        initial_reason,
        || connect(executable, dir_root, profile_root).map_err(classify_error),
        thread::sleep,
        || started.elapsed(),
        |attempt, delay| {
            eprintln!(
                "Reconnecting to Local App Server (attempt {attempt}, retrying in {} ms)...",
                delay.as_millis()
            );
        },
    )
}

fn classify_error(error: ClientError) -> Failure {
    match error {
        ClientError::Transport(_) => Failure::Retryable(error.to_string()),
        ClientError::Protocol(_) | ClientError::Server { .. } => Failure::Terminal(format!(
            "Local App Server reconnect stopped because the server rejected the connection: {error}"
        )),
    }
}

#[cfg(test)]
#[path = "local_tui_tests.rs"]
mod tests;
