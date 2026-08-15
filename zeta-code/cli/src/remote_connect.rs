use std::io::IsTerminal;
use std::path::Path;
use std::path::PathBuf;

use serde::Serialize;
use zeta_app_server_client::local_profile_root;
use zeta_remote::RemoteWorkspacePath;
use zeta_remote::SshHost;
use zeta_remote::SshTarget;
use zeta_remote_connections::RemoteConnectionCatalog;
use zeta_remote_connections::RemoteConnectionName;
use zeta_remote_connections::RemoteConnectionProfileRecord;
use zeta_remote_connections::RemoteConnectionProfileStore;

#[path = "remote_connect_runtime.rs"]
mod runtime;
#[path = "remote_connect_tui.rs"]
mod tui;

use runtime::RemoteConnectRuntimeInput;
use runtime::RemoteConnectRuntimeSelection;

const CONNECT_USAGE: &str = concat!(
    "usage: zeta remote connect (--name <saved-name> | --host <ssh-host> ",
    "--workspace <absolute-remote-path>) [--runtime <remote-runtime>] ",
    "[--ssh <openssh-path>] ",
    "[--runtime-catalog <local-catalog> --runtime-catalog-sha256 <digest> | ",
    "--runtime-catalog-url <https-catalog.json> --runtime-catalog-sha256 <digest> ",
    "[--runtime-cache <absolute-local-path>]] [--check]"
);

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct RemoteConnectOptions {
    pub(super) target: RemoteConnectTarget,
    runtime: RemoteConnectRuntimeSelection,
    pub(super) ssh_executable: Option<PathBuf>,
    pub(super) mode: RemoteConnectMode,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum RemoteConnectTarget {
    Named(RemoteConnectionName),
    Direct(SshTarget),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RemoteConnectMode {
    Interactive,
    Check,
}

pub(super) fn parse(arguments: &[String]) -> Result<RemoteConnectOptions, String> {
    let mut name = None;
    let mut host = None;
    let mut workspace = None;
    let mut runtime = None;
    let mut ssh_executable = None;
    let mut runtime_catalog = None;
    let mut runtime_catalog_url = None;
    let mut runtime_catalog_sha256 = None;
    let mut runtime_cache = None;
    let mut mode = None;
    let mut index = 0;
    while index < arguments.len() {
        let option = arguments[index].as_str();
        index += 1;
        if option == "--check" {
            assign_once(&mut mode, RemoteConnectMode::Check, option)?;
            continue;
        }
        let value = arguments
            .get(index)
            .ok_or_else(|| format!("{option} requires a value\n\n{CONNECT_USAGE}"))?;
        index += 1;
        match option {
            "--name" => assign_once(
                &mut name,
                RemoteConnectionName::parse(value).map_err(string_error)?,
                option,
            )?,
            "--host" => assign_once(
                &mut host,
                SshHost::parse(value).map_err(string_error)?,
                option,
            )?,
            "--workspace" => assign_once(
                &mut workspace,
                RemoteWorkspacePath::parse(value).map_err(string_error)?,
                option,
            )?,
            "--runtime" => assign_once(
                &mut runtime,
                zeta_remote::RemoteRuntime::new(value).map_err(string_error)?,
                option,
            )?,
            "--ssh" => assign_once(&mut ssh_executable, PathBuf::from(value), option)?,
            "--runtime-catalog" => assign_once(&mut runtime_catalog, PathBuf::from(value), option)?,
            "--runtime-catalog-url" => {
                assign_once(&mut runtime_catalog_url, value.to_owned(), option)?
            }
            "--runtime-catalog-sha256" => {
                assign_once(&mut runtime_catalog_sha256, value.to_owned(), option)?
            }
            "--runtime-cache" => assign_once(&mut runtime_cache, PathBuf::from(value), option)?,
            _ => {
                return Err(format!(
                    "unknown remote connect option: {option}\n\n{CONNECT_USAGE}"
                ));
            }
        }
    }
    let target = match (name, host, workspace) {
        (Some(name), None, None) => RemoteConnectTarget::Named(name),
        (None, Some(host), Some(workspace)) => {
            RemoteConnectTarget::Direct(SshTarget::new(host, workspace))
        }
        (Some(_), _, _) => {
            return Err(format!(
                "--name cannot be combined with --host or --workspace\n\n{CONNECT_USAGE}"
            ));
        }
        (None, _, _) => {
            return Err(format!(
                "select --name or both --host and --workspace\n\n{CONNECT_USAGE}"
            ));
        }
    };
    Ok(RemoteConnectOptions {
        target,
        runtime: RemoteConnectRuntimeSelection::parse(RemoteConnectRuntimeInput {
            runtime,
            local_catalog: runtime_catalog,
            catalog_url: runtime_catalog_url,
            catalog_sha256: runtime_catalog_sha256,
            runtime_cache,
        })
        .map_err(|error| format!("{error}\n\n{CONNECT_USAGE}"))?,
        ssh_executable,
        mode: mode.unwrap_or(RemoteConnectMode::Interactive),
    })
}

pub(super) fn run(options: RemoteConnectOptions) -> Result<(), String> {
    let RemoteConnectOptions {
        target,
        runtime,
        ssh_executable,
        mode,
    } = options;
    if mode == RemoteConnectMode::Interactive
        && (!std::io::stdin().is_terminal() || !std::io::stdout().is_terminal())
    {
        return Err(
            "remote connect requires a TTY; use --check to validate the connection without opening the TUI"
                .into(),
        );
    }
    let profile_root = local_profile_root();
    let target = resolve_target(&profile_root, target)?;
    let store = RemoteConnectionProfileStore::from_profile_root(&profile_root);
    let ready = runtime::connect(
        target,
        runtime,
        ssh_executable.as_deref(),
        &profile_root,
        &store,
    )?;
    match mode {
        RemoteConnectMode::Check => {
            ready
                .session
                .shutdown()
                .map_err(|error| format!("Remote connection did not shut down cleanly: {error}"))?;
            let record = store.activate(&ready.profile).map_err(string_error)?;
            print_profile(&record)
        }
        RemoteConnectMode::Interactive => {
            store.activate(&ready.profile).map_err(string_error)?;
            tui::run(ready, ssh_executable)
        }
    }
}

fn resolve_target(profile_root: &Path, target: RemoteConnectTarget) -> Result<SshTarget, String> {
    match target {
        RemoteConnectTarget::Direct(target) => Ok(target),
        RemoteConnectTarget::Named(name) => {
            RemoteConnectionCatalog::from_profile_root(profile_root)
                .connection(&name)
                .map_err(string_error)?
                .map(|entry| entry.target().clone())
                .ok_or_else(|| {
                    format!("saved Remote connection '{}' does not exist", name.as_str())
                })
        }
    }
}

fn assign_once<T>(slot: &mut Option<T>, value: T, name: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        return Err(format!("{name} may be specified only once"));
    }
    Ok(())
}

fn print_profile(record: &RemoteConnectionProfileRecord) -> Result<(), String> {
    println!(
        "{}",
        serde_json::to_string(&RemoteConnectOutput::from(record)).map_err(string_error)?
    );
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteConnectOutput {
    host: String,
    workspace: String,
    active_runtime: String,
}

impl From<&RemoteConnectionProfileRecord> for RemoteConnectOutput {
    fn from(record: &RemoteConnectionProfileRecord) -> Self {
        Self {
            host: record.target().host().as_str().into(),
            workspace: record.target().workspace().as_str().into(),
            active_runtime: record.active_runtime().executable().into(),
        }
    }
}

fn string_error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
#[path = "remote_connect_tests.rs"]
mod tests;
